#[cfg(feature = "experimental-ace-guard")]
pub mod ace_guard;
pub mod core;
pub mod db;
pub mod lock_check;
pub mod manager;
pub mod platform;
pub mod protocol;
pub mod schema_main_db;
pub mod schema_site_packs;
pub mod service;
pub mod settings;
pub mod watchdog;
pub mod watchdog_protocol;

pub mod ace_server_api;

use eyre::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::RwLock;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::db::{DBError, EngineDB};
use crate::manager::{AccountManager, NetworkManager};
use crate::service::proxy;
use crate::settings::Settings;
use tracing_appender::non_blocking::WorkerGuard;

pub async fn run() -> Result<()> {
    let _log_guard = setup_logging();
    tracing::info!("Tracing initialized. Starting proxy setup...");

    // Process protection (Windows only). The installed production path uses a
    // separate SCM-owned AceEngineWatchdog service. The historical same-binary
    // guardian remains only for release console runs and is never the service
    // watchdog.
    #[cfg(target_os = "windows")]
    {
        if let Err(error) = crate::platform::hide_important_files() {
            tracing::warn!("Failed to hide important files: {error}");
        }
        #[cfg(not(debug_assertions))]
        if !crate::platform::in_service_mode() {
            if let Err(error) = crate::platform::spawn_watchdog() {
                tracing::warn!("Console guardian spawn failed: {error}");
            }
        }
    }

    // The service-mode engine keeps a lightweight heartbeat connection to the
    // production watchdog. Loss of that connection makes the engine ask the
    // platform service manager to restore the watchdog; it never spawns a
    // privileged orphan process directly.
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    let _watchdog_link_handle = crate::platform::in_service_mode()
        .then(crate::watchdog::spawn_engine_watchdog_task)
        .flatten();

    let db = initialize_db().await;

    // Import the installer's bundled site packs (offline seed data) before the
    // network manager is constructed, so the manager loads rows that already
    // have usable rules — no "ghost" packs on a fresh offline install. The
    // pass is idempotent, fails soft (a missing/corrupt bundle is logged and
    // skipped), and performs no network I/O.
    let bundled_report = crate::service::bundled_site_packs::apply_bundled_packs(&db);
    for (id, action) in &bundled_report.actions {
        tracing::info!("Bundled site pack {}: {:?}", id, action);
    }
    if !bundled_report.rejected().is_empty() {
        tracing::warn!(
            "Bundled site packs rejected: {:?}",
            bundled_report.rejected()
        );
    }

    let account_manager = AccountManager::from(db.clone())
        .map_err(|error| eyre::eyre!("failed to initialize AccountManager: {}", error))?;
    let (network_manager, content_update_service) =
        initialize_network_manager(db.clone(), account_manager.clone()).await;

    let update_check_interval: crate::service::content_updater::SharedUpdateInterval = Arc::new(
        RwLock::new(crate::service::content_updater::DEFAULT_UPDATE_CHECK_INTERVAL),
    );

    // Create and trust both local CAs before any background task, listener, or
    // tunnel starts. Firefox on Linux may need an enterprise certificate policy
    // in addition to the OS trust store, so the policy must exist before the
    // first intercepted TLS handshake.
    crate::service::proxy::prepare_certificates()?;

    // Shutdown channel: authorized remoc RPC requests send on this; run() awaits it.
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);

    // Spawn the background content-update loop only in release builds. Debug
    // builds intentionally avoid updater network I/O; the pending future keeps
    // the service select structure identical without running the downloader.
    #[cfg(not(debug_assertions))]
    let background_update_handle =
        tokio::spawn(crate::service::content_updater::run_background_update_loop(
            content_update_service.clone(),
            network_manager.clone(),
            update_check_interval.clone(),
            account_manager.clone(),
        ));
    #[cfg(debug_assertions)]
    let background_update_handle = std::future::pending::<()>();

    // Periodic account/pro refresh (Issue 4): while an API key is stored the
    // engine re-verifies entitlement on its own cadence (single-flight, cache
    // preserved on transient errors), so subscription changes recorded on the
    // Ace server surface without waiting for the 24h lazy TTL. No requests are
    // made when logged out.
    let account_refresh_handle = tokio::spawn(
        crate::manager::account::run_periodic_account_refresh(account_manager.clone()),
    );

    let proxy_handle = spawn_proxy_service(network_manager.clone()).await;
    let api_handle = spawn_api_service(
        network_manager.clone(),
        account_manager,
        content_update_service,
        update_check_interval,
        shutdown_tx,
    )
    .await;

    // Wait for all services to complete.
    tokio::select! {
        _ = proxy_handle => {
            tracing::info!("Proxy service task finished.");
        },
        _ = api_handle => {
            tracing::info!("API service task finished.");
        },
        _ = background_update_handle => {
            tracing::info!("Background update task finished.");
        },
        _ = account_refresh_handle => {
            tracing::info!("Account refresh task finished.");
        },
        _ = signal::ctrl_c() => {
            tracing::info!("Received Ctrl+C (SIGINT). Shutting down services.");
        },
        Ok(_) = shutdown_rx.wait_for(|v| *v) => {
            tracing::info!("Shutdown requested via API. Shutting down services.");
        },
    }

    tracing::info!("Shutting down.");
    Ok(())
}

