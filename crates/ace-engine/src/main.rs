#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
#[cfg(debug_assertions)]
use std::time::Duration;
use sysuri::{UriScheme, register};

#[tokio::main]
async fn main() {
    let _args: Vec<String> = env::args().collect();

    // Lock check for the installer's uninstall gate: read-only query of the
    // engine database, exit codes 0/1/2 (see src/lock_check.rs). Must run
    // before anything else — it never boots the engine, never starts the
    // service, and never touches the single-instance guard.
    if _args.len() >= 2 && _args[1] == "--check-locks" {
        std::process::exit(ace_engine::lock_check::check_locks_main());
    }

    // Installer maintenance uses the same typed RPC shutdown operation.
    // The service itself still refuses shutdown with any active lock and
    // requires an authenticated watchdog maintenance lease.
    if _args.len() >= 2 && _args[1] == "--request-shutdown" {
        let token =
            std::fs::read_to_string(ace_engine::settings::Settings::get_capability_token_path())
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
        if token.is_none() {
            std::process::exit(24);
        }
        let result =
            ace_engine::service::api::bridge::call("shutdown", serde_json::json!({}), token).await;
        std::process::exit(match result {
            Ok(_) => 0,
            Err(error) if error.status == 423 => 23,
            Err(_) => 24,
        });
    }

    // Guardian mode: a lightweight copy spawned by the engine that watches the
    // engine's PID and restarts it if it dies. Must be handled before anything
    // else so it never boots the full engine. Release only.
    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    if _args.len() >= 3 && _args[1] == ace_engine::platform::GUARDIAN_ARG {
        if let Ok(pid) = _args[2].parse::<u32>() {
            ace_engine::platform::run_guardian(pid); // diverges
        }
        return;
    }

    // Handle platform service installation commands. Release builds always
    // support them; debug builds run as plain console processes.
    #[cfg(not(debug_assertions))]
    if _args.len() > 1 {
        match _args[1].as_str() {
            "--install" => {
                match ace_engine::platform::install_service() {
                    Ok(()) => log_to_file("Service installed successfully."),
                    Err(error) => log_to_file(&format!("Install failed: {error}")),
                }
                return;
            }
            "--uninstall" => {
                match ace_engine::platform::uninstall_service() {
                    Ok(()) => log_to_file("Service uninstalled successfully."),
                    Err(error) => log_to_file(&format!("Uninstall failed: {error}")),
                }
                return;
            }
            _ => {}
        }
    }

    // Windows needs an SCM dispatcher entry point. Linux systemd launches the
    // normal async entry point and is detected later through INVOCATION_ID.
    #[cfg(not(debug_assertions))]
    match ace_engine::platform::try_run_as_service() {
        Ok(true) => return, // service ran to completion
        Ok(false) => { /* console mode */ }
        Err(error) => {
            log_to_file(&format!("Service error: {error}"));
            std::process::exit(1);
        }
    }

    if let Err(error) = run().await {
        eprintln!("Fatal error: {:?}", error);
        std::process::exit(1);
    }
}

async fn run() -> eyre::Result<()> {
    color_eyre::install()?;

    // Check for URI Scheme handling (Client Mode)
    // In release mode, this is handled by the ace-redirect binary
    #[cfg(debug_assertions)]
    {
        let args: Vec<String> = env::args().collect();
        if args.len() > 1 && args[1].starts_with("aceblocker://") {
            handle_uri_scheme(&args[1]).await;
            return Ok(());
        }
    }

    // Single Instance Guard (Full Mode Only)
    // Only apply when running in full server mode, not for URI scheme handling
    #[cfg(not(debug_assertions))]
    let _instance_guard = {
        use single_instance::SingleInstance;
        let instance = SingleInstance::new("ace-engine-full-mode").unwrap();
        if !instance.is_single() {
            eprintln!("Ace Engine is already running in full mode.");
            std::process::exit(1);
        }
        instance
    };

    // Register URL scheme on startup (Server Mode)
    if let Err(error) = setup_url_scheme() {
        eprintln!("Failed to register URL scheme: {}", error);
    }

    ace_engine::run().await?;

    // Clean-stop report for the SCM: the service no longer accepts STOP, so
    // without this the SCM would treat the API-shutdown exit as an abnormal
    // termination (1067) and apply the failure actions (restart). No-op when
    // not running as a service.
    ace_engine::platform::report_service_stopped();

    Ok(())
}

fn setup_url_scheme() -> eyre::Result<()> {
    #[allow(unused_mut)]
    let mut exe = env::current_exe()?;

    #[cfg(not(debug_assertions))]
    {
        exe.pop();
        #[cfg(target_os = "windows")]
        exe.push("ace-redirect.exe");
        #[cfg(not(target_os = "windows"))]
        exe.push("ace-redirect");
    }

    let scheme = UriScheme::new("aceblocker", "AceBlocker Protocol", exe);

    register(&scheme)?;

    Ok(())
}

/// Debug-only protocol handler (release uses `ace-redirect.exe`).
#[cfg(debug_assertions)]
async fn handle_uri_scheme(uri: &str) {
    // SECURITY: same allowlist as ace-redirect (see src/protocol.rs). Debug
    // builds do not enforce the capability token, so the allowlist is even
    // more important here: only `/account/api-key` may be forwarded.
    let forward = match ace_engine::protocol::resolve_forward_target(uri) {
        Ok(forward) => forward,
        Err(error) => {
            eprintln!("Rejected aceblocker:// forward: {error}");
            eprintln!(
                "Only these routes may be forwarded: {}",
                ace_engine::protocol::ALLOWED_FORWARD_PATHS.join(", ")
            );
            std::thread::sleep(Duration::from_secs(5));
            std::process::exit(1);
        }
    };

    if forward.path != "/account/api-key" {
        eprintln!("Unsupported deep-link operation");
        std::process::exit(1);
    }
    let fragment = uri.split_once('#').map(|(_, fragment)| fragment);
    let api_key = ace_engine::protocol::api_key_from_fragment(fragment).or_else(|| {
        forward.query.as_deref().and_then(|raw| {
            url::form_urlencoded::parse(raw.as_bytes())
                .find(|(name, _)| name == "apiKey")
                .map(|(_, key)| key.into_owned())
        })
    });
    let Some(api_key) = api_key else {
        eprintln!("Missing account API key in deep link");
        std::process::exit(1);
    };
    match ace_engine::service::api::bridge::call(
        "setApiKey",
        serde_json::json!({ "payload": { "apiKey": api_key } }),
        Some(ace_engine::settings::Settings::load_or_create_capability_token()),
    )
    .await
    {
        Ok(_) => println!("Account key accepted"),
        Err(error) => {
            eprintln!("Engine rejected callback: {}", error.error);
            std::process::exit(1);
        }
    }
}

/// Write a line to the install log so GUI-subsystem binaries have visible output.
#[cfg_attr(debug_assertions, allow(dead_code))]
fn log_to_file(message: &str) {
    let path = match app_dirs2::app_dir(
        app_dirs2::AppDataType::SharedData,
        &ace_engine::settings::APP_INFO,
        "",
    ) {
        Ok(dir) => {
            let _ = std::fs::create_dir_all(&dir);
            dir.join("install.log")
        }
        Err(_) => {
            // Last resort: write next to the executable.
            let mut p = std::env::current_exe().unwrap_or_default();
            p.set_file_name("install.log");
            p
        }
    };
    let line = format!(
        "[{}] {message}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
}
