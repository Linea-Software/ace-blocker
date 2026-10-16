#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(any(target_os = "windows", target_os = "linux"))]
#[allow(dead_code)] // this binary reuses a protocol module that also exposes engine-only constants
use ace_watchdog_protocol as watchdog_protocol;


#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn main() {
    eprintln!("ace-watchdog is only supported on Windows and Linux");
}

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod linux_impl;

#[cfg(target_os = "windows")]
fn main() {
    if let Err(error) = windows_impl::run_dispatcher() {
        windows_impl::log_event(&format!("watchdog dispatcher failed: {error:#}"));
        std::process::exit(1);
    }
}

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() {
    if let Err(error) = linux_impl::run().await {
        eprintln!("ace-watchdog failed: {error:#}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use crate::watchdog_protocol::{
        clamp_maintenance_ttl, recovery_delay, EngineInstanceId, EngineToWatchdog, WatchdogNonce, WatchdogToEngine,
        ENGINE_SERVICE_NAME, ENGINE_STARTUP_GRACE, HEARTBEAT_FAILURE_THRESHOLD,
        HEARTBEAT_INTERVAL, HEARTBEAT_TIMEOUT, WATCHDOG_PIPE_NAME, WATCHDOG_SERVICE_NAME,
    };
    use eyre::{eyre, Context, Result};
    use std::ffi::OsString;
    use std::io::Write;
    use std::os::windows::io::AsRawHandle;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, MutexGuard};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
    use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
    use tokio::time;
    use uuid::Uuid;
    use windows::core::w;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
    use windows::Win32::System::Pipes::GetNamedPipeClientProcessId;
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    use windows_service::{
        define_windows_service,
        service::{
            ServiceAccess, ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState,
            ServiceStatus, ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult},
        service_dispatcher,
        service_manager::{ServiceManager, ServiceManagerAccess},
    };

    const MONITOR_INTERVAL: Duration = Duration::from_secs(2);
    const SCM_RECOVERY_GRACE: Duration = Duration::from_secs(3);
    const STABLE_RESET_INTERVAL: Duration = Duration::from_secs(5 * 60);
    const PIPE_HELLO_TIMEOUT: Duration = Duration::from_secs(5);

    define_windows_service!(ffi_watchdog_main, watchdog_service_main);

    #[derive(Debug)]
    enum WatchdogState {
        Starting { process_id: Option<u32> },
        Healthy { process_id: u32, instance_id: EngineInstanceId },
        Unhealthy { process_id: u32, failures: u32 },
        Recovering { process_id: Option<u32>, attempt: u32 },
        Maintenance { until: Instant },
        Backoff { until: Instant, failures: u32 },
    }

    #[derive(Clone, Debug, Default)]
    struct HeartbeatSnapshot {
        client_process_id: Option<u32>,
        engine_instance_id: Option<EngineInstanceId>,
        last_success: Option<Instant>,
        consecutive_failures: u32,
        maintenance_until: Option<Instant>,
    }

    #[derive(Clone, Copy, Debug)]
    struct EngineServiceStatus {
        state: ServiceState,
        process_id: Option<u32>,
    }

    struct MonitorContext {
        state: WatchdogState,
        observed_process_id: Option<u32>,
        process_observed_at: Instant,
        stopped_since: Option<Instant>,
        disconnected_failures: u32,
        recovery_failures: u32,
        next_recovery_at: Instant,
        stable_since: Option<Instant>,
    }

    impl MonitorContext {
        fn new() -> Self {
            Self {
                state: WatchdogState::Starting { process_id: None },
                observed_process_id: None,
                process_observed_at: Instant::now(),
                stopped_since: None,
                disconnected_failures: 0,
                recovery_failures: 0,
                next_recovery_at: Instant::now(),
                stable_since: None,
            }
        }

        fn observe_process(&mut self, process_id: u32, now: Instant) {
            if self.observed_process_id != Some(process_id) {
                self.observed_process_id = Some(process_id);
                self.process_observed_at = now;
                self.disconnected_failures = 0;
                self.stable_since = None;
            }
        }

        fn transition(&mut self, next: WatchdogState) {
            if std::mem::discriminant(&self.state) != std::mem::discriminant(&next) {
                log_event(&format!("watchdog state: {next:?}"));
            }
            self.state = next;
        }

        fn note_recovery_attempt(&mut self, now: Instant) {
            self.recovery_failures = self.recovery_failures.saturating_add(1);
            self.next_recovery_at = now + recovery_delay(self.recovery_failures);
            self.stable_since = None;
        }

        fn note_stable(&mut self, now: Instant) {
            let stable_since = self.stable_since.get_or_insert(now);
            if now.duration_since(*stable_since) >= STABLE_RESET_INTERVAL {
                self.recovery_failures = 0;
                self.next_recovery_at = now;
            }
        }
    }

    pub fn run_dispatcher() -> Result<()> {
        service_dispatcher::start(WATCHDOG_SERVICE_NAME, ffi_watchdog_main)
            .wrap_err("StartServiceCtrlDispatcherW failed")?;
        Ok(())
    }

    fn watchdog_service_main(_arguments: Vec<OsString>) {
        #[cfg(feature = "experimental-ace-guard")]
        if let Err(error) =
            ace_engine::ace_guard::register_self(ace_engine::ace_guard::AceGuardRole::Watchdog)
        {
            log_event(&format!("Ace Guard watchdog registration unavailable: {error}"));
        } else {
            log_event("registered AceEngineWatchdog with Ace Guard");
        }

        let stop_requested = Arc::new(AtomicBool::new(false));
        let handler_stop = Arc::clone(&stop_requested);
        let shared_health = Arc::new(Mutex::new(HeartbeatSnapshot::default()));
        let handler_health = Arc::clone(&shared_health);
        let status_handle = match service_control_handler::register(
            WATCHDOG_SERVICE_NAME,
            move |control_event| match control_event {
                ServiceControl::Stop => {
                    let maintenance_is_active = lock_health(&handler_health)
                        .maintenance_until
                        .is_some_and(|until| Instant::now() < until);
                    if maintenance_is_active {
                        handler_stop.store(true, Ordering::Release);
                        ServiceControlHandlerResult::NoError
                    } else {
                        ServiceControlHandlerResult::NotImplemented
                    }
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            },
        ) {
            Ok(handle) => handle,
            Err(error) => {
                log_event(&format!("failed to register watchdog control handler: {error}"));
                return;
            }
        };

        if let Err(error) = status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::NO_ERROR,
            checkpoint: 0,
            wait_hint: Duration::ZERO,
            process_id: None,
        }) {
            log_event(&format!("failed to report watchdog SERVICE_RUNNING: {error}"));
            return;
        }

        log_event("watchdog started");
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(runtime) => runtime,
            Err(error) => {
                log_event(&format!("failed to create watchdog runtime: {error}"));
                let _ = status_handle.set_service_status(stopped_status(1));
                return;
            }
        };

        let outcome = runtime.block_on(run_watchdog(
            Arc::clone(&stop_requested),
            shared_health,
        ));
        match outcome {
            Ok(()) => {
                log_event("watchdog stopped");
                let _ = status_handle.set_service_status(stopped_status(0));
            }
            Err(error) => {
                log_event(&format!("watchdog service failed: {error:#}"));
                let _ = status_handle.set_service_status(stopped_status(1));
            }
        }
    }

    fn stopped_status(error_code: u32) -> ServiceStatus {
        ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: if error_code == 0 {
                ServiceExitCode::NO_ERROR
            } else {
                ServiceExitCode::ServiceSpecific(error_code)
            },
            checkpoint: 0,
            wait_hint: Duration::ZERO,
            process_id: None,
        }
    }

    async fn run_watchdog(
        stop_requested: Arc<AtomicBool>,
        shared_health: Arc<Mutex<HeartbeatSnapshot>>,
    ) -> Result<()> {
        tokio::try_join!(
            monitor_engine(Arc::clone(&shared_health), Arc::clone(&stop_requested)),
            run_pipe_server(shared_health, stop_requested),
        )?;
        Ok(())
    }

    async fn monitor_engine(
        shared_health: Arc<Mutex<HeartbeatSnapshot>>,
        stop_requested: Arc<AtomicBool>,
    ) -> Result<()> {
        let mut context = MonitorContext::new();
        let mut interval = time::interval(MONITOR_INTERVAL);
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

        loop {
            interval.tick().await;
            if stop_requested.load(Ordering::Acquire) {
                return Ok(());
            }

            let now = Instant::now();
            expire_maintenance_if_needed(&shared_health, now);
            let snapshot = snapshot_health(&shared_health);
            if let Some(until) = snapshot.maintenance_until.filter(|until| *until > now) {
                context.transition(WatchdogState::Maintenance { until });
                continue;
            }

            let service_status = match query_engine_service() {
                Ok(status) => status,
                Err(error) => {
                    context.recovery_failures = context.recovery_failures.saturating_add(1);
                    context.next_recovery_at = now + recovery_delay(context.recovery_failures);
                    log_event(&format!("failed to query AceEngine service: {error:#}"));
                    context.transition(WatchdogState::Backoff {
                        until: context.next_recovery_at,
                        failures: context.recovery_failures,
                    });
                    continue;
                }
            };

            let Some(service_status) = service_status else {
                log_event("AceEngine service registration is missing");
                context.transition(WatchdogState::Backoff {
                    until: now + Duration::from_secs(60),
                    failures: context.recovery_failures,
                });
                continue;
            };

            match service_status.state {
                ServiceState::Stopped => {
                    handle_stopped_engine(&mut context, now)?;
                }
                ServiceState::StartPending
                | ServiceState::StopPending
                | ServiceState::ContinuePending
                | ServiceState::PausePending => {
                    context.transition(WatchdogState::Starting {
                        process_id: service_status.process_id,
                    });
                }
                ServiceState::Running => {
                    let Some(process_id) = service_status.process_id else {
                        context.transition(WatchdogState::Starting { process_id: None });
                        continue;
                    };
                    handle_running_engine(
                        &mut context,
                        &snapshot,
                        process_id,
                        now,
                    )?;
                }
                ServiceState::Paused => {
                    context.transition(WatchdogState::Unhealthy {
                        process_id: service_status.process_id.unwrap_or_default(),
                        failures: HEARTBEAT_FAILURE_THRESHOLD,
                    });
                }
            }
        }
    }

    fn handle_stopped_engine(context: &mut MonitorContext, now: Instant) -> Result<()> {
        context.observed_process_id = None;
        context.stable_since = None;
        if context.stopped_since.is_none() {
            context.stopped_since = Some(now);
            context.next_recovery_at = now + SCM_RECOVERY_GRACE;
            log_event("engine unexpectedly stopped; allowing SCM recovery grace period");
        }

        if now < context.next_recovery_at {
            context.transition(WatchdogState::Backoff {
                until: context.next_recovery_at,
                failures: context.recovery_failures,
            });
            return Ok(());
        }

        log_event("engine restart requested through SCM");
        context.transition(WatchdogState::Recovering {
            process_id: None,
            attempt: context.recovery_failures.saturating_add(1),
        });
        match start_engine_service() {
            Ok(()) => {
                log_event("engine restart request accepted by SCM");
                context.note_recovery_attempt(now);
            }
            Err(error) => {
                log_event(&format!("engine restart request failed: {error:#}"));
                context.note_recovery_attempt(now);
            }
        }
        Ok(())
    }

    fn handle_running_engine(
        context: &mut MonitorContext,
        snapshot: &HeartbeatSnapshot,
        process_id: u32,
        now: Instant,
    ) -> Result<()> {
        context.stopped_since = None;
        context.observe_process(process_id, now);

        if now.duration_since(context.process_observed_at) < ENGINE_STARTUP_GRACE {
            context.transition(WatchdogState::Starting {
                process_id: Some(process_id),
            });
            return Ok(());
        }

        let heartbeat_is_fresh = snapshot.client_process_id == Some(process_id)
            && snapshot.engine_instance_id.is_some()
            && snapshot.consecutive_failures < HEARTBEAT_FAILURE_THRESHOLD
            && snapshot
                .last_success
                .is_some_and(|last_success| now.duration_since(last_success) <= HEARTBEAT_INTERVAL + HEARTBEAT_TIMEOUT + Duration::from_secs(1));

        if heartbeat_is_fresh {
            let Some(instance_id) = snapshot.engine_instance_id else {
                return Ok(());
            };
            context.disconnected_failures = 0;
            context.note_stable(now);
            context.transition(WatchdogState::Healthy {
                process_id,
                instance_id,
            });
            return Ok(());
        }

        let failures = if snapshot.client_process_id == Some(process_id) {
            snapshot.consecutive_failures
        } else {
            context.disconnected_failures = context.disconnected_failures.saturating_add(1);
            context.disconnected_failures
        };
        context.transition(WatchdogState::Unhealthy {
            process_id,
            failures,
        });

        if failures < HEARTBEAT_FAILURE_THRESHOLD {
            return Ok(());
        }
        if now < context.next_recovery_at {
            context.transition(WatchdogState::Backoff {
                until: context.next_recovery_at,
                failures: context.recovery_failures,
            });
            return Ok(());
        }

        log_event(&format!(
            "engine declared hung after {failures} heartbeat failures; verifying SCM PID {process_id}"
        ));
        terminate_verified_engine_process(process_id)?;
        log_event(&format!("verified engine PID {process_id} terminated; SCM recovery will run"));
        context.note_recovery_attempt(now);
        context.next_recovery_at = now + SCM_RECOVERY_GRACE.max(recovery_delay(context.recovery_failures));
        context.transition(WatchdogState::Recovering {
            process_id: Some(process_id),
            attempt: context.recovery_failures,
        });
        Ok(())
    }

    async fn run_pipe_server(
        shared_health: Arc<Mutex<HeartbeatSnapshot>>,
        stop_requested: Arc<AtomicBool>,
    ) -> Result<()> {
        // ConvertStringSecurityDescriptorToSecurityDescriptor allocates this
        // descriptor. We retain the one descriptor for the lifetime of the
        // watchdog process so every recreated pipe instance uses the same
        // SYSTEM-only ACL; process exit releases it with the process heap.
        let mut security_attributes = pipe_security_attributes()?;

        loop {
            if stop_requested.load(Ordering::Acquire) {
                return Ok(());
            }

            let server = unsafe {
                // SAFETY: `security_attributes` is initialized from a valid SDDL
                // descriptor and outlives this CreateNamedPipe call and every
                // pipe handle created from it.
                ServerOptions::new()
                    .first_pipe_instance(true)
                    .reject_remote_clients(true)
                    .create_with_security_attributes_raw(
                        WATCHDOG_PIPE_NAME,
                        (&mut security_attributes as *mut SECURITY_ATTRIBUTES).cast(),
                    )
            }
            .wrap_err("failed to create restricted watchdog named pipe")?;

            tokio::select! {
                connect_result = server.connect() => {
                    connect_result.wrap_err("failed accepting watchdog pipe client")?;
                }
                _ = wait_for_stop(Arc::clone(&stop_requested)) => return Ok(()),
            }

            let client_process_id = named_pipe_client_process_id(&server)?;
            if current_engine_process_id()? != Some(client_process_id) {
                log_event(&format!(
                    "rejected watchdog pipe client PID {client_process_id}: not current SCM-owned AceEngine PID"
                ));
                continue;
            }

            if let Err(error) = serve_engine_connection(
                server,
                client_process_id,
                &shared_health,
                Arc::clone(&stop_requested),
            )
            .await
            {
                log_event(&format!("engine heartbeat connection ended: {error:#}"));
            }
            clear_connection(&shared_health, client_process_id);
        }
    }

    async fn serve_engine_connection(
        server: NamedPipeServer,
        client_process_id: u32,
        shared_health: &Arc<Mutex<HeartbeatSnapshot>>,
        stop_requested: Arc<AtomicBool>,
    ) -> Result<()> {
        let (read_half, mut write_half) = tokio::io::split(server);
        let mut lines = BufReader::new(read_half).lines();

        let hello_line = time::timeout(PIPE_HELLO_TIMEOUT, lines.next_line())
            .await
            .wrap_err("timed out waiting for engine watchdog hello")?
            .wrap_err("failed reading engine watchdog hello")?
            .ok_or_else(|| eyre!("engine closed watchdog pipe before hello"))?;
        let hello: EngineToWatchdog = serde_json::from_str(&hello_line)
            .wrap_err("invalid engine watchdog hello message")?;
        let engine_instance_id = match hello {
            EngineToWatchdog::Hello { engine_instance_id } => engine_instance_id,
            _ => return Err(eyre!("first watchdog pipe message was not hello")),
        };

        if current_engine_process_id()? != Some(client_process_id) {
            return Err(eyre!("AceEngine SCM PID changed during watchdog handshake"));
        }
        set_connected_engine(shared_health, client_process_id, &engine_instance_id);
        log_event(&format!(
            "engine heartbeat connected: SCM PID {client_process_id}, instance {engine_instance_id}"
        ));

        let mut sequence = 0_u64;
        let mut pending_ping: Option<(u64, WatchdogNonce, Instant)> = None;
        let mut interval = time::interval(HEARTBEAT_INTERVAL);
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
        interval.tick().await;

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Some((_, _, sent_at)) = &pending_ping
                        && sent_at.elapsed() >= HEARTBEAT_TIMEOUT
                    {
                        note_heartbeat_failure(shared_health);
                        pending_ping = None;
                    }

                    sequence = sequence.wrapping_add(1);
                    let nonce = WatchdogNonce::from(Uuid::new_v4());
                    write_watchdog_message(
                        &mut write_half,
                        &WatchdogToEngine::Ping { sequence, nonce: nonce.clone() },
                    ).await?;
                    pending_ping = Some((sequence, nonce, Instant::now()));
                }
                line = lines.next_line() => {
                    let Some(line) = line.wrap_err("failed reading engine heartbeat pipe")? else {
                        return Err(eyre!("engine closed heartbeat pipe"));
                    };
                    let message: EngineToWatchdog = match serde_json::from_str(&line) {
                        Ok(message) => message,
                        Err(error) => {
                            log_event(&format!("ignored malformed engine heartbeat message: {error}"));
                            continue;
                        }
                    };
                    handle_engine_message(
                        message,
                        client_process_id,
                        &engine_instance_id,
                        &mut pending_ping,
                        shared_health,
                        &mut write_half,
                    ).await?;
                }
                _ = wait_for_stop(Arc::clone(&stop_requested)) => return Ok(()),
            }
        }
    }

    async fn handle_engine_message<W: AsyncWrite + Unpin>(
        message: EngineToWatchdog,
        client_process_id: u32,
        expected_instance_id: &EngineInstanceId,
        pending_ping: &mut Option<(u64, WatchdogNonce, Instant)>,
        shared_health: &Arc<Mutex<HeartbeatSnapshot>>,
        writer: &mut W,
    ) -> Result<()> {
        match message {
            EngineToWatchdog::Hello { engine_instance_id } => {
                if engine_instance_id != *expected_instance_id {
                    return Err(eyre!("engine instance identity changed on an existing pipe"));
                }
            }
            EngineToWatchdog::Pong {
                sequence,
                nonce,
                engine_instance_id,
            } => {
                let matches_challenge = pending_ping
                    .as_ref()
                    .is_some_and(|(expected_sequence, expected_nonce, _)| {
                        *expected_sequence == sequence && *expected_nonce == nonce
                    });
                if engine_instance_id != *expected_instance_id || !matches_challenge {
                    log_event("ignored heartbeat pong with mismatched instance/challenge");
                    return Ok(());
                }
                pending_ping.take();
                note_heartbeat_success(shared_health, client_process_id, expected_instance_id);
            }
            EngineToWatchdog::BeginMaintenance {
                engine_instance_id,
                ttl_ms,
                reason,
            } => {
                if engine_instance_id != *expected_instance_id
                    || current_engine_process_id()? != Some(client_process_id)
                {
                    write_watchdog_message(
                        writer,
                        &WatchdogToEngine::MaintenanceRejected {
                            reason: "engine identity no longer matches SCM service instance".to_string(),
                        },
                    )
                    .await?;
                    return Ok(());
                }

                let requested = Duration::from_millis(ttl_ms);
                let ttl = clamp_maintenance_ttl(requested);
                let until = Instant::now() + ttl;
                {
                    let mut health = lock_health(shared_health);
                    health.maintenance_until = Some(until);
                }
                log_event(&format!(
                    "maintenance window opened for {} seconds ({reason:?})",
                    ttl.as_secs()
                ));
                write_watchdog_message(
                    writer,
                    &WatchdogToEngine::MaintenanceAccepted {
                        ttl_ms: u64::try_from(ttl.as_millis()).unwrap_or(u64::MAX),
                    },
                )
                .await?;
            }
        }
        Ok(())
    }

    fn pipe_security_attributes() -> Result<SECURITY_ATTRIBUTES> {
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        unsafe {
            // SAFETY: the SDDL literal is valid and `descriptor` is a valid
            // out-parameter. SYSTEM-only access is sufficient because both
            // services run as LocalSystem; the pipe also verifies the SCM PID.
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                w!("D:P(A;;GA;;;SY)"),
                SDDL_REVISION_1,
                &mut descriptor,
                None,
            )
            .wrap_err("failed to build watchdog pipe security descriptor")?;
        }
        Ok(SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: false.into(),
        })
    }

    fn named_pipe_client_process_id(server: &NamedPipeServer) -> Result<u32> {
        let mut process_id = 0_u32;
        let handle = HANDLE(server.as_raw_handle());
        unsafe {
            // SAFETY: the named pipe server owns a live Windows HANDLE for the
            // duration of this call and `process_id` is a valid out-parameter.
            GetNamedPipeClientProcessId(handle, &mut process_id)
                .wrap_err("GetNamedPipeClientProcessId failed")?;
        }
        Ok(process_id)
    }

    fn query_engine_service() -> Result<Option<EngineServiceStatus>> {
        let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
            .wrap_err("failed to connect to SCM")?;
        let service = match manager.open_service(
            ENGINE_SERVICE_NAME,
            ServiceAccess::QUERY_STATUS | ServiceAccess::START,
        ) {
            Ok(service) => service,
            Err(windows_service::Error::Winapi(error)) if error.raw_os_error() == Some(1060) => {
                return Ok(None);
            }
            Err(error) => return Err(error).wrap_err("failed to open AceEngine service"),
        };
        let status = service
            .query_status()
            .wrap_err("failed to query AceEngine service status")?;
        Ok(Some(EngineServiceStatus {
            state: status.current_state,
            process_id: status.process_id,
        }))
    }

    fn current_engine_process_id() -> Result<Option<u32>> {
        Ok(query_engine_service()?.and_then(|status| {
            matches!(status.state, ServiceState::Running | ServiceState::StartPending)
                .then_some(status.process_id)
                .flatten()
        }))
    }

    fn start_engine_service() -> Result<()> {
        let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
            .wrap_err("failed to connect to SCM")?;
        let service = manager
            .open_service(
                ENGINE_SERVICE_NAME,
                ServiceAccess::START | ServiceAccess::QUERY_STATUS,
            )
            .wrap_err("failed to open AceEngine for start")?;
        match service.start(&[] as &[&std::ffi::OsStr]) {
            Ok(()) => Ok(()),
            Err(error) => {
                let status = service
                    .query_status()
                    .wrap_err("engine start failed and status re-check failed")?;
                if matches!(status.current_state, ServiceState::Running | ServiceState::StartPending) {
                    Ok(())
                } else {
                    Err(error).wrap_err("StartService(AceEngine) failed")
                }
            }
        }
    }

    fn terminate_verified_engine_process(expected_process_id: u32) -> Result<()> {
        let status = query_engine_service()?
            .ok_or_else(|| eyre!("AceEngine service disappeared before termination"))?;
        if status.state != ServiceState::Running || status.process_id != Some(expected_process_id) {
            return Err(eyre!(
                "AceEngine SCM PID changed before termination; refusing to target PID {expected_process_id}"
            ));
        }

        let process = unsafe {
            // SAFETY: OpenProcess validates the PID and returns an owned kernel
            // handle; only PROCESS_TERMINATE is requested.
            OpenProcess(PROCESS_TERMINATE, false, expected_process_id)
        }
        .wrap_err("failed to open verified AceEngine process for termination")?;
        let terminate_result = unsafe {
            // SAFETY: `process` is the live handle returned by OpenProcess above.
            TerminateProcess(process, 1)
        };
        let close_result = unsafe {
            // SAFETY: `process` is no longer used after this point.
            CloseHandle(process)
        };
        terminate_result.wrap_err("TerminateProcess(AceEngine) failed")?;
        close_result.wrap_err("CloseHandle(AceEngine) failed")?;
        Ok(())
    }

    fn snapshot_health(shared_health: &Arc<Mutex<HeartbeatSnapshot>>) -> HeartbeatSnapshot {
        lock_health(shared_health).clone()
    }

    fn set_connected_engine(
        shared_health: &Arc<Mutex<HeartbeatSnapshot>>,
        process_id: u32,
        instance_id: &EngineInstanceId,
    ) {
        let mut health = lock_health(shared_health);
        health.client_process_id = Some(process_id);
        health.engine_instance_id = Some(*instance_id);
        health.last_success = None;
        health.consecutive_failures = 0;
    }

    fn clear_connection(shared_health: &Arc<Mutex<HeartbeatSnapshot>>, process_id: u32) {
        let mut health = lock_health(shared_health);
        if health.client_process_id == Some(process_id) {
            health.client_process_id = None;
            health.engine_instance_id = None;
            health.last_success = None;
            health.consecutive_failures = 0;
        }
    }

    fn note_heartbeat_failure(shared_health: &Arc<Mutex<HeartbeatSnapshot>>) {
        let mut health = lock_health(shared_health);
        health.consecutive_failures = health.consecutive_failures.saturating_add(1);
        if health.consecutive_failures == 1 {
            log_event("engine heartbeat failure detected; waiting for threshold");
        }
    }

    fn note_heartbeat_success(
        shared_health: &Arc<Mutex<HeartbeatSnapshot>>,
        process_id: u32,
        instance_id: &EngineInstanceId,
    ) {
        let mut health = lock_health(shared_health);
        if health.consecutive_failures > 0 {
            log_event("engine heartbeat recovered");
        }
        health.client_process_id = Some(process_id);
        health.engine_instance_id = Some(*instance_id);
        health.last_success = Some(Instant::now());
        health.consecutive_failures = 0;
    }

    fn expire_maintenance_if_needed(
        shared_health: &Arc<Mutex<HeartbeatSnapshot>>,
        now: Instant,
    ) {
        let mut health = lock_health(shared_health);
        if health.maintenance_until.is_some_and(|until| now >= until) {
            health.maintenance_until = None;
            log_event("maintenance window expired; recovery resumed");
        }
    }

    fn lock_health(
        shared_health: &Arc<Mutex<HeartbeatSnapshot>>,
    ) -> MutexGuard<'_, HeartbeatSnapshot> {
        match shared_health.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    async fn write_watchdog_message<W: AsyncWrite + Unpin>(
        writer: &mut W,
        message: &WatchdogToEngine,
    ) -> Result<()> {
        let mut encoded = serde_json::to_vec(message).wrap_err("failed to encode watchdog protocol message")?;
        encoded.push(b'\n');
        writer
            .write_all(&encoded)
            .await
            .wrap_err("failed writing watchdog pipe")?;
        writer.flush().await.wrap_err("failed flushing watchdog pipe")?;
        Ok(())
    }

    async fn wait_for_stop(stop_requested: Arc<AtomicBool>) {
        while !stop_requested.load(Ordering::Acquire) {
            time::sleep(Duration::from_millis(250)).await;
        }
    }

    pub fn log_event(message: &str) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        let line = format!("[{timestamp}] {message}\n");
        let path = watchdog_log_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let write_result = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut file| file.write_all(line.as_bytes()));
        if write_result.is_err() {
            eprint!("{line}");
        }
    }

    fn watchdog_log_path() -> PathBuf {
        let base = std::env::var_os("ProgramData")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        base.join("LineaSoftware")
            .join("AceBlocker")
            .join("logs")
            .join("ace-watchdog.log")
    }
}
