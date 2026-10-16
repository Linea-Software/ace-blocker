use crate::api_client::{
    AddUserDefinedRulesRequest, CreateNetworkBlockRequest, SetEnabledRequest, UserDefinedRule,
    UserDefinedRuleType,
};
use crate::common::{get_api_client_from, get_default_test_server};

#[tokio::test]
async fn test_backup_export_import() {
    let server = get_default_test_server();
    let client = get_api_client_from(&server);

    // 1. Create some data
    let create_res = client
        .create_network_block(CreateNetworkBlockRequest {
            block_name: "Backup Test Block".to_string(),
            icon: None,
        })
        .await
        .expect("Failed to create network block");

    let block_id = create_res.network_block.id;

    client
        .add_user_rules(
            &block_id,
            AddUserDefinedRulesRequest {
                rules: vec![UserDefinedRule::new(
                    "backup.example.com".to_string(),
                    UserDefinedRuleType::Domain,
                )],
            },
        )
        .await
        .expect("Failed to add user rules");

    server
        .post(&format!("/network-blocks/{block_id}/enabled"))
        .json(&SetEnabledRequest { enabled: false })
        .await
        .assert_status_ok();

    // 2. Export backup
    let backup = client
        .export_backup()
        .await
        .expect("Failed to export backup");

    assert_eq!(backup.version, "1.0");
    let backed_up_block = backup
        .network_blocks
        .iter()
        .find(|block| block.name == "Backup Test Block")
        .expect("Backup Test Block must be exported");
    assert!(!backed_up_block.enabled);

    // 3. Clear data by using a fresh server (this is the easiest way in these integration tests)
    let server2 = get_default_test_server();
    let client2 = get_api_client_from(&server2);

    // Verify it's empty
    let blocks = client2
        .get_all_network_blocks()
        .await
        .expect("Failed to get blocks");
    assert!(
        !blocks
            .network_blocks
            .iter()
            .any(|b| b.name == "Backup Test Block")
    );

    // 4. Import backup
    client2
        .import_backup(backup)
        .await
        .expect("Failed to import backup");

    // 5. Verify data is restored
    let blocks = client2
        .get_all_network_blocks()
        .await
        .expect("Failed to get blocks after import");
    let restored_block = blocks
        .network_blocks
        .iter()
        .find(|b| b.name == "Backup Test Block")
        .expect("Restored block not found");
    assert!(!restored_block.enabled);

    let rules = client2
        .get_user_rules(&restored_block.id)
        .await
        .expect("Failed to get user rules");
    assert!(
        rules
            .user_defined_rules
            .iter()
            .any(|r| r.rule == "backup.example.com")
    );
}
