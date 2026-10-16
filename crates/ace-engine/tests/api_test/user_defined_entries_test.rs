use crate::common::get_default_test_server;
use ace_engine::service::api::schemas::*;
use fake::Fake;
use fake::faker::lorem::en::Word;
use fake::faker::name::en::Name;

fn domain_rule(rule: String) -> UserDefinedRule {
    UserDefinedRule::new(rule, UserDefinedRuleType::Domain)
}

fn regex_rule(rule: String) -> UserDefinedRule {
    UserDefinedRule::new(rule, UserDefinedRuleType::Regex)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_user_defined_entries_crud() {
    let test_server = get_default_test_server();

    let block_name: String = Name().fake();
    let domain1 = format!("{}.com", Word().fake::<String>().to_lowercase());
    let domain2 = format!("{}.org", Word().fake::<String>().to_lowercase());

    // Create Network List
    let req = CreateNetworkBlockRequest {
        block_name: block_name.clone(),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    let body: CreateNetworkBlockResponse = res.json();
    let net_list_id = body.network_block.id;

    // Add Entries
    let req = AddUserDefinedRulesRequest {
        rules: vec![domain_rule(domain1.clone()), domain_rule(domain2.clone())],
    };
    let res = test_server
        .post(&format!("/network-blocks/{}/user-rules", net_list_id))
        .json(&req)
        .await;
    res.assert_status_ok();

    // Get Entries
    let res = test_server
        .get(&format!("/network-blocks/{}/user-rules", net_list_id))
        .await;
    let body: GetUserDefinedRulesResponse = res.json();

    let entries = body.user_defined_rules.clone();
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|r| r.rule == domain1));
    assert!(entries.iter().any(|r| r.rule == domain2));

    let rule_to_delete = entries.iter().find(|r| r.rule == domain1).unwrap();
    let rule_id = rule_to_delete.id;

    // Delete Entry
    let res = test_server
        .delete(&format!("/user-rules/{}", rule_id))
        .await;
    res.assert_status_ok();

    // Verify Deletion
    let res = test_server
        .get(&format!("/network-blocks/{}/user-rules", net_list_id))
        .await;
    let body: GetUserDefinedRulesResponse = res.json();

    let entries = body.user_defined_rules.clone();
    assert_eq!(entries.len(), 1);
    assert!(!entries.iter().any(|r| r.rule == domain1));
    assert!(entries.iter().any(|r| r.rule == domain2));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_user_defined_entries_bulk() {
    let test_server = get_default_test_server();

    // Create Network List
    let req = CreateNetworkBlockRequest {
        block_name: Name().fake(),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    let net_list_id = res.json::<CreateNetworkBlockResponse>().network_block.id;

    // Add 50 unique Entries at once
    let mut rules_set = std::collections::HashSet::new();
    while rules_set.len() < 50 {
        rules_set.insert(format!("{}.com", Word().fake::<String>().to_lowercase()));
    }
    let rules: Vec<String> = rules_set.into_iter().collect();

    let req = AddUserDefinedRulesRequest {
        rules: rules.iter().cloned().map(domain_rule).collect(),
    };
    let res = test_server
        .post(&format!("/network-blocks/{}/user-rules", net_list_id))
        .json(&req)
        .await;
    res.assert_status_ok();

    // Get Entries and verify count
    let res = test_server
        .get(&format!("/network-blocks/{}/user-rules", net_list_id))
        .await;
    let body: GetUserDefinedRulesResponse = res.json();
    assert_eq!(body.user_defined_rules.len(), 50);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_user_defined_entries_fuzz() {
    let test_server = get_default_test_server();

    // Create Network List
    let req = CreateNetworkBlockRequest {
        block_name: Name().fake(),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    let net_list_id = res.json::<CreateNetworkBlockResponse>().network_block.id;

    // Add various random patterns (regexes, strange chars, etc)
    let fuzz_patterns = vec![
        ".*\\.bad-site\\.com".to_string(),
        "^https?://[a-z]+\\.evil\\.org".to_string(),
        "!!!".to_string(),
        " ".to_string(),
        "💩.com".to_string(),
        "A".repeat(255),
    ];

    for pattern in fuzz_patterns {
        let req = AddUserDefinedRulesRequest {
            rules: vec![regex_rule(pattern.clone())],
        };
        let res = test_server
            .post(&format!("/network-blocks/{}/user-rules", net_list_id))
            .json(&req)
            .await;
        // Some might be invalid regex, which is OK to fail, but we want to see it doesn't crash
        if res.status_code() == http::StatusCode::OK {
            // Verify it was added
            let res = test_server
                .get(&format!("/network-blocks/{}/user-rules", net_list_id))
                .await;
            let body: GetUserDefinedRulesResponse = res.json();
            assert!(body.user_defined_rules.iter().any(|r| r.rule == pattern));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_user_defined_entries_lock_constraints() {
    let test_server = get_default_test_server();

    // 1. Create Network List
    let req = CreateNetworkBlockRequest {
        block_name: Name().fake(),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    let net_list_id = res.json::<CreateNetworkBlockResponse>().network_block.id;

    // 2. Add initial rule
    let domain: String = Word().fake();
    let initial_rule = format!("{}.com", domain.to_lowercase());
    let req = AddUserDefinedRulesRequest {
        rules: vec![domain_rule(initial_rule.clone())],
    };
    test_server
        .post(&format!("/network-blocks/{}/user-rules", net_list_id))
        .json(&req)
        .await
        .assert_status_ok();

    // Get rule ID
    let res = test_server
        .get(&format!("/network-blocks/{}/user-rules", net_list_id))
        .await;
    let body: GetUserDefinedRulesResponse = res.json();
    let rule_id = body.user_defined_rules[0].id;

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

    // 4. Adding rule should SUCCEED when locked (as it does not weaken the block)
    let domain2: String = Word().fake();
    let next_rule = format!("{}.org", domain2.to_lowercase());
    let req = AddUserDefinedRulesRequest {
        rules: vec![domain_rule(next_rule.clone())],
    };
    let res = test_server
        .post(&format!("/network-blocks/{}/user-rules", net_list_id))
        .json(&req)
        .await;
    res.assert_status_ok();

    // 5. Deleting rule should FAIL when locked
    let res = test_server
        .delete(&format!("/user-rules/{}", rule_id))
        .await;
    res.assert_status(http::StatusCode::FORBIDDEN);
}
