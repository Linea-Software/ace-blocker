use crate::watchdog_protocol::{
    ENGINE_STARTUP_GRACE, EngineInstanceId, EngineToWatchdog, HEARTBEAT_FAILURE_THRESHOLD,
    HEARTBEAT_INTERVAL, HEARTBEAT_TIMEOUT, LINUX_ENGINE_SERVICE_NAME, LINUX_WATCHDOG_RUNTIME_DIR,
    LINUX_WATCHDOG_SOCKET_PATH, WatchdogNonce, WatchdogToEngine, clamp_maintenance_ttl,
    recovery_delay,
};
use eyre::{Context, Result, bail, eyre};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::Command;
use tokio::signal::unix::{SignalKind, signal};
use tokio::time;
use uuid::Uuid;

const MONITOR_INTERVAL: Duration = Duration::from_secs(2);
const SYSTEMD_RECOVERY_GRACE: Duration = Duration::from_secs(3);
const STABLE_RESET_INTERVAL: Duration = Duration::from_secs(5 * 60);
const SOCKET_HELLO_TIMEOUT: Duration = Duration::from_secs(5);
const SYSTEMCTL_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WatchdogState {
    Starting,
    Healthy,
    Unhealthy,
    Recovering,
    Maintenance,
    Backoff,
}

