//! Engine-side authenticated connection to the separate watchdog service.
//!
//! The watchdog remains SCM/systemd-managed and verifies the engine process at
//! the OS transport boundary. Remoc replaces the bespoke newline-JSON framing;
//! the heartbeat challenge and the guarded maintenance lease are unchanged.

use crate::watchdog_protocol::MaintenanceReason;
use eyre::Result;
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
use std::time::Duration;

#[cfg(any(target_os = "windows", target_os = "linux"))]
mod imp {
    use super::*;
    #[cfg(all(target_os = "linux", debug_assertions))]
    use crate::watchdog_protocol::LINUX_WATCHDOG_PAUSE_PATH;
    #[cfg(target_os = "linux")]
    use crate::watchdog_protocol::LINUX_WATCHDOG_SOCKET_PATH;
    #[cfg(target_os = "windows")]
    use crate::watchdog_protocol::WATCHDOG_PIPE_NAME;
    use crate::watchdog_protocol::{EngineInstanceId, EngineToWatchdog, WatchdogToEngine};
    use eyre::{Context, eyre};
    #[cfg(target_os = "windows")]
    use interprocess::os::windows::named_pipe::{pipe_mode, tokio::DuplexPipeStream};
    use remoc::rch::base::{Receiver, Sender};
    use std::sync::OnceLock;
    #[cfg(target_os = "linux")]
    use tokio::net::UnixStream;
    use tokio::sync::{mpsc, oneshot};
    use tokio::time::{self, Duration};
    use tracing::{info, warn};
    use uuid::Uuid;

    #[cfg(target_os = "linux")]
    type WatchdogStream = UnixStream;
    #[cfg(target_os = "windows")]
    type WatchdogStream = DuplexPipeStream<pipe_mode::Bytes>;

    static MAINTENANCE_CHANNEL: OnceLock<mpsc::Sender<MaintenanceCommand>> = OnceLock::new();

    struct MaintenanceCommand {
        reason: MaintenanceReason,
        ttl: Duration,
        response: oneshot::Sender<Result<()>>,
    }

    pub fn spawn_engine_watchdog_task() -> Option<tokio::task::JoinHandle<()>> {
        let (command_tx, command_rx) = mpsc::channel(2);
        if MAINTENANCE_CHANNEL.set(command_tx).is_err() {
            warn!("Engine watchdog link was already initialized; not starting a duplicate task.");
            return None;
        }

        let engine_instance_id = EngineInstanceId::from(Uuid::new_v4());
        Some(tokio::spawn(async move {
            run_engine_link(engine_instance_id, command_rx).await;
        }))
    }

    pub async fn request_maintenance(reason: MaintenanceReason, ttl: Duration) -> Result<()> {
        let sender = MAINTENANCE_CHANNEL
            .get()
            .ok_or_else(|| eyre!("watchdog link is not initialized"))?
            .clone();
        let (response_tx, response_rx) = oneshot::channel();
        sender
            .send(MaintenanceCommand {
                reason,
                ttl,
                response: response_tx,
            })
            .await
            .wrap_err("watchdog maintenance channel is closed")?;

        time::timeout(Duration::from_secs(5), response_rx)
            .await
            .wrap_err("timed out waiting for watchdog maintenance acknowledgement")?
            .wrap_err("watchdog maintenance response channel was dropped")?
    }