fn setup_logging() -> WorkerGuard {
    let filter_directives = "info,ace_proxy=trace,ace_tun=debug,mitm_proxy=debug";
    let filter = EnvFilter::new(filter_directives);

    // Setup logging to file. A failure to create the log file (error.g. a stale
    // file owned by a previous SYSTEM service run, or a locked-down account)
    // must NEVER prevent the engine from starting — fall back to stdout-only
    // logging instead of panicking.
    let log_dir = Settings::get_site_packs_dir()
        .parent()
        .expect("Failed to get app data root")
        .join("logs");

    let file_appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("ace-engine.log")
        .build(&log_dir);

    match file_appender {
        Ok(file_appender) => {
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

            tracing_subscriber::registry()
                .with(filter)
                .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(non_blocking)
                        .with_ansi(true),
                )
                .init();

            guard
        }
        Err(error) => {
            eprintln!(
                "WARNING: failed to create log file in {:?} ({}); logging to stdout only",
                log_dir, error
            );
            let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(non_blocking)
                        .with_ansi(true),
                )
                .init();
            guard
        }
    }
}

async fn initialize_db() -> Arc<EngineDB> {
    let db_path = Settings::get_main_db_path();
    let mut db_retries = 0;
    const MAX_RETRIES: u32 = 3;

    loop {
        match EngineDB::from(&db_path) {
            Ok(db) => {
                if db_retries > 0 {
                    tracing::info!(
                        "Database connection established after {} retries.",
                        db_retries
                    );
                }
                break Arc::new(db);
            }
            Err(error) => {
                db_retries += 1;
                tracing::warn!(
                    "Failed to open database (attempt {}/{}): {}",
                    db_retries,
                    MAX_RETRIES,
                    error
                );

                if db_retries >= MAX_RETRIES {
                    match error {
                        DBError::DatabaseConnectionError { .. }
                        | DBError::MigrationError { .. } => {
                            tracing::warn!(
                                "Database seems corrupted or inaccessible. Attempting recovery by invalidating old DB..."
                            );
                            let mut backup_path = db_path.clone();
                            if let Some(ext) = db_path.extension() {
                                let mut ext = ext.to_os_string();
                                ext.push(".bak");
                                backup_path.set_extension(ext);
                            } else {
                                backup_path.set_extension("bak");
                            }

                            if let Err(io_err) = std::fs::rename(&db_path, &backup_path) {
                                tracing::error!(
                                    "Failed to rename corrupted DB to {:?}: {}. Trying to delete...",
                                    backup_path,
                                    io_err
                                );
                                if let Err(del_err) = std::fs::remove_file(&db_path) {
                                    tracing::error!("Failed to delete corrupted DB: {}", del_err);
                                    std::process::exit(1);
                                }
                            } else {
                                tracing::info!("Corrupted database backed up to {:?}", backup_path);
                            }

                            if let Ok(new_db) = EngineDB::from(&db_path) {
                                tracing::info!(
                                    "Successfully created fresh database after recovery."
                                );
                                break Arc::new(new_db);
                            } else {
                                tracing::error!("Recovery failed. Exiting.");
                                std::process::exit(1);
                            }
                        }
                        _ => {
                            tracing::error!("Fatal database error: {}. Exiting.", error);
                            std::process::exit(1);
                        }
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }
}

async fn initialize_network_manager(
    db: Arc<EngineDB>,
    account_manager: AccountManager,
) -> (
    Arc<RwLock<NetworkManager>>,
    crate::service::content_updater::SharedContentUpdateService,
) {
    let network_manager = match NetworkManager::from(db.clone(), account_manager.clone()) {
        Ok(m) => m,
        Err(error) => {
            tracing::error!(
                "CRITICAL: NetworkManager failed to load: {}. Starting with EMPTY POLICY under the approved availability policy; persisted blocks are NOT enforced until recovered.",
                error
            );
            NetworkManager::new_empty(db.clone(), account_manager)
        }
    };

    // Initialize content updates (site lists, resistance scripts). The first
    // pass of the background update loop (spawned in `run()`) does the actual
    // fetching — never synchronously here, so a slow or unreachable server
    // cannot delay engine startup (SVC-01). Construction itself cannot fail
    // (LIF-03): the persisted "last check" timestamp is best-effort.
    let content_service = crate::service::content_updater::ContentUpdateService::from(db.clone());
    let shared_service: crate::service::content_updater::SharedContentUpdateService =
        Arc::new(tokio::sync::Mutex::new(content_service));

    let network_manager = Arc::new(RwLock::new(network_manager));
    (network_manager, shared_service)
}

async fn spawn_proxy_service(
    network_manager: Arc<RwLock<NetworkManager>>,
) -> tokio::task::JoinHandle<()> {
    let listen_port: u16 =
        openport::pick_random_unused_port().expect("No free port for the proxy. Aborting");
    // No HTTP/3 listener: ace-tun drops QUIC (UDP 443) at the tunnel so clients
    // fall back to TCP, which the proxy inspects reliably. MITM-ing QUIC is
    // fragile — browsers may reject a locally-rooted cert over HTTP/3 — so we
    // force TCP instead. `TunRedirectBuilder::quic_policy` can revisit this.
    let proxy = match proxy::AceProxy::new(network_manager.clone(), listen_port).await {
        Ok(p) => p,
        Err(error) => {
            tracing::error!("Failed to setup proxy service: {}. Exiting.", error);
            std::process::exit(1);
        }
    };

    tokio::spawn(async move {
        tracing::info!("Starting proxy service on port {}...", listen_port);
        if let Err(error) = proxy.start().await {
            tracing::error!("Proxy service failed: {:?}", error);
        }
        tracing::info!("Proxy service stopped.");
    })
}

async fn spawn_api_service(
    network_manager: Arc<RwLock<NetworkManager>>,
    account_manager: AccountManager,
    content_update_service: crate::service::content_updater::SharedContentUpdateService,
    update_check_interval: crate::service::content_updater::SharedUpdateInterval,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
) -> tokio::task::JoinHandle<()> {
    let state = service::api::AppState {
        network_manager,
        account_manager,
        content_update_service,
        update_check_interval,
        shutdown_tx,
        unlock_challenges: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        capability_token: Some(Settings::load_or_create_capability_token()),
        enforce_capability: cfg!(not(debug_assertions)),
    };
    tokio::spawn(async move {
        if let Err(error) = service::api::transport::serve(state).await {
            tracing::error!("Engine IPC server failed: {error:#}");
        }
    })
}
