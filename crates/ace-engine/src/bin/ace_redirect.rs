use std::env;
#[cfg(target_os = "windows")]
use std::time::Duration;

use app_dirs2::*;

pub const APP_INFO: AppInfo = AppInfo {
    name: "AceBlocker",
    author: "LineaSoftware",
};

pub fn get_api_port() -> u16 {
    let path =
        get_app_dir(AppDataType::SharedData, &APP_INFO, "port.lock").expect("Couldn't get appdir");

    if let Ok(port_str) = std::fs::read_to_string(&path)
        && let Ok(port) = port_str.trim().parse::<u16>()
    {
        return port;
    }

    10567
}

/// Reads the per-install capability token so protocol-handler forwards can
/// authenticate state-changing requests (see
/// `docs/local-api-protection-design.md`). Returns `None` when the file is
/// missing — the engine accepts token-less requests in debug builds.
pub fn get_capability_token() -> Option<String> {
    let path = get_app_dir(AppDataType::SharedData, &APP_INFO, "capability.token")
        .expect("Couldn't get appdir");
    let token = std::fs::read_to_string(&path).ok()?;
    let trimmed = token.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1].starts_with("aceblocker://") {
        handle_uri_scheme(&args[1]).await;
    } else {
        eprintln!("Usage: ace-redirect aceblocker://...");
        std::process::exit(1);
    }
}

async fn handle_uri_scheme(uri: &str) {
    let port = get_api_port();

    if uri == "aceblocker://get-engine-port" {
        std::process::exit(port as i32);
    }

    // SECURITY: the capability token below authenticates the forwarded POST
    // as the desktop app. Any website can render an `aceblocker://` link, so
    // the forwarded path is restricted to the explicit allowlist in
    // `src/protocol.rs` (only `/account/api-key` today). A malicious
    // `aceblocker://system/shutdown` or `aceblocker://backup/import` link
    // must never reach the engine with the token attached.
    let forward = match ace_engine::protocol::resolve_forward_target(uri) {
        Ok(forward) => forward,
        Err(error) => {
            eprintln!("Rejected aceblocker:// forward: {error}");
            eprintln!(
                "Only these routes may be forwarded: {}",
                ace_engine::protocol::ALLOWED_FORWARD_PATHS.join(", ")
            );

            // On Windows, keep the window open for a bit if launched via protocol
            #[cfg(target_os = "windows")]
            std::thread::sleep(Duration::from_secs(5));

            std::process::exit(2);
        }
    };

    // Convert aceblocker://path to https://127.0.0.1:{port}/path. The legacy
    // ?apiKey= query string (older clients) is forwarded when present; the
    // fragment is not — the key travels as a JSON body instead so it never
    // appears in the forwarded request line or in logs.
    let mut proxy_url = format!("https://127.0.0.1:{}{}", port, forward.path);
    if let Some(query) = &forward.query {
        proxy_url.push('?');
        proxy_url.push_str(query);
    }

    println!("Redirecting {} to {}", forward.path, proxy_url);

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true) // The engine uses a self-signed cert
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();

    let fragment = uri.split_once('#').map(|(_, fragment)| fragment);
    let mut request = client.post(&proxy_url);
    if let Some(api_key) = ace_engine::protocol::api_key_from_fragment(fragment) {
        request = request.json(&serde_json::json!({ "apiKey": api_key }));
    }
    if let Some(token) = get_capability_token() {
        request = request.header("X-Ace-Capability", token);
    }

    match request.send().await {
        Ok(res) => {
            println!("Request successful: Status {}", res.status());
        }
        Err(error) => {
            eprintln!("Failed to forward request to Ace Engine service: {}", error);
            eprintln!("Ensure the Ace Engine service is running on port {}.", port);

            // On Windows, keep the window open for a bit if launched via protocol
            #[cfg(target_os = "windows")]
            std::thread::sleep(Duration::from_secs(5));

            std::process::exit(1);
        }
    }
}