    async fn run_engine_link(
        engine_instance_id: EngineInstanceId,
        mut command_rx: mpsc::Receiver<MaintenanceCommand>,
    ) {
        let mut reconnect_failures = 0_u32;
        loop {
            if let Err(error) = ensure_watchdog_service().await {
                warn!("Failed to ensure watchdog service is running: {error:#}");
            }

            match connect_watchdog().await {
                Ok(client) => {
                    reconnect_failures = 0;
                    info!("Connected to production watchdog health channel.");
                    if let Err(error) =
                        serve_connection(client, &engine_instance_id, &mut command_rx).await
                    {
                        warn!("Watchdog IPC connection ended: {error:#}");
                    }
                }
                Err(error) => {
                    fail_queued_maintenance(&mut command_rx, "watchdog IPC is unavailable");
                    reconnect_failures = reconnect_failures.saturating_add(1);
                    if reconnect_failures == 1 || reconnect_failures.is_multiple_of(10) {
                        warn!("Watchdog IPC unavailable; will retry: {error:#}");
                    }
                }
            }

            let delay = crate::watchdog_protocol::recovery_delay(reconnect_failures);
            tokio::select! {
                _ = time::sleep(delay) => {}
                command = command_rx.recv() => {
                    if let Some(command) = command {
                        let _ = command.response.send(Err(eyre!("watchdog is not connected")));
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    async fn connect_watchdog() -> Result<WatchdogStream> {
        WatchdogStream::connect_by_path(WATCHDOG_PIPE_NAME)
            .await
            .wrap_err("failed to open watchdog named pipe")
    }

    #[cfg(target_os = "linux")]
    async fn connect_watchdog() -> Result<WatchdogStream> {
        UnixStream::connect(LINUX_WATCHDOG_SOCKET_PATH)
            .await
            .wrap_err_with(|| {
                format!("failed to connect to watchdog socket {LINUX_WATCHDOG_SOCKET_PATH}")
            })
    }

    async fn ensure_watchdog_service() -> Result<()> {
        tokio::task::spawn_blocking(crate::platform::ensure_watchdog_service_running)
            .await
            .wrap_err("watchdog service-manager check task failed")??;
        Ok(())
    }

    fn fail_queued_maintenance(command_rx: &mut mpsc::Receiver<MaintenanceCommand>, reason: &str) {
        while let Ok(command) = command_rx.try_recv() {
            let _ = command.response.send(Err(eyre!(reason.to_string())));
        }
    }

    async fn serve_connection(
        client: WatchdogStream,
        engine_instance_id: &EngineInstanceId,
        command_rx: &mut mpsc::Receiver<MaintenanceCommand>,
    ) -> Result<()> {
        let (read_half, write_half) = tokio::io::split(client);
        // Both sides use the shared, typed protocol enums. Remoc supplies
        // serialization, multiplexing and framing; there is no private codec.
        let (connection, mut tx, mut rx): (
            _,
            Sender<EngineToWatchdog>,
            Receiver<WatchdogToEngine>,
        ) = remoc::Connect::io(remoc::Cfg::default(), read_half, write_half)
            .await
            .wrap_err("failed to establish watchdog remoc session")?;
        let connection_task = tokio::spawn(connection);

        tx.send(EngineToWatchdog::Hello {
            engine_instance_id: *engine_instance_id,
        })
        .await
        .wrap_err("failed to send watchdog hello")?;

        let mut pending_maintenance: Option<oneshot::Sender<Result<()>>> = None;
        let result = loop {
            tokio::select! {
                received = rx.recv() => {
                    match received {
                        Ok(Some(message)) => {
                            if let Err(error) = handle_watchdog_message(
                                message, engine_instance_id, &mut tx, &mut pending_maintenance
                            ).await {
                                break Err(error);
                            }
                        }
                        Ok(None) => break Err(eyre!("watchdog closed the IPC channel")),
                        Err(error) => break Err(eyre!("watchdog channel receive failed: {error}")),
                    }
                }
                command = command_rx.recv() => {
                    let Some(command) = command else {
                        break Err(eyre!("engine watchdog command channel closed"));
                    };
                    if pending_maintenance.is_some() {
                        let _ = command.response.send(Err(eyre!("a watchdog maintenance request is already pending")));
                        continue;
                    }
                    let ttl_ms = u64::try_from(command.ttl.as_millis()).unwrap_or(u64::MAX);
                    if let Err(error) = tx.send(EngineToWatchdog::BeginMaintenance {
                        engine_instance_id: *engine_instance_id,
                        ttl_ms,
                        reason: command.reason,
                    }).await {
                        let _ = command.response.send(Err(eyre!("watchdog maintenance send failed: {error}")));
                        break Err(eyre!("watchdog send failed: {error}"));
                    }
                    pending_maintenance = Some(command.response);
                }
            }
        };
        if let Some(response) = pending_maintenance.take() {
            let _ = response.send(Err(eyre!(
                "watchdog disconnected during maintenance request"
            )));
        }
        connection_task.abort();
        result
    }

    async fn handle_watchdog_message(
        message: WatchdogToEngine,
        engine_instance_id: &EngineInstanceId,
        writer: &mut Sender<EngineToWatchdog>,
        pending_maintenance: &mut Option<oneshot::Sender<Result<()>>>,
    ) -> Result<()> {
        match message {
            WatchdogToEngine::Ping { sequence, nonce } => {
                if dev_heartbeat_failure_enabled() {
                    return Ok(());
                }
                writer
                    .send(EngineToWatchdog::Pong {
                        sequence,
                        nonce,
                        engine_instance_id: *engine_instance_id,
                    })
                    .await
                    .wrap_err("failed to send watchdog pong")?;
            }
            WatchdogToEngine::MaintenanceAccepted { ttl_ms } => {
                info!("Watchdog maintenance window accepted for {ttl_ms} ms.");
                if let Some(response) = pending_maintenance.take() {
                    let _ = response.send(Ok(()));
                }
            }
            WatchdogToEngine::MaintenanceRejected { reason } => {
                warn!("Watchdog rejected maintenance request: {reason}");
                if let Some(response) = pending_maintenance.take() {
                    let _ = response.send(Err(eyre!(reason)));
                }
            }
        }
        Ok(())
    }

    fn dev_heartbeat_failure_enabled() -> bool {
        #[cfg(debug_assertions)]
        {
            let environment_enabled = std::env::var_os("ACE_WATCHDOG_DEV_DROP_HEARTBEAT")
                .is_some_and(|value| value.to_string_lossy() != "0");

            #[cfg(target_os = "windows")]
            let pause_file_enabled = std::env::var_os("ProgramData")
                .map(std::path::PathBuf::from)
                .map(|path| {
                    path.join("LineaSoftware")
                        .join("AceBlocker")
                        .join("watchdog-heartbeat.pause")
                })
                .is_some_and(|path| path.exists());

            #[cfg(target_os = "linux")]
            let pause_file_enabled = std::path::Path::new(LINUX_WATCHDOG_PAUSE_PATH).exists();

            environment_enabled || pause_file_enabled
        }
        #[cfg(not(debug_assertions))]
        {
            false
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub use imp::{request_maintenance, spawn_engine_watchdog_task};

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn spawn_engine_watchdog_task() -> Option<tokio::task::JoinHandle<()>> {
    None
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub async fn request_maintenance(_reason: MaintenanceReason, _ttl: Duration) -> Result<()> {
    Ok(())
}
