use crate::common::get_default_test_server;
use ace_engine::service::api::schemas::*;
use fake::Fake;
use fake::faker::internet::en::IPv4;
use fake::faker::lorem::en::Word;
use fake::faker::name::en::Name;
use std::io::Write;

fn generate_hosts_list_line() -> String {
    let ip: String = IPv4().fake();
    let hostname = format!("{}.test", Word().fake::<String>().to_lowercase());
    format!("{} {}", ip, hostname)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sites_list_crud_file() {
    let test_server = get_default_test_server();

    // Create a large sites file
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_string_lossy().to_string();

    let mut file = std::fs::File::create(temp_file.path()).unwrap();
    for _ in 0..100 {
        writeln!(file, "{}", generate_hosts_list_line()).unwrap();
    }

    let list_name: String = Name().fake();

    // Create from Files
    let request = CreateSitePackRequest {
        name: list_name.clone(),
        description: Some("Test description".to_string()),
        custom_folder: None,
        icon: None,
        sources: vec![BaseSitePackSourceSchema {
            source: file_path.clone(),
            source_type: SitePackSourceType::File,
            format: SitePackFormat::Standard,
        }],
    };

    let response = test_server.post("/site-lists").json(&request).await;
    response.assert_status_ok();

    // Verify it exists
    let response = test_server.get("/site-lists").await;
    let body: GetAllSitePacksResponse = response.json();

    let sites_list = body
        .site_packs
        .iter()
        .find(|l| l.name == list_name)
        .expect("Sites list not found");
    let list_id = sites_list.id.clone();

    // Delete
    let res = test_server
        .delete(&format!("/site-lists/{}", list_id))
        .await;
    res.assert_status_ok();

    // Verify Deletion
    let res = test_server.get("/site-lists").await;
    let body: GetAllSitePacksResponse = res.json();
    assert!(!body.site_packs.iter().any(|l| l.id == list_id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sites_list_sources_management() {
    let test_server = get_default_test_server();

    // 1. Create Sites List with one source
    let temp_file1 = tempfile::NamedTempFile::new().unwrap();
    let file_path1 = temp_file1.path().to_string_lossy().to_string();
    std::fs::write(temp_file1.path(), "0.0.0.0 initial.com").unwrap();

    let list_name: String = Name().fake();
    let req = CreateSitePackRequest {
        name: list_name.clone(),
        description: None,
        custom_folder: None,
        icon: None,
        sources: vec![BaseSitePackSourceSchema {
            source: file_path1,
            source_type: SitePackSourceType::File,
            format: SitePackFormat::Standard,
        }],
    };
    let res = test_server.post("/site-lists").json(&req).await;
    res.assert_status_ok();

    // Get List ID
    let res = test_server.get("/site-lists").await;
    let body: GetAllSitePacksResponse = res.json();
    let list_id = body
        .site_packs
        .iter()
        .find(|l| l.name == list_name)
        .unwrap()
        .id
        .clone();

    // 2. Get Sources
    let res = test_server
        .get(&format!("/site-lists/{}/sources", list_id))
        .await;
    let body: GetSitePackSourcesResponse = res.json();
    assert_eq!(body.sources.len(), 1);
    let first_source_id = body.sources[0].id.clone();

    // 3. Add Another Source
    let temp_file2 = tempfile::NamedTempFile::new().unwrap();
    let file_path2 = temp_file2.path().to_string_lossy().to_string();
    std::fs::write(temp_file2.path(), "0.0.0.0 added.com").unwrap();

    let add_req = AddSitePackSourceRequest {
        source: file_path2,
        source_type: SitePackSourceType::File,
        format: SitePackFormat::Standard,
    };
    test_server
        .post(&format!("/site-lists/{}/sources", list_id))
        .json(&add_req)
        .await
        .assert_status_ok();

    // Verify both exist
    let res = test_server
        .get(&format!("/site-lists/{}/sources", list_id))
        .await;
    let body: GetSitePackSourcesResponse = res.json();
    assert_eq!(body.sources.len(), 2);

    // 4. Delete First Source
    test_server
        .delete(&format!("/site-lists/sources/{}", first_source_id))
        .await
        .assert_status_ok();

    // Verify only one left
    let res = test_server
        .get(&format!("/site-lists/{}/sources", list_id))
        .await;
    let body: GetSitePackSourcesResponse = res.json();
    assert_eq!(body.sources.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sites_list_resync() {
    let test_server = get_default_test_server();

    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_string_lossy().to_string();
    std::fs::write(temp_file.path(), "0.0.0.0 sync.com").unwrap();

    let list_name: String = Name().fake();
    let req = CreateSitePackRequest {
        name: list_name.clone(),
        description: None,
        custom_folder: None,
        icon: None,
        sources: vec![BaseSitePackSourceSchema {
            source: file_path,
            source_type: SitePackSourceType::File,
            format: SitePackFormat::Standard,
        }],
    };
    test_server
        .post("/site-lists")
        .json(&req)
        .await
        .assert_status_ok();

    let res = test_server.get("/site-lists").await;
    let list_id = res
        .json::<GetAllSitePacksResponse>()
        .site_packs
        .iter()
        .find(|l| l.name == list_name)
        .unwrap()
        .id
        .clone();

    // Trigger Resync
    test_server
        .post(&format!("/site-lists/{}/resync", list_id))
        .await
        .assert_status_ok();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sites_list_url_import() {
    let test_server = get_default_test_server();

    // Start a simple file server using Axum
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server_url = format!("http://127.0.0.1:{}/hosts.txt", port);

    tokio::spawn(async move {
        let app = axum::Router::new().route(
            "/hosts.txt",
            axum::routing::get(|| async {
                axum::response::Response::builder()
                    .header(axum::http::header::CONTENT_LENGTH, "18")
                    .body(axum::body::Body::from("0.0.0.0 mocked.com"))
                    .unwrap()
            }),
        );
        axum::serve(listener, app).await.unwrap();
    });

    let list_name: String = Name().fake();
    // Create from URL (Local Server)
    let req = CreateSitePackRequest {
        name: list_name.clone(),
        description: None,
        custom_folder: None,
        icon: None,
        sources: vec![BaseSitePackSourceSchema {
            source: server_url.clone(),
            source_type: SitePackSourceType::Url,
            format: SitePackFormat::Standard,
        }],
    };

    let res = test_server.post("/site-lists").json(&req).await;
    res.assert_status_ok();

    // Verify created (wait for sync)
    let mut success = false;
    for _ in 0..30 {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let res = test_server.get("/site-lists").await;
        let body: GetAllSitePacksResponse = res.json();
        if body
            .site_packs
            .iter()
            .any(|l| l.name == list_name && l.is_synced)
        {
            success = true;
            break;
        }
    }
    assert!(
        success,
        "Sites list {} was not created or synced in time",
        list_name
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sites_list_delete_in_use_conflicts_and_locked_pack_is_refused() {
    let test_server = get_default_test_server();

    // Create a synced local pack + a network block, then attach the pack.
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_string_lossy().to_string();
    std::fs::write(temp_file.path(), "0.0.0.0 sync.com").unwrap();

    let list_name: String = Name().fake();
    let req = CreateSitePackRequest {
        name: list_name.clone(),
        description: None,
        custom_folder: None,
        icon: None,
        sources: vec![BaseSitePackSourceSchema {
            source: file_path,
            source_type: SitePackSourceType::File,
            format: SitePackFormat::Standard,
        }],
    };
    test_server
        .post("/site-lists")
        .json(&req)
        .await
        .assert_status_ok();
    let res = test_server.get("/site-lists").await;
    let pack_id = res
        .json::<GetAllSitePacksResponse>()
        .site_packs
        .iter()
        .find(|l| l.name == list_name)
        .unwrap()
        .id
        .clone();

    let block_req = CreateNetworkBlockRequest {
        block_name: Name().fake(),
        icon: None,
    };
    let block_res = test_server.post("/network-blocks").json(&block_req).await;
    let block_id = block_res
        .json::<CreateNetworkBlockResponse>()
        .network_block
        .id;

    test_server
        .post(&format!("/network-blocks/{block_id}/site-lists"))
        .json(&AddNetListToSitePackRequest {
            sites_list_id: pack_id.clone(),
        })
        .await
        .assert_status_ok();

    // 1. In use by an unlocked block → 409 conflict, code "conflict".
    let res = test_server.delete(&format!("/site-lists/{pack_id}")).await;
    assert_eq!(res.status_code(), http::StatusCode::CONFLICT);
    let body: serde_json::Value = res.json();
    assert_eq!(body["code"], "conflict");

    // 2. Lock the block, then the pack is locked → 423, code "locked".
    let far_future = (chrono::Local::now() + chrono::Duration::days(30))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
    let lock_req = SetNetworkLockRequest {
        lock_state: "active_until_lock_end".to_string(),
        lock_end: Some(far_future),
    };
    test_server
        .post(&format!("/network-blocks/{block_id}/lock"))
        .json(&lock_req)
        .await
        .assert_status_ok();

    let res = test_server.delete(&format!("/site-lists/{pack_id}")).await;
    assert_eq!(res.status_code(), http::StatusCode::LOCKED);
    let body: serde_json::Value = res.json();
    assert_eq!(body["code"], "locked");

    // 3. Resync of a locked pack is also refused.
    let res = test_server
        .post(&format!("/site-lists/{pack_id}/resync"))
        .await;
    assert_eq!(res.status_code(), http::StatusCode::LOCKED);
    let body: serde_json::Value = res.json();
    assert_eq!(body["code"], "locked");

    // 4. The listing exposes the lock state.
    let res = test_server.get("/site-lists").await;
    let body: GetAllSitePacksResponse = res.json();
    let pack = body.site_packs.iter().find(|l| l.id == pack_id).unwrap();
    assert!(pack.is_locked, "a locked pack must be reported as locked");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sites_list_fuzz() {
    let test_server = get_default_test_server();

    // A source file that does not exist: the import stream must report a
    // FAILED phase instead of hanging, crashing, or claiming success.
    let req = CreateSitePackRequest {
        name: Name().fake(),
        description: None,
        custom_folder: None,
        icon: None,
        sources: vec![BaseSitePackSourceSchema {
            source: "non_existent_file.txt".to_string(),
            source_type: SitePackSourceType::File,
            format: SitePackFormat::Standard,
        }],
    };

    let res = test_server.post("/site-lists").json(&req).await;
    let stream_text = res.text();
    assert!(
        stream_text.contains("FAILED"),
        "a missing-file source must report a failed import, got: {stream_text}"
    );
}
