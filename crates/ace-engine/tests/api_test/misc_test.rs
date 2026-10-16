use crate::common::{get_api_client_from, get_default_test_server};

#[tokio::test]
async fn test_ping() {
    let server = get_default_test_server();
    let res = server.get("/ping").await;
    res.assert_status_ok();
    res.assert_text("pong");
}

#[tokio::test]
async fn test_dev_routes() {
    let server = get_default_test_server();
    let client = get_api_client_from(&server);

    // The test harness forces Pro, so the dev policy must report it. (The
    // route returns a JSON-encoded enum, i.e. the text includes the quotes.)
    let policy = client
        .get_debug_pro_policy()
        .await
        .expect("Failed to get debug pro policy");
    assert_eq!(
        policy.trim_matches('"'),
        "Pro",
        "the test harness forces the Pro debug policy"
    );

    // Default resistance level is the neutral "Default" sentinel.
    let level = client
        .get_debug_resistance_level()
        .await
        .expect("Failed to get debug resistance level");
    assert_eq!(
        level.trim_matches('"'),
        "Default",
        "the default debug resistance level is Default"
    );
}

#[tokio::test]
async fn test_system_shutdown_refused_while_network_block_locked() {
    // Spec §5 + service contract: the engine refuses to stop while a lock is
    // active (it shuts down only via POST /system/shutdown, and even that is
    // refused while locked — otherwise restarting/shutting down the engine
    // would defeat the lock).
    let server = get_default_test_server();

    let req = crate::api_client::CreateNetworkBlockRequest {
        block_name: "Shutdown Test Block".to_string(),
        icon: None,
    };
    let res = server.post("/network-blocks").json(&req).await;
    let net_list_id = res
        .json::<crate::api_client::CreateNetworkBlockResponse>()
        .network_block
        .id;

    // Lock the block for the next hour.
    let req = crate::api_client::SetNetworkLockRequest {
        lock_state: "active_until_lock_end".to_string(),
        lock_end: Some(crate::common::get_later_date_time()),
    };
    server
        .post(&format!("/network-blocks/{net_list_id}/lock"))
        .json(&req)
        .await
        .assert_status_ok();

    let res = server.post("/system/shutdown").await;
    res.assert_status(http::StatusCode::LOCKED);
    let json: serde_json::Value = res.json();
    assert_eq!(json["code"], "locked");
}

#[tokio::test]
async fn test_system_shutdown_refused_while_resistance_mode_locked() {
    let server = get_default_test_server();
    let client = get_api_client_from(&server);

    client
        .set_resistance_mode_enabled(crate::api_client::SetEnabledRequest { enabled: true })
        .await
        .expect("Failed to enable resistance mode");
    client
        .set_resistance_mode_lock(crate::api_client::SetNetworkLockRequest {
            lock_state: "active_until_lock_end".to_string(),
            lock_end: Some(crate::common::get_later_date_time()),
        })
        .await
        .expect("Failed to lock resistance mode");

    let res = server.post("/system/shutdown").await;
    res.assert_status(http::StatusCode::LOCKED);
    let json: serde_json::Value = res.json();
    assert_eq!(json["code"], "locked");
}

#[tokio::test]
async fn test_system_shutdown_succeeds_and_signals_graceful_stop() {
    // With no active locks the endpoint must accept the shutdown AND actually
    // signal the main run loop — otherwise it would be a no-op and the
    // service would never stop.
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_db.sqlite");
    let sites_dir = temp_dir.path().join("sites_lists");
    std::fs::create_dir_all(&sites_dir).unwrap();
    ace_engine::settings::Settings::set_test_sites_dir(sites_dir);
    let (server, mut shutdown_rx) = crate::common::get_test_server_with_shutdown_rx(&db_path);

    let res = server.post("/system/shutdown").await;
    res.assert_status_ok();

    shutdown_rx
        .changed()
        .await
        .expect("POST /system/shutdown must signal the shutdown channel");
    assert!(*shutdown_rx.borrow(), "shutdown flag must be true");
}