#[derive(Clone, Debug, Default)]
struct HeartbeatSnapshot {
    client_process_id: Option<u32>,
    engine_instance_id: Option<EngineInstanceId>,
    last_success: Option<Instant>,
    consecutive_failures: u32,
    maintenance_until: Option<Instant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EngineServiceState {
    Running,
    Starting,
    Stopping,
    Stopped,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EngineServiceStatus {
    state: EngineServiceState,
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
            state: WatchdogState::Starting,
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
        if self.state != next {
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

struct SocketPathGuard {
    path: PathBuf,
}

impl Drop for SocketPathGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub async fn run() -> Result<()> {
    let stop_requested = Arc::new(AtomicBool::new(false));
    install_signal_task(Arc::clone(&stop_requested))?;

    log_event("watchdog started");
    let outcome = run_watchdog(Arc::clone(&stop_requested)).await;
    stop_requested.store(true, Ordering::Release);

    match &outcome {
        Ok(()) => log_event("watchdog stopped"),
        Err(error) => log_event(&format!("watchdog failed: {error:#}")),
    }
    outcome
}

fn install_signal_task(stop_requested: Arc<AtomicBool>) -> Result<()> {
    let mut terminate = signal(SignalKind::terminate())
        .wrap_err("failed to register SIGTERM handler for watchdog")?;
    let mut interrupt = signal(SignalKind::interrupt())
        .wrap_err("failed to register SIGINT handler for watchdog")?;

    tokio::spawn(async move {
        tokio::select! {
            _ = terminate.recv() => {}
            _ = interrupt.recv() => {}
        }
        stop_requested.store(true, Ordering::Release);
    });
    Ok(())
}

async fn run_watchdog(stop_requested: Arc<AtomicBool>) -> Result<()> {
    let shared_health = Arc::new(Mutex::new(HeartbeatSnapshot::default()));
    tokio::try_join!(
        monitor_engine(Arc::clone(&shared_health), Arc::clone(&stop_requested)),
        run_socket_server(shared_health, stop_requested),
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
        if snapshot.maintenance_until.is_some_and(|until| until > now) {
            context.transition(WatchdogState::Maintenance);
            continue;
        }

        let service_status = match query_engine_service().await {
            Ok(status) => status,
            Err(error) => {
                context.recovery_failures = context.recovery_failures.saturating_add(1);
                context.next_recovery_at = now + recovery_delay(context.recovery_failures);
                log_event(&format!("failed to query ace-engine.service: {error:#}"));
                context.transition(WatchdogState::Backoff);
                continue;
            }
        };

        let Some(service_status) = service_status else {
            log_event("ace-engine.service registration is missing");
            context.next_recovery_at = now + Duration::from_secs(60);
            context.transition(WatchdogState::Backoff);
            continue;
        };

        match service_status.state {
            EngineServiceState::Stopped | EngineServiceState::Failed => {
                handle_stopped_engine(&mut context, now).await?;
            }
            EngineServiceState::Starting | EngineServiceState::Stopping => {
                context.transition(WatchdogState::Starting);
            }
            EngineServiceState::Running => {
                let Some(process_id) = service_status.process_id else {
                    context.transition(WatchdogState::Starting);
                    continue;
                };
                handle_running_engine(&mut context, &snapshot, process_id, now).await?;
            }
        }
    }
}

async fn handle_stopped_engine(context: &mut MonitorContext, now: Instant) -> Result<()> {
    context.observed_process_id = None;
    context.stable_since = None;
    if context.stopped_since.is_none() {
        context.stopped_since = Some(now);
        context.next_recovery_at = now + SYSTEMD_RECOVERY_GRACE;
        log_event("engine unexpectedly stopped; allowing systemd recovery grace period");
    }

    if now < context.next_recovery_at {
        context.transition(WatchdogState::Backoff);
        return Ok(());
    }

    context.transition(WatchdogState::Recovering);
    log_event("engine restart requested through systemd");
    match start_engine_service().await {
        Ok(()) => log_event("engine restart request accepted by systemd"),
        Err(error) => log_event(&format!("engine restart request failed: {error:#}")),
    }
    context.note_recovery_attempt(now);
    Ok(())
}

async fn handle_running_engine(
    context: &mut MonitorContext,
    snapshot: &HeartbeatSnapshot,
    process_id: u32,
    now: Instant,
) -> Result<()> {
    context.stopped_since = None;
    context.observe_process(process_id, now);

    if now.duration_since(context.process_observed_at) < ENGINE_STARTUP_GRACE {
        context.transition(WatchdogState::Starting);
        return Ok(());
    }

    let heartbeat_is_fresh = snapshot.client_process_id == Some(process_id)
        && snapshot.engine_instance_id.is_some()
        && snapshot.consecutive_failures < HEARTBEAT_FAILURE_THRESHOLD
        && snapshot.last_success.is_some_and(|last_success| {
            now.duration_since(last_success)
                <= HEARTBEAT_INTERVAL + HEARTBEAT_TIMEOUT + Duration::from_secs(1)
        });

    if heartbeat_is_fresh {
        context.disconnected_failures = 0;
        context.note_stable(now);
        context.transition(WatchdogState::Healthy);
        return Ok(());
    }

    let failures = if snapshot.client_process_id == Some(process_id) {
        snapshot.consecutive_failures
    } else {
        context.disconnected_failures = context.disconnected_failures.saturating_add(1);
        context.disconnected_failures
    };
    context.transition(WatchdogState::Unhealthy);

    if failures < HEARTBEAT_FAILURE_THRESHOLD {
        return Ok(());
    }
    if now < context.next_recovery_at {
        context.transition(WatchdogState::Backoff);
        return Ok(());
    }

    log_event(&format!(
        "engine declared hung after {failures} heartbeat failures; verifying systemd MainPID {process_id}"
    ));
    terminate_verified_engine_process(process_id).await?;
    log_event(&format!(
        "verified engine PID {process_id} terminated; systemd recovery will run"
    ));
    context.note_recovery_attempt(now);
    context.next_recovery_at =
        now + SYSTEMD_RECOVERY_GRACE.max(recovery_delay(context.recovery_failures));
    context.transition(WatchdogState::Recovering);
    Ok(())
}

async fn run_socket_server(
    shared_health: Arc<Mutex<HeartbeatSnapshot>>,
    stop_requested: Arc<AtomicBool>,
) -> Result<()> {
    let listener = bind_watchdog_socket().await?;
    let _socket_guard = SocketPathGuard {
        path: PathBuf::from(LINUX_WATCHDOG_SOCKET_PATH),
    };

    loop {
        if stop_requested.load(Ordering::Acquire) {
            return Ok(());
        }

        let (stream, _) = tokio::select! {
            accepted = listener.accept() => accepted.wrap_err("failed accepting watchdog Unix socket client")?,
            _ = wait_for_stop(Arc::clone(&stop_requested)) => return Ok(()),
        };

        let client_process_id = unix_peer_process_id(&stream)?;
        if current_engine_process_id().await? != Some(client_process_id) {
            log_event(&format!(
                "rejected watchdog socket client PID {client_process_id}: not current systemd ace-engine.service MainPID"
            ));
            continue;
        }

        if let Err(error) = serve_engine_connection(
            stream,
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

async fn bind_watchdog_socket() -> Result<UnixListener> {
    let runtime_dir = Path::new(LINUX_WATCHDOG_RUNTIME_DIR);
    std::fs::create_dir_all(runtime_dir)
        .wrap_err_with(|| format!("failed to create {}", runtime_dir.display()))?;
    std::fs::set_permissions(runtime_dir, std::fs::Permissions::from_mode(0o700))
        .wrap_err_with(|| format!("failed to secure {}", runtime_dir.display()))?;

    let socket_path = Path::new(LINUX_WATCHDOG_SOCKET_PATH);
    if socket_path.exists() {
        if UnixStream::connect(socket_path).await.is_ok() {
            bail!(
                "watchdog socket {} is already accepting connections",
                socket_path.display()
            );
        }
        std::fs::remove_file(socket_path)
            .wrap_err_with(|| format!("failed to remove stale {}", socket_path.display()))?;
    }

    let listener = UnixListener::bind(socket_path)
        .wrap_err_with(|| format!("failed to bind {}", socket_path.display()))?;
    std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o600))
        .wrap_err_with(|| format!("failed to secure {}", socket_path.display()))?;
    Ok(listener)
}

fn unix_peer_process_id(stream: &UnixStream) -> Result<u32> {
    let credentials = stream
        .peer_cred()
        .wrap_err("failed to read watchdog Unix peer credentials")?;
    let process_id = credentials
        .pid()
        .ok_or_else(|| eyre!("watchdog Unix peer did not expose a process ID"))?;
    u32::try_from(process_id).wrap_err("watchdog Unix peer process ID was negative")
}

async fn serve_engine_connection(
    stream: UnixStream,
    client_process_id: u32,
    shared_health: &Arc<Mutex<HeartbeatSnapshot>>,
    stop_requested: Arc<AtomicBool>,
) -> Result<()> {
    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut lines = BufReader::new(read_half).lines();

    let hello_line = time::timeout(SOCKET_HELLO_TIMEOUT, lines.next_line())
        .await
        .wrap_err("timed out waiting for engine watchdog hello")?
        .wrap_err("failed reading engine watchdog hello")?
        .ok_or_else(|| eyre!("engine closed watchdog socket before hello"))?;
    let hello: EngineToWatchdog =
        serde_json::from_str(&hello_line).wrap_err("invalid engine watchdog hello message")?;
    let engine_instance_id = match hello {
        EngineToWatchdog::Hello { engine_instance_id } => engine_instance_id,
        _ => return Err(eyre!("first watchdog socket message was not hello")),
    };

    if current_engine_process_id().await? != Some(client_process_id) {
        return Err(eyre!(
            "ace-engine.service MainPID changed during watchdog handshake"
        ));
    }
    set_connected_engine(shared_health, client_process_id, &engine_instance_id);
    log_event(&format!(
        "engine heartbeat connected: systemd MainPID {client_process_id}, instance {engine_instance_id}"
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
                }

                sequence = sequence.wrapping_add(1);
                let nonce = WatchdogNonce::from(Uuid::new_v4());
                write_watchdog_message(
                    &mut write_half,
                    &WatchdogToEngine::Ping { sequence, nonce },
                ).await?;
                pending_ping = Some((sequence, nonce, Instant::now()));
            }
            line = lines.next_line() => {
                let Some(line) = line.wrap_err("failed reading engine heartbeat socket")? else {
                    return Err(eyre!("engine closed heartbeat socket"));
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
                return Err(eyre!(
                    "engine instance identity changed on an existing watchdog socket"
                ));
            }
        }
        EngineToWatchdog::Pong {
            sequence,
            nonce,
            engine_instance_id,
        } => {
            let matches_challenge =
                pending_ping
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
                || current_engine_process_id().await? != Some(client_process_id)
            {
                write_watchdog_message(
                    writer,
                    &WatchdogToEngine::MaintenanceRejected {
                        reason: "engine identity no longer matches systemd service instance"
                            .to_string(),
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

async fn query_engine_service() -> Result<Option<EngineServiceStatus>> {
    let output = run_systemctl(&[
        "show",
        "--no-page",
        "--property=LoadState",
        "--property=ActiveState",
        "--property=SubState",
        "--property=MainPID",
        LINUX_ENGINE_SERVICE_NAME,
    ])
    .await?;
    if !output.status.success() {
        bail!(
            "systemctl show {} failed: {}",
            LINUX_ENGINE_SERVICE_NAME,
            command_error_text(&output)
        );
    }
    parse_engine_service_status(&String::from_utf8_lossy(&output.stdout))
}

fn parse_engine_service_status(text: &str) -> Result<Option<EngineServiceStatus>> {
    let mut load_state = None;
    let mut active_state = None;
    let mut sub_state = None;
    let mut main_pid = None;

    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "LoadState" => load_state = Some(value),
            "ActiveState" => active_state = Some(value),
            "SubState" => sub_state = Some(value),
            "MainPID" => main_pid = Some(value),
            _ => {}
        }
    }

    match load_state {
        Some("not-found") => return Ok(None),
        Some("loaded") => {}
        Some(other) => bail!("unexpected ace-engine.service LoadState {other:?}"),
        None => bail!("systemctl did not return LoadState for ace-engine.service"),
    }

    let process_id = match main_pid {
        Some("0") | None => None,
        Some(value) => Some(
            value
                .parse::<u32>()
                .wrap_err_with(|| format!("invalid ace-engine.service MainPID {value:?}"))?,
        ),
    };

    let state = match active_state {
        Some("active") if process_id.is_some() => EngineServiceState::Running,
        Some("active" | "activating" | "reloading" | "refreshing") => EngineServiceState::Starting,
        Some("deactivating") => EngineServiceState::Stopping,
        Some("inactive") => EngineServiceState::Stopped,
        Some("failed") => EngineServiceState::Failed,
        Some(other) => bail!(
            "unexpected ace-engine.service ActiveState {other:?} (SubState {:?})",
            sub_state.unwrap_or_default()
        ),
        None => bail!("systemctl did not return ActiveState for ace-engine.service"),
    };

    Ok(Some(EngineServiceStatus { state, process_id }))
}

async fn current_engine_process_id() -> Result<Option<u32>> {
    Ok(query_engine_service().await?.and_then(|status| {
        (status.state == EngineServiceState::Running)
            .then_some(status.process_id)
            .flatten()
    }))
}

async fn start_engine_service() -> Result<()> {
    let output = run_systemctl(&["start", LINUX_ENGINE_SERVICE_NAME]).await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(eyre!(
            "systemctl start {} failed: {}",
            LINUX_ENGINE_SERVICE_NAME,
            command_error_text(&output)
        ))
    }
}

async fn terminate_verified_engine_process(expected_process_id: u32) -> Result<()> {
    let status = query_engine_service()
        .await?
        .ok_or_else(|| eyre!("ace-engine.service disappeared before termination"))?;
    if status.state != EngineServiceState::Running || status.process_id != Some(expected_process_id)
    {
        return Err(eyre!(
            "ace-engine.service MainPID changed before termination; refusing to target PID {expected_process_id}"
        ));
    }

    let process_id = i32::try_from(expected_process_id)
        .wrap_err("ace-engine.service MainPID does not fit a Linux pid_t")?;
    let result = unsafe {
        // SAFETY: process_id was read from systemd immediately above and is
        // revalidated against the service before this exact PID is signalled.
        libc_kill(process_id, SIGKILL)
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).wrap_err_with(|| {
            format!("failed to SIGKILL verified engine PID {expected_process_id}")
        })
    }
}

const SIGKILL: i32 = 9;

unsafe extern "C" {
    #[link_name = "kill"]
    fn libc_kill(pid: i32, signal: i32) -> i32;
}

async fn run_systemctl(args: &[&str]) -> Result<std::process::Output> {
    let mut command = Command::new("systemctl");
    command.args(args);
    time::timeout(SYSTEMCTL_TIMEOUT, command.output())
        .await
        .wrap_err_with(|| format!("timed out executing systemctl {}", args.join(" ")))?
        .wrap_err_with(|| format!("failed to execute systemctl {}", args.join(" ")))
}

fn command_error_text(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        format!("systemctl exited with {}", output.status)
    } else {
        stderr
    }
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

fn expire_maintenance_if_needed(shared_health: &Arc<Mutex<HeartbeatSnapshot>>, now: Instant) {
    let mut health = lock_health(shared_health);
    if health.maintenance_until.is_some_and(|until| now >= until) {
        health.maintenance_until = None;
        log_event("maintenance window expired; recovery resumed");
    }
}

fn lock_health(shared_health: &Arc<Mutex<HeartbeatSnapshot>>) -> MutexGuard<'_, HeartbeatSnapshot> {
    match shared_health.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

async fn write_watchdog_message<W: AsyncWrite + Unpin>(
    writer: &mut W,
    message: &WatchdogToEngine,
) -> Result<()> {
    let mut encoded =
        serde_json::to_vec(message).wrap_err("failed to encode watchdog protocol message")?;
    encoded.push(b'\n');
    writer
        .write_all(&encoded)
        .await
        .wrap_err("failed writing watchdog socket")?;
    writer
        .flush()
        .await
        .wrap_err("failed flushing watchdog socket")?;
    Ok(())
}

async fn wait_for_stop(stop_requested: Arc<AtomicBool>) {
    while !stop_requested.load(Ordering::Acquire) {
        time::sleep(Duration::from_millis(250)).await;
    }
}

fn log_event(message: &str) {
    eprintln!("[ace-watchdog] {message}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_running_systemd_service_status() {
        let status = parse_engine_service_status(
            "MainPID=4242\nLoadState=loaded\nActiveState=active\nSubState=running\n",
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            status,
            EngineServiceStatus {
                state: EngineServiceState::Running,
                process_id: Some(4242),
            }
        );
    }

    #[test]
    fn parse_missing_systemd_service_returns_none() {
        let status = parse_engine_service_status(
            "MainPID=0\nLoadState=not-found\nActiveState=inactive\nSubState=dead\n",
        )
        .unwrap();

        assert_eq!(status, None);
    }

    #[test]
    fn parse_failed_systemd_service_status() {
        let status = parse_engine_service_status(
            "LoadState=loaded\nActiveState=failed\nSubState=failed\nMainPID=0\n",
        )
        .unwrap()
        .unwrap();

        assert_eq!(status.state, EngineServiceState::Failed);
    }

    #[tokio::test]
    async fn unix_peer_credentials_report_connecting_process() {
        let directory = tempfile::tempdir().unwrap();
        let socket_path = directory.path().join("watchdog.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        let client = UnixStream::connect(&socket_path).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();

        assert_eq!(unix_peer_process_id(&server).unwrap(), std::process::id());
        drop(client);
    }
}
