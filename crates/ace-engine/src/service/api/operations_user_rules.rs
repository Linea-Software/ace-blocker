use crate::core::NetworkBlockId;

use crate::service::api::{AppState, error::ApiError, schemas::*};

pub(super) async fn get_user_rules(
    state: AppState,
    network_block_id: uuid::Uuid,
) -> Result<GetUserDefinedRulesResponse, ApiError> {
    let mut manager = state.network_manager.write().await;

    let list_id = NetworkBlockId::from(network_block_id);
    let rules = manager.get_user_defined_entries_of(&list_id)?;

    let rules = rules
        .into_iter()
        .map(UserDefinedRuleSchema::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(ApiError::Internal)?;

    Ok(GetUserDefinedRulesResponse {
        user_defined_rules: rules,
    })
}

pub(super) async fn add_user_rules(
    state: AppState,
    network_block_id: uuid::Uuid,
    payload: AddUserDefinedRulesRequest,
) -> Result<AddUserDefinedRulesResponse, ApiError> {
    let mut manager = state.network_manager.write().await;
    let list_id = NetworkBlockId::from(network_block_id);
    let models = manager.add_user_defined_entries(&list_id, payload.rules)?;

    let rules = models
        .into_iter()
        .map(UserDefinedRuleSchema::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(ApiError::Internal)?;

    Ok(AddUserDefinedRulesResponse {
        user_defined_rules: rules,
    })
}

pub(super) async fn delete_user_rule(
    state: AppState,
    user_rule_id: uuid::Uuid,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.delete_user_defined_entry(user_rule_id)?;
    Ok(())
}

