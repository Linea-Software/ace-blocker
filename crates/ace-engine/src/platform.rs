//! # Platform abstraction layer
//!
//! OS-specific service management and process protection.

#[cfg(not(target_os = "windows"))]
use eyre::Result;
#[cfg(not(target_os = "windows"))]
use std::path::Path;

#[cfg(target_os = "windows")]
mod imp {
    use eyre::{Context, Result};
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;
    use tracing::{info, warn};

    use windows::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_NORMAL,
        FILE_ATTRIBUTE_SYSTEM, SetFileAttributesW,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
    };
    use windows::core::PCWSTR;

    use std::os::windows::process::CommandExt;
    use std::process::Command;

    use windows_service::{
        define_windows_service,
        service::{
            ServiceAccess, ServiceAction, ServiceActionType, ServiceControl, ServiceControlAccept,
            ServiceErrorControl, ServiceExitCode, ServiceFailureActions, ServiceFailureResetPeriod,
            ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
        service_dispatcher,
        service_manager::{ServiceManager, ServiceManagerAccess},
    };

    use crate::settings::Settings;

    // -------------------------------------------------------------------
    // Constants
    // -------------------------------------------------------------------

    const SERVICE_NAME: &str = "AceEngine";
    const SERVICE_DISPLAY_NAME: &str = "Ace Blocker Engine";

    /// The registered service status handle. Needed to report
    /// `SERVICE_STOPPED` when the engine shuts itself down through its own API
    /// (`POST /system/shutdown`): the service no longer accepts STOP from the
    /// SCM, so an exit while still RUNNING would be recorded as an abnormal
    /// termination (1067) and trigger the SCM failure actions (restart).
    static SERVICE_STATUS_HANDLE: Mutex<Option<ServiceStatusHandle>> = Mutex::new(None);

    // -------------------------------------------------------------------
    // Service installation / uninstallation
    // -------------------------------------------------------------------

    /// Install ace-engine as a Windows service (SYSTEM account, auto-start,
    /// automatic restart on crash).
    pub fn install_service() -> Result<()> {
        let exe_path =
            std::env::current_exe().wrap_err("cannot determine current executable path")?;

        let manager =
            ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)
                .wrap_err("failed to connect to service control manager")?;

        let service_info = ServiceInfo {
            name: OsString::from(SERVICE_NAME),
            display_name: OsString::from(SERVICE_DISPLAY_NAME),
            service_type: ServiceType::OWN_PROCESS,
            start_type: ServiceStartType::AutoStart,
            error_control: ServiceErrorControl::Normal,
            executable_path: exe_path,
            launch_arguments: vec![],
            dependencies: vec![],
            account_name: None, // LOCAL SYSTEM
            account_password: None,
        };

        let service = manager
            .create_service(&service_info, ServiceAccess::ALL_ACCESS)
            .wrap_err("failed to create service")?;

        // Match the MSI: SCM repeats the third 2-second restart action for
        // subsequent failures. The non-crash-failures flag also covers
        // clean-but-unexpected exits, not intentional SERVICE_STOPPED exits.
        let failure_actions = ServiceFailureActions {
            reset_period: ServiceFailureResetPeriod::After(Duration::from_secs(24 * 60 * 60)),
            reboot_msg: None,
            command: None,
            actions: Some(vec![
                ServiceAction {
                    action_type: ServiceActionType::Restart,
                    delay: Duration::from_secs(2),
                },
                ServiceAction {
                    action_type: ServiceActionType::Restart,
                    delay: Duration::from_secs(2),
                },
                ServiceAction {
                    action_type: ServiceActionType::Restart,
                    delay: Duration::from_secs(2),
                },
            ]),
        };

        service
            .update_failure_actions(failure_actions)
            .wrap_err("failed to configure failure actions")?;

        service
            .set_failure_actions_on_non_crash_failures(true)
            .wrap_err("failed to enable non-crash failure actions")?;

        info!(
            "Windows service '{}' installed (auto-start, recovery enabled).",
            SERVICE_NAME
        );

        Ok(())
    }

    /// Remove the Windows service. Stops it first if running.
    pub fn uninstall_service() -> Result<()> {
        let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
            .wrap_err("failed to connect to service control manager")?;

        let service = match manager.open_service(
            SERVICE_NAME,
            ServiceAccess::DELETE | ServiceAccess::STOP | ServiceAccess::QUERY_STATUS,
        ) {
            Ok(s) => s,
            Err(windows_service::Error::Winapi(error)) if error.raw_os_error() == Some(1060) => {
                // ERROR_SERVICE_DOES_NOT_EXIST — nothing to remove.
                info!(
                    "Windows service '{}' not found; nothing to uninstall.",
                    SERVICE_NAME
                );
                return Ok(());
            }
            Err(error) => return Err(error).wrap_err("failed to open service"),
        };

        // Stop the service if it is running. The engine deliberately refuses
        // SCM STOP (see `service_event_handler`), so this normally does not
        // stop anything — the user must shut the engine down via its own API
        // (`POST /system/shutdown`) first. Warn loudly instead of silently
        // deleting a service whose process keeps running (LIF-01).
        if let Ok(status) = service.query_status()
            && status.current_state != ServiceState::Stopped
        {
            let _ = service.stop();
            std::thread::sleep(Duration::from_secs(3));

            let still_running = service
                .query_status()
                .map(|status| status.current_state != ServiceState::Stopped)
                .unwrap_or(true);
            if still_running {
                warn!(
                    "The engine service is still running after the stop attempt: the engine \
                         refuses SCM STOP by design. It will keep blocking and hold its database \
                         until it is shut down via POST /system/shutdown (or the machine \
                         reboots). The service registration is being deleted anyway."
                );
            }
        }

        service.delete().wrap_err("failed to delete service")?;

        info!("Windows service '{}' uninstalled.", SERVICE_NAME);
        Ok(())
    }

    // -------------------------------------------------------------------
    // Service entry point
    // -------------------------------------------------------------------

    /// Stores the result of the service run so `try_run_as_service` can
    /// propagate errors back to the caller.
    static SERVICE_RESULT: OnceLock<eyre::Result<()>> = OnceLock::new();

    /// Whether this process was launched by the SCM as a service. Used to
    /// disable user-mode hardening that would fight the service lifecycle
    /// (the watchdog must not resurrect the engine after `sc stop`).
    static IN_SERVICE_MODE: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);

    /// Mark this process as running under the SCM.
    fn set_in_service_mode() {
        IN_SERVICE_MODE.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Whether this process was launched by the SCM as a service.
    pub fn in_service_mode() -> bool {
        IN_SERVICE_MODE.load(std::sync::atomic::Ordering::Relaxed)
    }

    define_windows_service!(ffi_service_main, my_service_main);

    /// Service control handler. The engine deliberately does NOT accept STOP:
    /// `sc stop`, `net stop` and Task Manager's Services tab all fail, so an
    /// admin cannot stop the engine through the SCM. The engine shuts itself
    /// down only via its own API (`POST /system/shutdown`), which reports
    /// `SERVICE_STOPPED` first (see [`report_service_stopped`]).
    fn service_event_handler(
        _status_handle: ServiceStatusHandle,
        control_event: ServiceControl,
    ) -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => ServiceControlHandlerResult::NotImplemented,
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    }

    /// Report `SERVICE_STOPPED` so the SCM records a clean stop. Called by the
    /// engine's own shutdown path before the process exits; without it the SCM
    /// would treat the exit as an abnormal termination and apply the failure
    /// actions (restart). No-op when not running as a service.
    pub fn report_service_stopped() {
        if let Some(handle) = SERVICE_STATUS_HANDLE.lock().unwrap().as_ref() {
            let _ = handle.set_service_status(ServiceStatus {
                service_type: ServiceType::OWN_PROCESS,
                current_state: ServiceState::Stopped,
                controls_accepted: ServiceControlAccept::empty(),
                exit_code: ServiceExitCode::NO_ERROR,
                checkpoint: 0,
                wait_hint: Duration::ZERO,
                process_id: None,
            });
        }
    }

    /// Service entry point invoked by the SCM on a background thread.
    fn my_service_main(_arguments: Vec<OsString>) {
        let outcome = (|| -> eyre::Result<()> {
            set_in_service_mode();

            #[cfg(feature = "experimental-ace-guard")]
            if let Err(error) =
                crate::ace_guard::register_self(crate::ace_guard::AceGuardRole::Engine)
            {
                warn!("Ace Guard engine registration unavailable: {error}");
            } else {
                info!("Registered AceEngine with Ace Guard.");
            }

            if let Err(error) = hide_important_files() {
                warn!("File hiding failed: {:#}", error);
            }

            // Complete the SCM start handshake: without a registered control
            // handler and a SERVICE_RUNNING report, the service stays
            // START_PENDING until the SCM's start timeout fires (4 minutes),
            // which fails MSI installs and `sc start`. Register first so
            // Stop requests are delivered, then report RUNNING — the engine
            // is considered up once its main loop begins.
            //
            // The control handler needs the status handle, which only exists
            // after registration succeeds, so the handle is handed over
            // through a shared cell.
            let status_handle_cell = std::sync::Arc::new(std::sync::Mutex::new(None));
            let handler_cell = status_handle_cell.clone();
            let status_handle = service_control_handler::register(SERVICE_NAME, move |event| {
                match *handler_cell.lock().unwrap() {
                    Some(handle) => service_event_handler(handle, event),
                    None => ServiceControlHandlerResult::NotImplemented,
                }
            })
            .wrap_err("failed to register service control handler")?;
            *status_handle_cell.lock().unwrap() = Some(status_handle);
            *SERVICE_STATUS_HANDLE.lock().unwrap() = Some(status_handle);
            status_handle_cell
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .set_service_status(ServiceStatus {
                    service_type: ServiceType::OWN_PROCESS,
                    current_state: ServiceState::Running,
                    // Deliberately no STOP: the engine must not be stoppable
                    // through the SCM (see service_event_handler).
                    controls_accepted: ServiceControlAccept::empty(),
                    exit_code: ServiceExitCode::NO_ERROR,
                    checkpoint: 0,
                    wait_hint: Duration::from_secs(5),
                    process_id: None,
                })
                .wrap_err("failed to report SERVICE_RUNNING")?;

            let rt = tokio::runtime::Runtime::new().wrap_err("failed to create Tokio runtime")?;

            let run_result = rt.block_on(async { crate::run().await });
            if run_result.is_ok() {
                // Service-mode execution enters `crate::run()` directly rather
                // than the console wrapper in main.rs, so the clean-stop report
                // must live here too. Error exits deliberately remain failures
                // so SCM recovery still applies.
                report_service_stopped();
            }
            run_result
        })();

        let _ = SERVICE_RESULT.set(outcome);
    }

    /// If launched by the SCM, run the service and return `Ok(true)`.
    /// Otherwise return `Ok(false)` so the caller runs as a console process.
    pub fn try_run_as_service() -> Result<bool> {
        match service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
            Ok(()) => {
                // The dispatcher ran successfully.
                if let Some(r) = SERVICE_RESULT.get()
                    && let Err(error) = r
                {
                    return Err(eyre::eyre!("Service failed: {:#}", error));
                }
                Ok(true)
            }
            Err(windows_service::Error::Winapi(error)) if error.raw_os_error() == Some(1063) => {
                // ERROR_FAILED_SERVICE_CONTROLLER_CONNECT (1063)
                // means we were not launched by the SCM.
                Ok(false)
            }
            Err(error) => Err(eyre::eyre!("StartServiceCtrlDispatcherW failed: {}", error)),
        }
    }

    /// Ensure the production watchdog remains under SCM supervision. The
    /// engine calls this only when its watchdog IPC connection is absent; it
    /// never launches a SYSTEM process directly.
    pub fn ensure_watchdog_service_running() -> Result<()> {
        const WATCHDOG_SERVICE_NAME: &str = "AceEngineWatchdog";

        let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
            .wrap_err("failed to connect to service control manager")?;
        let service = match manager.open_service(
            WATCHDOG_SERVICE_NAME,
            ServiceAccess::START | ServiceAccess::QUERY_STATUS,
        ) {
            Ok(service) => service,
            Err(windows_service::Error::Winapi(error)) if error.raw_os_error() == Some(1060) => {
                // Console/dev installations legitimately do not have the
                // production watchdog service.
                return Ok(());
            }
            Err(error) => return Err(error).wrap_err("failed to open watchdog service"),
        };

        let status = service
            .query_status()
            .wrap_err("failed to query watchdog service status")?;
        match status.current_state {
            ServiceState::Running | ServiceState::StartPending => Ok(()),
            ServiceState::Stopped => {
                match service.start(&[] as &[&std::ffi::OsStr]) {
                    Ok(()) => {
                        info!("Watchdog restarted by engine through the SCM.");
                        Ok(())
                    }
                    Err(error) => {
                        let current = service
                            .query_status()
                            .wrap_err("watchdog start failed and status re-check failed")?;
                        if matches!(current.current_state, ServiceState::Running | ServiceState::StartPending) {
                            Ok(())
                        } else {
                            Err(error).wrap_err("failed to start watchdog service")
                        }
                    }
                }
            }
            _ => Ok(()),
        }
    }

    // -------------------------------------------------------------------
    // Legacy console guardian (not the installed production watchdog)
    // -------------------------------------------------------------------
    //
    // A restrictive DACL cannot stop an elevated administrator: Task Manager /
    // Process Explorer run with SeDebugPrivilege, which bypasses object DACLs
    // entirely. Instead of *preventing* the kill we make it pointless — the
    // engine and a lightweight guardian process watch each other and each
    // restarts the other within ~1s. Combined with the SCM recovery actions,
    // an interactive "End task" (or even `sc stop`) self-heals almost at once.
    // To take the engine down for real you must kill both faster than either
    // can resurrect the other, which is not possible one-click at a time.

    /// CREATE_NO_WINDOW — keep the headless guardian invisible.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    /// Argument that switches a spawned copy of this binary into guardian mode.
    pub const GUARDIAN_ARG: &str = "--guardian";

    /// Spawn a guardian child that watches this (engine) process, and keep a
    /// monitor thread that re-spawns the guardian if *it* is killed.
    pub fn spawn_watchdog() -> Result<()> {
        let exe = std::env::current_exe().wrap_err("cannot determine current exe for watchdog")?;
        let engine_pid = std::process::id();

        std::thread::Builder::new()
            .name("watchdog-monitor".into())
            .spawn(move || {
                loop {
                    match spawn_guardian(&exe, engine_pid) {
                        Ok(mut child) => {
                            // Block until the guardian exits, then respawn it.
                            let _ = child.wait();
                            warn!("Watchdog guardian exited; respawning.");
                        }
                        Err(error) => warn!("Failed to spawn watchdog guardian: {error}"),
                    }
                    std::thread::sleep(Duration::from_millis(500));
                }
            })
            .wrap_err("failed to spawn watchdog monitor thread")?;

        info!("Watchdog active (engine pid {engine_pid}).");
        Ok(())
    }

    /// Launch one guardian process: `<self> --guardian <engine_pid>`.
    fn spawn_guardian(exe: &Path, engine_pid: u32) -> std::io::Result<std::process::Child> {
        Command::new(exe)
            .arg(GUARDIAN_ARG)
            .arg(engine_pid.to_string())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
    }

    /// Guardian entry point. Waits for the engine to exit, restarts it, then
    /// returns so the process can exit — the freshly started engine spawns its
    /// own guardian. Never returns while the engine is alive.
    pub fn run_guardian(engine_pid: u32) -> ! {
        wait_for_process_exit(engine_pid);
        warn!("Guardian: engine (pid {engine_pid}) exited; restarting.");

        // Prefer restarting via the SCM so the engine re-enters the hardened
        // SYSTEM service context. Retry briefly to ride out the window where
        // the SCM has not yet noticed the old process died. Fall back to
        // relaunching the executable directly (error.g. console / no service).
        let mut restarted = false;
        for _ in 0..20 {
            match start_service() {
                ServiceStart::Started | ServiceStart::AlreadyRunning => {
                    restarted = true;
                    break;
                }
                ServiceStart::NoService => break,
                ServiceStart::Retry => std::thread::sleep(Duration::from_millis(500)),
            }
        }
        if !restarted && let Err(error) = relaunch_executable() {
            warn!("Guardian: fallback relaunch failed: {error}");
        }

        std::process::exit(0);
    }

    /// Block until the process with `pid` exits. Returns immediately if the
    /// process cannot be opened (already gone).
    fn wait_for_process_exit(pid: u32) {
        use windows::Win32::Foundation::CloseHandle;
        unsafe {
            let handle = match OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
                Ok(h) if !h.is_invalid() => h,
                _ => return,
            };
            WaitForSingleObject(handle, u32::MAX); // INFINITE
            let _ = CloseHandle(handle);
        }
    }

    /// Outcome of attempting to (re)start the installed service.
    enum ServiceStart {
        Started,
        AlreadyRunning,
        NoService,
        Retry,
    }

    /// Try to start the installed Windows service via the service control
    /// manager.
    fn start_service() -> ServiceStart {
        let manager =
            match ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT) {
                Ok(m) => m,
                Err(_) => return ServiceStart::Retry,
            };

        let service = match manager.open_service(
            SERVICE_NAME,
            ServiceAccess::START | ServiceAccess::QUERY_STATUS,
        ) {
            Ok(s) => s,
            Err(windows_service::Error::Winapi(error)) if error.raw_os_error() == Some(1060) => {
                return ServiceStart::NoService;
            }
            Err(_) => return ServiceStart::Retry,
        };

        match service.start(&[] as &[&std::ffi::OsStr]) {
            Ok(()) => ServiceStart::Started,
            Err(_) => {
                // Check if the service is already running.
                match service.query_status() {
                    Ok(status) if status.current_state == ServiceState::Running => {
                        ServiceStart::AlreadyRunning
                    }
                    _ => ServiceStart::Retry,
                }
            }
        }
    }

    /// Relaunch the engine executable directly (used when no service exists).
    fn relaunch_executable() -> std::io::Result<()> {
        let exe = std::env::current_exe()?;
        Command::new(exe).creation_flags(CREATE_NO_WINDOW).spawn()?;
        Ok(())
    }

    // -------------------------------------------------------------------
    // File hiding (tamper protection)
    // -------------------------------------------------------------------

    /// Mark a file or directory as HIDDEN + SYSTEM.
    pub fn set_file_hide(path: &Path) -> bool {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            SetFileAttributesW(
                PCWSTR(wide.as_ptr()),
                FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM | FILE_ATTRIBUTE_ARCHIVE,
            )
            .is_ok()
        }
    }

    /// Restore a file or directory to normal visibility.
    pub fn restore_file_hide(path: &Path) -> bool {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe { SetFileAttributesW(PCWSTR(wide.as_ptr()), FILE_ATTRIBUTE_NORMAL).is_ok() }
    }

    // -------------------------------------------------------------------
    // Bulk file protection (called at startup)
    // -------------------------------------------------------------------

    /// Hide the app-data directory and installation directory for tamper
    /// protection.
    pub fn hide_important_files() -> Result<()> {
        let app_data_dir = Settings::get_app_data_dir();
        let parent_dir = app_data_dir
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| app_data_dir.clone());

        if !set_file_hide(&parent_dir) {
            warn!("Failed to hide app-data parent: {:?}", parent_dir);
        }

        let install_dir = Settings::get_install_dir();
        if install_dir != app_data_dir && install_dir.exists() && !set_file_hide(&install_dir) {
            warn!("Failed to hide install directory: {:?}", install_dir);
        }

        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod linux_imp {
    use crate::watchdog_protocol::{
        LINUX_ENGINE_SERVICE_NAME, LINUX_WATCHDOG_SERVICE_NAME,
    };
    use eyre::{Context, Result, bail};
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};
    use tracing::info;

    const SYSTEMD_UNIT_DIR: &str = "/etc/systemd/system";

    /// Install the engine and watchdog as systemd services. The executable
    /// paths are captured from the installed binaries, so no distro-specific
    /// installation prefix is hard-coded here.
    pub fn install_service() -> Result<()> {
        let engine_path = std::env::current_exe()
            .wrap_err("cannot determine current executable path for systemd install")?
            .canonicalize()
            .wrap_err("cannot canonicalize ace-engine path for systemd install")?;
        let watchdog_path = engine_path.with_file_name("ace-watchdog");
        if !watchdog_path.is_file() {
            bail!(
                "ace-watchdog binary is missing next to ace-engine: {}",
                watchdog_path.display()
            );
        }
        let watchdog_path = watchdog_path
            .canonicalize()
            .wrap_err("cannot canonicalize ace-watchdog path for systemd install")?;

        let engine_unit = engine_unit_contents(&engine_path);
        let watchdog_unit = watchdog_unit_contents(&watchdog_path);
        let engine_unit_path = unit_path(LINUX_ENGINE_SERVICE_NAME);
        let watchdog_unit_path = unit_path(LINUX_WATCHDOG_SERVICE_NAME);

        std::fs::write(&engine_unit_path, engine_unit).wrap_err_with(|| {
            format!("failed to write {}", engine_unit_path.display())
        })?;
        std::fs::write(&watchdog_unit_path, watchdog_unit).wrap_err_with(|| {
            format!("failed to write {}", watchdog_unit_path.display())
        })?;

        require_systemctl_success(&["daemon-reload"])?;
        require_systemctl_success(&[
            "enable",
            LINUX_ENGINE_SERVICE_NAME,
            LINUX_WATCHDOG_SERVICE_NAME,
        ])?;

        info!(
            "systemd services '{}' and '{}' installed and enabled.",
            LINUX_ENGINE_SERVICE_NAME, LINUX_WATCHDOG_SERVICE_NAME
        );
        Ok(())
    }

    /// Remove the systemd registrations. The protected engine must already have
    /// been shut down through its authenticated API so the watchdog maintenance
    /// lease is active and databases are closed cleanly.
    pub fn uninstall_service() -> Result<()> {
        let engine_active = systemctl(&["is-active", "--quiet", LINUX_ENGINE_SERVICE_NAME])?;
        if engine_active.status.success() {
            bail!(
                "{} is still running; shut the engine down through POST /system/shutdown before uninstalling",
                LINUX_ENGINE_SERVICE_NAME
            );
        }

        // Once the engine is down, it can no longer resurrect the watchdog.
        // A missing/disabled unit is harmless here, so non-zero disable status
        // is tolerated while command-execution failures are still surfaced.
        let _ = systemctl(&["disable", "--now", LINUX_WATCHDOG_SERVICE_NAME])?;
        let _ = systemctl(&["disable", LINUX_ENGINE_SERVICE_NAME])?;

        remove_unit_if_present(LINUX_WATCHDOG_SERVICE_NAME)?;
        remove_unit_if_present(LINUX_ENGINE_SERVICE_NAME)?;
        require_systemctl_success(&["daemon-reload"])?;
        let _ = systemctl(&[
            "reset-failed",
            LINUX_ENGINE_SERVICE_NAME,
            LINUX_WATCHDOG_SERVICE_NAME,
        ])?;

        info!("Ace Blocker systemd services uninstalled.");
        Ok(())
    }

    fn engine_unit_contents(engine_path: &Path) -> String {
        format!(
            "[Unit]\n\
Description=Ace Blocker Engine\n\
After=network-online.target\n\
Wants=network-online.target\n\
RefuseManualStop=yes\n\
StartLimitIntervalSec=60\n\
StartLimitBurst=5\n\
\n\
[Service]\n\
Type=simple\n\
ExecStart={}\n\
Restart=on-failure\n\
RestartSec=2s\n\
KillMode=control-group\n\
\n\
[Install]\n\
WantedBy=multi-user.target\n",
            systemd_quote_path(engine_path)
        )
    }

    fn watchdog_unit_contents(watchdog_path: &Path) -> String {
        format!(
            "[Unit]\n\
Description=Ace Blocker Watchdog\n\
After=basic.target\n\
StartLimitIntervalSec=60\n\
StartLimitBurst=5\n\
\n\
[Service]\n\
Type=simple\n\
ExecStart={}\n\
Restart=always\n\
RestartSec=2s\n\
RuntimeDirectory=ace-blocker\n\
RuntimeDirectoryMode=0700\n\
UMask=0077\n\
\n\
[Install]\n\
WantedBy=multi-user.target\n",
            systemd_quote_path(watchdog_path)
        )
    }

    fn systemd_quote_path(path: &Path) -> String {
        use std::fmt::Write as _;

        let mut escaped = String::from("\"");
        for byte in path.as_os_str().as_bytes() {
            match *byte {
                b'%' => escaped.push_str("%%"),
                b'\\' => escaped.push_str("\\\\"),
                b'"' => escaped.push_str("\\\""),
                0x20..=0x7e => escaped.push(char::from(*byte)),
                other => {
                    let _ = write!(escaped, "\\x{other:02x}");
                }
            }
        }
        escaped.push('"');
        escaped
    }

    fn unit_path(service_name: &str) -> PathBuf {
        Path::new(SYSTEMD_UNIT_DIR).join(service_name)
    }

    fn remove_unit_if_present(service_name: &str) -> Result<()> {
        let path = unit_path(service_name);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error).wrap_err_with(|| format!("failed to remove {}", path.display())),
        }
    }

    fn require_systemctl_success(args: &[&str]) -> Result<()> {
        let output = systemctl(args)?;
        if output.status.success() {
            Ok(())
        } else {
            bail!(
                "systemctl {} failed: {}",
                args.join(" "),
                stderr_text(&output)
            )
        }
    }

    /// systemd sets INVOCATION_ID for service processes. Console launches do
    /// not get this environment variable, so watchdog recovery stays disabled
    /// during ordinary development runs.
    pub fn in_service_mode() -> bool {
        std::env::var_os("INVOCATION_ID").is_some()
    }

    pub fn ensure_watchdog_service_running() -> Result<()> {
        if !in_service_mode() {
            return Ok(());
        }

        let load_state = systemctl(&[
            "show",
            "--property=LoadState",
            "--value",
            LINUX_WATCHDOG_SERVICE_NAME,
        ])?;
        if !load_state.status.success() {
            bail!(
                "failed to query {} load state: {}",
                LINUX_WATCHDOG_SERVICE_NAME,
                stderr_text(&load_state)
            );
        }

        if String::from_utf8_lossy(&load_state.stdout).trim() == "not-found" {
            // Console/dev installations can legitimately omit the production
            // watchdog unit, matching the Windows missing-service behavior.
            return Ok(());
        }

        let active = systemctl(&["is-active", "--quiet", LINUX_WATCHDOG_SERVICE_NAME])?;
        if active.status.success() {
            return Ok(());
        }

        let start = systemctl(&["start", LINUX_WATCHDOG_SERVICE_NAME])?;
        if !start.status.success() {
            bail!(
                "failed to start {}: {}",
                LINUX_WATCHDOG_SERVICE_NAME,
                stderr_text(&start)
            );
        }

        info!("Watchdog restarted by engine through systemd.");
        Ok(())
    }

    fn systemctl(args: &[&str]) -> Result<Output> {
        Command::new("systemctl")
            .args(args)
            .output()
            .wrap_err_with(|| format!("failed to execute systemctl {}", args.join(" ")))
    }

    fn stderr_text(output: &Output) -> String {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if message.is_empty() {
            format!("systemctl exited with {}", output.status)
        } else {
            message
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn systemd_exec_paths_are_quoted_and_percent_escaped() {
            assert_eq!(
                systemd_quote_path(Path::new("/opt/Ace Blocker/ace-engine")),
                "\"/opt/Ace Blocker/ace-engine\""
            );
            assert_eq!(
                systemd_quote_path(Path::new("/opt/100%/ace-engine")),
                "\"/opt/100%%/ace-engine\""
            );
        }

        #[test]
        fn generated_units_have_required_recovery_and_runtime_settings() {
            let engine = engine_unit_contents(Path::new("/opt/Ace Blocker/ace-engine"));
            assert!(engine.contains("ExecStart=\"/opt/Ace Blocker/ace-engine\""));
            assert!(engine.contains("RefuseManualStop=yes"));
            assert!(engine.contains("Restart=on-failure"));

            let watchdog = watchdog_unit_contents(Path::new("/opt/Ace Blocker/ace-watchdog"));
            assert!(watchdog.contains("ExecStart=\"/opt/Ace Blocker/ace-watchdog\""));
            assert!(watchdog.contains("Restart=always"));
            assert!(watchdog.contains("RuntimeDirectory=ace-blocker"));
            assert!(watchdog.contains("RuntimeDirectoryMode=0700"));
            assert!(watchdog.contains("UMask=0077"));
        }
    }
}

