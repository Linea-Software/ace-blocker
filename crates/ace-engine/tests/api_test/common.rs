#![allow(dead_code)]
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use ace_engine::{core::constants, db::EngineDB, manager::NetworkManager};
use axum_test::TestServer;

use crate::api_client::HttpClient;

#[cfg(test)]
pub fn get_api_client_from(server: &axum_test::TestServer) -> HttpClient {
    let mut url = server.server_url("/").unwrap().to_string();
    if url.ends_with('/') {
        url.pop();
    }
    HttpClient::new().with_base_url(url)
}

#[cfg(test)]
fn create_test_app_state_and_shutdown_rx(
    db_path: &Path,
    _is_pro: bool,
) -> (
    ace_engine::service::api::AppState,
    tokio::sync::watch::Receiver<bool>,
) {
    let db = Arc::new(EngineDB::from(db_path).expect("Failed to open or create main database"));

    let account_manager = ace_engine::manager::AccountManager::from(db.clone()).unwrap();
    let mut network_manager_sync =
        NetworkManager::from(db.clone(), account_manager.clone()).unwrap();

    // Always enable Pro for tests to allow all features
    #[cfg(debug_assertions)]
    network_manager_sync.set_debug_pro_policy(ace_engine::manager::network::DebugProPolicy::Pro);

    let network_manager = Arc::new(RwLock::new(network_manager_sync));

    let content_update_service = Arc::new(tokio::sync::Mutex::new(
        ace_engine::service::content_updater::ContentUpdateService::from(db.clone()),
    ));
    let update_check_interval = Arc::new(RwLock::new(
        ace_engine::service::content_updater::DEFAULT_UPDATE_CHECK_INTERVAL,
    ));
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    (
        ace_engine::service::api::AppState {
            network_manager,
            account_manager,
            content_update_service,
            update_check_interval,
            shutdown_tx,
            unlock_challenges: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            // Integration tests: no capability enforcement (matches the dev/debug
            // convention). Security tests that need enforcement build their own
            // AppState with a token (see the `service::api::security` unit tests).
            capability_token: None,
            enforce_capability: false,
        },
        shutdown_rx,
    )
}

#[cfg(test)]
fn get_test_api_handle_from(db_path: &Path, is_pro: bool) -> axum_test::TestServer {
    let (state, _shutdown_rx) = create_test_app_state_and_shutdown_rx(db_path, is_pro);
    let router = ace_engine::service::api::create_router(state);

    axum_test::TestServer::builder()
        .transport(axum_test::Transport::HttpRandomPort)
        .build(router)
}

/// One process-wide temp dir for all test site packs.
///
/// Tests run in parallel and `Settings::set_test_sites_dir` is
/// process-global, so a per-test dir would race (TESTINFRA-01: one test's
/// server would resolve the global to another test's dir). Sharing a single
/// writable dir is safe because pack directories are UUID-named, and the
/// dir is leaked for the process lifetime like the old per-test tempdirs.
fn shared_test_sites_dir() -> PathBuf {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let dir = tempfile::tempdir().expect("failed to create shared test sites dir");
        let path = dir.path().to_path_buf();
        Box::leak(Box::new(dir)); // keep alive for the whole process
        path
    })
    .clone()
}

/// Process-wide shared empty resistance-scripts dir (same rationale as
/// [`shared_test_sites_dir`]; tests that need script fixtures override it
/// explicitly).
fn shared_test_resistance_dir() -> PathBuf {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let dir = tempfile::tempdir().expect("failed to create shared test resistance dir");
        let path = dir.path().to_path_buf();
        Box::leak(Box::new(dir));
        path
    })
    .clone()
}

/// Serializes mutations of the process-global resistance-scripts dir
/// (TESTINFRA-01): `get_default_test_server` sets the shared empty dir, and
/// fixture tests set their own dir before building a server whose manager
/// loads the script tree synchronously from it. Holding this lock across
/// set+build makes those loads deterministic.
pub static RESISTANCE_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub fn get_default_test_server() -> TestServer {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_db.sqlite");

    // All test servers share one process-wide sites dir (see
    // `shared_test_sites_dir`); point the resistance-scripts dir at a shared
    // empty dir so tests are deterministic: a developer machine may have real
    // downloaded scripts in the ProgramData app-data dir, which would
    // otherwise leak into every test server's script tree (and every test
    // that asserts on script counts or enforcement status).
    let sites_dir = shared_test_sites_dir();
    ace_engine::settings::Settings::set_test_sites_dir(sites_dir);

    let _resistance_lock = RESISTANCE_DIR_LOCK.lock().unwrap();
    let resistance_dir = shared_test_resistance_dir();
    ace_engine::settings::Settings::set_test_resistance_scripts_dir(resistance_dir);
    drop(_resistance_lock);

    let server = get_test_api_handle_from(&db_path, true);

    // We need to keep the temp_dir alive. axum-test TestServer doesn't seem to have a way to
    // attach arbitrary data. But since we are in a test, let's just leak it for now to avoid premature deletion.
    // In a real scenario we'd use a better way.
    Box::leak(Box::new(temp_dir));

    server
}

/// Builds a fresh test server from an existing DB file — simulates a restart
/// (settings, script states, etc. are reloaded from disk). The caller is
/// responsible for keeping the DB's directory alive.
#[cfg(test)]
pub fn get_test_server_from_db_path(db_path: &Path) -> TestServer {
    get_test_api_handle_from(db_path, true)
}

/// Like [`get_test_server_from_db_path`], but also returns the engine's
/// shutdown-channel receiver so tests can assert that `POST /system/shutdown`
/// actually signals the graceful-shutdown path.
#[cfg(test)]
pub fn get_test_server_with_shutdown_rx(
    db_path: &Path,
) -> (TestServer, tokio::sync::watch::Receiver<bool>) {
    let (state, shutdown_rx) = create_test_app_state_and_shutdown_rx(db_path, true);
    let router = ace_engine::service::api::create_router(state);

    let server = axum_test::TestServer::builder()
        .transport(axum_test::Transport::HttpRandomPort)
        .build(router);
    (server, shutdown_rx)
}

#[cfg(test)]
pub fn get_later_date_time() -> String {
    let now = chrono::Local::now();
    let later = now + chrono::Duration::hours(1);
    later.format(constants::DATETIME_FORMAT).to_string()
}

#[cfg(test)]
pub fn get_later_time() -> String {
    let now = chrono::Local::now();
    let later = now + chrono::Duration::hours(1);
    later.time().format(constants::TIME_FORMAT).to_string()
}
