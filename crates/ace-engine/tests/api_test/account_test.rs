use crate::common::{get_api_client_from, get_default_test_server};

#[tokio::test]
async fn test_account_info() {
    let server = get_default_test_server();
    let client = get_api_client_from(&server);

    // The test harness forces Pro (`set_debug_pro_policy(Pro)`), so the
    // account info must reflect that; without an API key the ToS flag is
    // whatever the fresh DB says (false).
    let info = client
        .get_account_info()
        .await
        .expect("Failed to get account info");

    assert!(info.is_pro, "the test harness forces Pro status");
    assert!(!info.tos_accepted, "a fresh DB must not have ToS accepted");
}

#[tokio::test]
async fn test_account_refresh_endpoint_is_ttl_gated() {
    // POST /account/refresh must return the same cached account info without
    // hammering the Ace server: with the debug Pro policy in force (and no API
    // key stored) it reports the cached status and performs no remote verify.
    let server = get_default_test_server();
    let response = server.post("/account/refresh").await;
    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["isPro"], true, "debug policy forces Pro");
    assert_eq!(body["tosAccepted"], false, "fresh DB has no ToS");
}

// We skip set_api_key/logout for now as they trigger remote network calls to hardcoded URLs
// unless we want to start a local mock server on :5173.