// =========================================================================
// Platform-neutral public API
// =========================================================================

#[cfg(target_os = "windows")]
pub use imp::{
    GUARDIAN_ARG, ensure_watchdog_service_running, hide_important_files, in_service_mode,
    install_service, report_service_stopped, restore_file_hide, run_guardian, set_file_hide,
    spawn_watchdog, try_run_as_service, uninstall_service,
};

#[cfg(target_os = "linux")]
pub use linux_imp::{
    ensure_watchdog_service_running, in_service_mode, install_service, uninstall_service,
};

// Non-Windows stubs.

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn install_service() -> Result<()> {
    Ok(())
}
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn uninstall_service() -> Result<()> {
    Ok(())
}
#[cfg(not(target_os = "windows"))]
pub fn try_run_as_service() -> Result<bool> {
    Ok(false)
}
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn in_service_mode() -> bool {
    false
}
#[cfg(not(target_os = "windows"))]
pub fn spawn_watchdog() -> Result<()> {
    Ok(())
}
#[cfg(not(target_os = "windows"))]
pub fn set_file_hide(_path: &Path) -> bool {
    true
}
#[cfg(not(target_os = "windows"))]
pub fn restore_file_hide(_path: &Path) -> bool {
    true
}
#[cfg(not(target_os = "windows"))]
pub fn hide_important_files() -> Result<()> {
    Ok(())
}
#[cfg(not(target_os = "windows"))]
pub fn report_service_stopped() {}
