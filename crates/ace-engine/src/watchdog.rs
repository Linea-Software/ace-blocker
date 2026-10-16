//! Engine-side link to the production userspace watchdog service.
//!
//! The production watchdog is owned by the platform service manager (SCM on
//! Windows, systemd on Linux). This module never spawns a privileged watchdog
//! process directly; it connects to the watchdog IPC endpoint and asks the
//! service manager to restore the watchdog service when that endpoint disappears.

use crate::watchdog_protocol::MaintenanceReason;
use eyre::Result;
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
use std::time::Duration;

#[cfg(any(target_os = "windows", target_os = "linux"))]
mod imp {
    use super::*;
    #[cfg(target_os = "windows")]
    use crate::watchdog_protocol::WATCHDOG_PIPE_NAME;
    use crate::watchdog_protocol::{EngineInstanceId, EngineToWatchdog, WatchdogToEngine};
    #[cfg(all(target_os = "linux", debug_assertions))]
    use crate::watchdog_protocol::LINUX_WATCHDOG_PAUSE_PATH;
    #[cfg(target_os = "linux")]
    use crate::watchdog_protocol::LINUX_WATCHDOG_SOCKET_PATH;
    use eyre::{Context, eyre};
    use std::sync::OnceLock;
    use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
    #[cfg(target_os = "linux")]
    use tokio::net::UnixStream;
    #[cfg(target_os = "windows")]
    use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
    use tokio::sync::{mpsc, oneshot};
    use tokio::time::{self, Duration};
    use tracing::{info, warn};
    use uuid::Uuid;

    #[cfg(target_os = "linux")]
    type WatchdogStream = UnixStream;
    #[cfg(target_os = "windows")]
    type WatchdogStream = NamedPipeClient;

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
        ClientOptions::new()
            .open(WATCHDOG_PIPE_NAME)
            .wrap_err("failed to open watchdog named pipe")
    }

    #[cfg(target_os = "linux")]
    async fn connect_watchdog() -> Result<WatchdogStream> {
        UnixStream::connect(LINUX_WATCHDOG_SOCKET_PATH)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to connect to watchdog socket {}",
                    LINUX_WATCHDOG_SOCKET_PATH
                )
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
        let (read_half, mut write_half) = tokio::io::split(client);
        let mut lines = BufReader::new(read_half).lines();
        let mut pending_maintenance: Option<oneshot::Sender<Result<()>>> = None;

        write_message(
            &mut write_half,
            &EngineToWatchdog::Hello {
                engine_instance_id: *engine_instance_id,
            },
        )
        .await?;

        loop {
            tokio::select! {
                line = lines.next_line() => {
                    let Some(line) = line.wrap_err("failed reading watchdog IPC")? else {
                        if let Some(response) = pending_maintenance.take() {
                            let _ = response.send(Err(eyre!("watchdog disconnected during maintenance request")));
                        }
                        return Err(eyre!("watchdog closed the IPC channel"));
                    };

                    let message: WatchdogToEngine = match serde_json::from_str(&line) {
                        Ok(message) => message,
                        Err(error) => {
                            warn!("Ignoring malformed watchdog protocol message: {error}");
                            continue;
                        }
                    };
                    handle_watchdog_message(
                        message,
                        engine_instance_id,
                        &mut write_half,
                        &mut pending_maintenance,
                    ).await?;
                }
                command = command_rx.recv() => {
                    let Some(command) = command else {
                        return Err(eyre!("engine watchdog command channel closed"));
                    };
                    if pending_maintenance.is_some() {
                        let _ = command.response.send(Err(eyre!("a watchdog maintenance request is already pending")));
                        continue;
                    }
                    let ttl_ms = u64::try_from(command.ttl.as_millis()).unwrap_or(u64::MAX);
                    write_message(
                        &mut write_half,
                        &EngineToWatchdog::BeginMaintenance {
                            engine_instance_id: *engine_instance_id,
                            ttl_ms,
                            reason: command.reason,
                        },
                    ).await?;
                    pending_maintenance = Some(command.response);
                }
            }
        }
    }

    async fn handle_watchdog_message<W: AsyncWrite + Unpin>(
        message: WatchdogToEngine,
        engine_instance_id: &EngineInstanceId,
        writer: &mut W,
        pending_maintenance: &mut Option<oneshot::Sender<Result<()>>>,
    ) -> Result<()> {
        match message {
            WatchdogToEngine::Ping { sequence, nonce } => {
                if dev_heartbeat_failure_enabled() {
                    return Ok(());
                }
                write_message(
                    writer,
                    &EngineToWatchdog::Pong {
                        sequence,
                        nonce,
                        engine_instance_id: *engine_instance_id,
                    },
                )
                .await?;
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

    async fn write_message<W: AsyncWrite + Unpin>(
        writer: &mut W,
        message: &EngineToWatchdog,
    ) -> Result<()> {
        let mut encoded =
            serde_json::to_vec(message).wrap_err("failed to encode watchdog message")?;
        encoded.push(b'\n');
        writer
            .write_all(&encoded)
            .await
            .wrap_err("failed writing watchdog IPC")?;
        writer
            .flush()
            .await
            .wrap_err("failed flushing watchdog IPC")?;
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
