use crate::common::{get_default_test_server, get_later_date_time};
use ace_engine::service::api::schemas::*;
use fake::Fake;
use fake::faker::name::en::Name;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_network_list_crud() {
    let test_server = get_default_test_server();

    // Create
    let name: String = Name().fake();
    let req = CreateNetworkBlockRequest {
        block_name: name.clone(),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    res.assert_status_ok();

    let body: CreateNetworkBlockResponse = res.json();
    let list_id = body.network_block.id;

    // Get All
    let res = test_server.get("/network-blocks").await;
    let body: GetAllNetworkBlocksResponse = res.json();

    assert!(
        body.network_blocks
            .iter()
            .any(|l| l.id == list_id && l.name == name)
    );

    // Delete
    let res = test_server
        .delete(&format!("/network-blocks/{}", list_id))
        .await;
    res.assert_status_ok();

    // Verify Deletion
    let res = test_server.get("/network-blocks").await;
    let body: GetAllNetworkBlocksResponse = res.json();

    assert!(!body.network_blocks.iter().any(|l| l.id == list_id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_network_block_enabled_api_path() {
    let test_server = get_default_test_server();

    let req = CreateNetworkBlockRequest {
        block_name: Name().fake(),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    res.assert_status_ok();

    let created: CreateNetworkBlockResponse = res.json();
    let block_id = created.network_block.id;
    assert!(created.network_block.enabled);
    assert!(
        !created.network_block.is_active,
        "an enabled block with an empty schedule must remain inactive"
    );

    test_server
        .post(&format!("/network-blocks/{block_id}/enabled"))
        .json(&SetEnabledRequest { enabled: false })
        .await
        .assert_status_ok();

    let res = test_server.get("/network-blocks").await;
    let blocks: GetAllNetworkBlocksResponse = res.json();
    let disabled = blocks
        .network_blocks
        .iter()
        .find(|block| block.id == block_id)
        .unwrap();
    assert!(!disabled.enabled);
    assert!(!disabled.is_active);

    test_server
        .post(&format!("/network-blocks/{block_id}/enabled"))
        .json(&SetEnabledRequest { enabled: true })
        .await
        .assert_status_ok();

    let res = test_server.get("/network-blocks").await;
    let blocks: GetAllNetworkBlocksResponse = res.json();
    let enabled = blocks
        .network_blocks
        .iter()
        .find(|block| block.id == block_id)
        .unwrap();
    assert!(enabled.enabled);
    assert!(
        !enabled.is_active,
        "enabling must not change empty-schedule semantics"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_network_list_locking() {
    let test_server = get_default_test_server();

    // Create
    let req = CreateNetworkBlockRequest {
        block_name: Name().fake(),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    let body: CreateNetworkBlockResponse = res.json();
    let list_id = body.network_block.id;

    let later_date_time = get_later_date_time();
    // Set Lock
    let req = SetNetworkLockRequest {
        lock_state: "active_until_lock_end".to_string(),
        lock_end: Some(later_date_time.clone()),
    };
    let res = test_server
        .post(&format!("/network-blocks/{}/lock", list_id))
        .json(&req)
        .await;
    res.assert_status_ok();

    // Get Lock
    let res = test_server
        .get(&format!("/network-blocks/{}/lock", list_id))
        .await;
    let body: GetNetworkLockResponse = res.json();
    let lock = body.lock;

    assert!(lock.is_active);
    match lock.lock_state.state.as_str() {
        "active_until_lock_end" => {}
        _ => panic!("Wrong lock type"),
    }

    assert_eq!(lock.lock_end, Some(later_date_time));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_soft_unlock_enforces_five_minute_cooldown() {
    let test_server = get_default_test_server();

    let req = CreateNetworkBlockRequest {
        block_name: Name().fake(),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    let body: CreateNetworkBlockResponse = res.json();
    let list_id = body.network_block.id;

    let req = SetNetworkLockRequest {
        lock_state: "soft_unlockable".to_string(),
        lock_end: None,
    };
    let res = test_server
        .post(&format!("/network-blocks/{}/lock", list_id))
        .json(&req)
        .await;
    res.assert_status_ok();

    let res = test_server
        .post(&format!("/network-blocks/{}/unlock/initiate", list_id))
        .await;
    res.assert_status_ok();
    let challenge: UnlockChallengeInitiateResponse = res.json();
    assert_eq!(challenge.cooldown_seconds, 5 * 60);

    let confirm = UnlockChallengeConfirmRequest {
        challenge_id: challenge.challenge_id,
    };
    let res = test_server
        .post(&format!("/network-blocks/{}/unlock/confirm", list_id))
        .json(&confirm)
        .await;
    res.assert_status(http::StatusCode::TOO_EARLY);
    let too_early: TooEarlyResponse = res.json();
    assert!(too_early.retry_after_seconds > 0);
    assert!(too_early.retry_after_seconds <= 5 * 60);

    // An early confirm must not weaken or consume the soft lock.
    let res = test_server
        .get(&format!("/network-blocks/{}/lock", list_id))
        .await;
    let body: GetNetworkLockResponse = res.json();
    assert_eq!(body.lock.lock_state.state, "soft_unlockable");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_network_list_sites_connection() {
    let test_server = get_default_test_server();

    // Create Net List
    let req = CreateNetworkBlockRequest {
        block_name: Name().fake(),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    let body: CreateNetworkBlockResponse = res.json();
    let net_list_id = body.network_block.id;

    // Create Sites List
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_string_lossy().to_string();

    let site_name: String = Name().fake();
    let req = CreateSitePackRequest {
        name: site_name.clone(),
        custom_folder: None,
        description: None,
        icon: None,
        sources: vec![BaseSitePackSourceSchema {
            source: file_path,
            source_type: SitePackSourceType::File,
            format: SitePackFormat::Standard,
        }],
    };
    let res = test_server.post("/site-lists").json(&req).await;

    // Wait for stream (just check text as stream is consumed)
    let s = res.text();
    if s.contains("FAILED") {
        panic!("Sites list creation failed: {}", s);
    }
    assert!(
        s.contains("COMPLETED"),
        "the import stream must report completion, got: {}",
        s
    );

    // Get Sites List ID
    let res = test_server.get("/site-lists").await;
    let body: GetAllSitePacksResponse = res.json();
    let sites_list = body
        .site_packs
        .iter()
        .find(|h| h.name == site_name)
        .unwrap();
    let sites_list_id = sites_list.id.clone();

    // Add Connection
    let req = AddNetListToSitePackRequest {
        sites_list_id: sites_list_id.clone(),
    };
    let res = test_server
        .post(&format!("/network-blocks/{}/site-lists", net_list_id))
        .json(&req)
        .await;
    res.assert_status_ok();

    // Get Connections
    let res = test_server
        .get(&format!("/network-blocks/{}/site-lists", net_list_id))
        .await;
    let body: GetNetListToSitePacksResponse = res.json();

    assert!(
        body.connections
            .iter()
            .any(|c| c.site_pack_id == sites_list_id)
    );

    // Remove Connection
    let res = test_server
        .delete(&format!(
            "/network-blocks/{}/site-lists/{}",
            net_list_id, sites_list_id
        ))
        .await;
    res.assert_status_ok();

    // Verify Removal
    let res = test_server
        .get(&format!("/network-blocks/{}/site-lists", net_list_id))
        .await;
    let body: GetNetListToSitePacksResponse = res.json();
    assert!(
        !body
            .connections
            .iter()
            .any(|c| c.site_pack_id == sites_list_id)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_network_list_bulk() {
    let test_server = get_default_test_server();

    // Create 20 network blocks
    for _ in 0..20 {
        let req = CreateNetworkBlockRequest {
            block_name: Name().fake(),
            icon: None,
        };
        test_server
            .post("/network-blocks")
            .json(&req)
            .await
            .assert_status_ok();
    }

    let res = test_server.get("/network-blocks").await;
    let body: GetAllNetworkBlocksResponse = res.json();
    assert!(body.network_blocks.len() >= 20);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_network_list_fuzz() {
    let test_server = get_default_test_server();

    let strange_names = vec![
        "List with spaces".to_string(),
        "Special chars: !@#$%^&*()".to_string(),
        "💩 Emoji List".to_string(),
        "A".repeat(255),
        "   ".to_string(), // Just spaces
    ];

    for name in strange_names {
        let req = CreateNetworkBlockRequest {
            block_name: name.clone(),
            icon: None,
        };
        let res = test_server.post("/network-blocks").json(&req).await;
        res.assert_status_ok();

        let list_id = res.json::<CreateNetworkBlockResponse>().network_block.id;

        // Verify it was created with the correct name
        let res = test_server.get("/network-blocks").await;
        let body: GetAllNetworkBlocksResponse = res.json();
        assert!(
            body.network_blocks
                .iter()
                .any(|b| b.id == list_id && b.name == name)
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_network_block_lock_constraints() {
    let test_server = get_default_test_server();

    // 1. Create Network Block
    let req = CreateNetworkBlockRequest {
        block_name: Name().fake(),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    let net_list_id = res.json::<CreateNetworkBlockResponse>().network_block.id;

    // 2. Add a site list connection (so we can try to remove it under lock)
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_string_lossy().to_string();
    let site_name = Name().fake::<String>();
    let req = CreateSitePackRequest {
        name: site_name.clone(),
        custom_folder: None,
        description: None,
        icon: None,
        sources: vec![BaseSitePackSourceSchema {
            source: file_path,
            source_type: SitePackSourceType::File,
            format: SitePackFormat::Standard,
        }],
    };
    let _ = test_server.post("/site-lists").json(&req).await;

    let res = test_server.get("/site-lists").await;
    let body: GetAllSitePacksResponse = res.json();
    let sites_list_id = body
        .site_packs
        .iter()
        .find(|s| s.name == site_name)
        .unwrap()
        .id
        .clone();

    let req = AddNetListToSitePackRequest {
        sites_list_id: sites_list_id.clone(),
    };
    test_server
        .post(&format!("/network-blocks/{}/site-lists", net_list_id))
        .json(&req)
        .await
        .assert_status_ok();

    // 3. Lock the block (within the 90-day cap)
    let far_future = (chrono::Local::now() + chrono::Duration::days(30))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();

    let req = SetNetworkLockRequest {
        lock_state: "active_until_lock_end".to_string(),
        lock_end: Some(far_future),
    };
    test_server
        .post(&format!("/network-blocks/{}/lock", net_list_id))
        .json(&req)
        .await
        .assert_status_ok();

    // 4. Deleting the network block should FAIL
    let res = test_server
        .delete(&format!("/network-blocks/{}", net_list_id))
        .await;
    res.assert_status(http::StatusCode::FORBIDDEN);

    // 5. Removing connection to site list should FAIL
    let res = test_server
        .delete(&format!(
            "/network-blocks/{}/site-lists/{}",
            net_list_id, sites_list_id
        ))
        .await;
    res.assert_status(http::StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_lock_persists_across_restart() {
    // Spec §3.3/§5: a time-lock has a defined end time and is designed to
    // survive relapse — restarting the engine must not defeat it. Rebuild
    // the whole server (fresh AppState + NetworkManager) from the same DB
    // file to simulate a restart, then verify the lock is still active,
    // still carries the same end time, and is still enforced.
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_db.sqlite");
    let sites_dir = temp_dir.path().join("sites_lists");
    std::fs::create_dir_all(&sites_dir).unwrap();
    ace_engine::settings::Settings::set_test_sites_dir(sites_dir);

    // First "boot": create a block and lock it until a defined end time.
    let lock_end = get_later_date_time();
    let net_list_id = {
        let test_server = crate::common::get_test_server_from_db_path(&db_path);

        let req = CreateNetworkBlockRequest {
            block_name: Name().fake(),
            icon: None,
        };
        let res = test_server.post("/network-blocks").json(&req).await;
        let net_list_id = res.json::<CreateNetworkBlockResponse>().network_block.id;

        let req = SetNetworkLockRequest {
            lock_state: "active_until_lock_end".to_string(),
            lock_end: Some(lock_end.clone()),
        };
        test_server
            .post(&format!("/network-blocks/{net_list_id}/lock"))
            .json(&req)
            .await
            .assert_status_ok();

        // Sanity: the lock is active before the restart.
        let res = test_server
            .get(&format!("/network-blocks/{net_list_id}/lock"))
            .await;
        let body: GetNetworkLockResponse = res.json();
        assert!(body.lock.is_active);
        assert_eq!(body.lock.lock_end, Some(lock_end.clone()));

        net_list_id
    }; // test_server dropped here — simulated engine restart

    // Second "boot" from the same DB: the lock must have survived.
    let test_server = crate::common::get_test_server_from_db_path(&db_path);

    let res = test_server
        .get(&format!("/network-blocks/{net_list_id}/lock"))
        .await;
    let body: GetNetworkLockResponse = res.json();
    assert!(body.lock.is_active, "the lock must survive a restart");
    assert_eq!(
        body.lock.lock_end,
        Some(lock_end),
        "the lock's end time must be preserved across a restart"
    );

    // Enforcement must survive too: the block cannot be deleted while locked.
    let res = test_server
        .delete(&format!("/network-blocks/{net_list_id}"))
        .await;
    res.assert_status(http::StatusCode::FORBIDDEN);

    // And the lock cannot be downgraded or removed after the restart.
    let req = SetNetworkLockRequest {
        lock_state: "inactive".to_string(),
        lock_end: None,
    };
    let res = test_server
        .post(&format!("/network-blocks/{net_list_id}/lock"))
        .json(&req)
        .await;
    res.assert_status(http::StatusCode::FORBIDDEN);
}
