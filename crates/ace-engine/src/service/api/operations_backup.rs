use crate::service::{
    api::{AppState, error::ApiError},
    clients::AceServerClient,
};

pub(super) async fn export_backup(
    state: AppState,
) -> Result<crate::manager::network::backup::EngineBackup, ApiError> {
    let mut manager = state.network_manager.write().await;
    let backup = manager.export_backup()?;
    Ok(backup)
}

pub(super) async fn export_backup_to_cloud(state: AppState) -> Result<(), ApiError> {
    let is_pro = state.is_pro_account_entitled().await;

    if !is_pro {
        return Err(ApiError::Forbidden(
            "Cloud backup is a Pro feature".to_string(),
        ));
    }

    let api_key = state
        .account_manager
        .get_api_key()
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::Unauthorized("Not logged in".to_string()))?;

    let backup = {
        let mut manager = state.network_manager.write().await;
        manager.export_backup()?
    };

    let client = AceServerClient::new_http_client(Some(&api_key));

    use crate::ace_server_api::types::*;

    let cloud_backup = CreateBackupRequest {
        version: backup.version,
        custom_sites_lists: backup
            .custom_site_packs
            .into_iter()
            .map(|list| CreateBackupRequestCustomSitesListsItem {
                id: list.id,
                name: list.name,
                description: Some(list.description),
                remote_id: Some(list.remote_id),
                requires_pro: Some(list.requires_pro),
                version: Some(list.version),
                sources: list
                    .sources
                    .into_iter()
                    .map(
                        |source| CreateBackupRequestCustomSitesListsItemSourcesItem {
                            source: source.source,
                            source_type: source.source_type,
                            format: source.format,
                        },
                    )
                    .collect(),
            })
            .collect(),
        network_blocks: backup
            .network_blocks
            .into_iter()
            .map(|block| CreateBackupRequestNetworkBlocksItem {
                id: block.id,
                name: block.name,
                icon: Some(block.icon),
                safe_search: block.safe_search,
                user_rules: block
                    .user_rules
                    .into_iter()
                    .map(|rule| CreateBackupRequestNetworkBlocksItemUserRulesItem {
                        rule: rule.rule,
                        rule_type: match rule.rule_type {
                            crate::core::UserDefinedRuleType::Domain => {
                                CreateBackupRequestNetworkBlocksItemUserRulesItemRuleType::Domain
                            }
                            crate::core::UserDefinedRuleType::Regex => {
                                CreateBackupRequestNetworkBlocksItemUserRulesItemRuleType::Regex
                            }
                        },
                    })
                    .collect(),
                connected_sites_lists: block.connected_site_packs,
                schedules: block
                    .schedules
                    .into_iter()
                    .map(|sched| CreateBackupRequestNetworkBlocksItemSchedulesItem {
                        start_time: sched.start_time,
                        end_time: sched.end_time,
                        monday: sched.monday,
                        tuesday: sched.tuesday,
                        wednesday: sched.wednesday,
                        thursday: sched.thursday,
                        friday: sched.friday,
                        saturday: sched.saturday,
                        sunday: sched.sunday,
                    })
                    .collect(),
            })
            .collect(),
        deactivated_resistance_mode_scripts: backup.deactivated_resistance_scripts,
    };

    client
        .create_backup(cloud_backup)
        .await
        .map_err(|error| ApiError::Internal(format!("Cloud backup failed: {}", error)))?;

    Ok(())
}

pub(super) async fn import_backup_from_cloud(state: AppState) -> Result<(), ApiError> {
    let is_pro = state.is_pro_account_entitled().await;

    if !is_pro {
        return Err(ApiError::Forbidden(
            "Cloud import is a Pro feature".to_string(),
        ));
    }

    let api_key = state
        .account_manager
        .get_api_key()
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::Unauthorized("Not logged in".to_string()))?;

    let client = AceServerClient::new_http_client(Some(&api_key));
    let backup = client
        .fetch_backup()
        .await
        .map_err(|error| ApiError::Internal(format!("Cloud import failed: {}", error)))?;

    use crate::ace_server_api::types::FetchBackupResponseNetworkBlocksItemUserRulesItemRuleType;
    use crate::manager::network::backup::*;

    let internal_backup = EngineBackup {
        version: backup.version,
        custom_site_packs: backup
            .custom_sites_lists
            .into_iter()
            .map(|list| BackupSitePack {
                id: list.id,
                remote_id: list.remote_id.flatten(),
                requires_pro: list.requires_pro.unwrap_or(false),
                version: list.version.flatten(),
                name: list.name,
                description: list.description.flatten(),
                icon: None,
                sources: list
                    .sources
                    .into_iter()
                    .map(|source| BackupSitePackSource {
                        source: source.source,
                        source_type: source.source_type,
                        format: source.format,
                    })
                    .collect(),
            })
            .collect(),
        network_blocks: backup
            .network_blocks
            .into_iter()
            .map(|block| BackupNetworkBlock {
                id: block.id,
                name: block.name,
                icon: block.icon.flatten(),
                // The remote backup contract predates this field. Treat it like a
                // legacy local backup and preserve the historical enabled behavior.
                enabled: true,
                safe_search: block.safe_search,
                user_rules: block
                    .user_rules
                    .into_iter()
                    .map(|rule| {
                        let rule_type = match rule.rule_type {
                            FetchBackupResponseNetworkBlocksItemUserRulesItemRuleType::Domain => {
                                crate::core::UserDefinedRuleType::Domain
                            }
                            FetchBackupResponseNetworkBlocksItemUserRulesItemRuleType::Regex => {
                                crate::core::UserDefinedRuleType::Regex
                            }
                        };
                        crate::core::UserDefinedRule::new(rule.rule, rule_type)
                    })
                    .collect(),
                connected_site_packs: block.connected_sites_lists,
                schedules: block
                    .schedules
                    .into_iter()
                    .map(|sched| BackupScheduleEntry {
                        start_time: sched.start_time,
                        end_time: sched.end_time,
                        monday: sched.monday,
                        tuesday: sched.tuesday,
                        wednesday: sched.wednesday,
                        thursday: sched.thursday,
                        friday: sched.friday,
                        saturday: sched.saturday,
                        sunday: sched.sunday,
                    })
                    .collect(),
            })
            .collect(),
        deactivated_resistance_scripts: backup.deactivated_resistance_mode_scripts,
    };

    let mut manager = state.network_manager.write().await;
    manager.import_backup(internal_backup).await?;

    Ok(())
}

pub(super) async fn import_backup(
    state: AppState,
    payload: crate::manager::network::backup::EngineBackup,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;

    manager.import_backup(payload).await?;

    Ok(())
}
