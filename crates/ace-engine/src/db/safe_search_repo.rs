use super::{DBError, EngineDB};
use crate::db::models::SafeSearchEnforcementModel;
use crate::schema_main_db;
use diesel::prelude::*;

impl EngineDB {
    pub fn get_all_safe_search_enforcements(
        &self,
    ) -> Result<Vec<SafeSearchEnforcementModel>, DBError> {
        let conn = &mut self.get_db_connection()?;

        schema_main_db::safe_search_enforcements::table
            .select(SafeSearchEnforcementModel::as_select())
            .load::<SafeSearchEnforcementModel>(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "safe_search_enforcements".to_string(),
                message: error.to_string(),
            })
    }

    pub fn add_safe_search_redirection(
        &self,
        domain_name: String,
        target_host_name: String,
    ) -> Result<SafeSearchEnforcementModel, DBError> {
        use crate::schema_main_db::safe_search_enforcements;

        let conn = &mut self.get_db_connection()?;

        let new_enforcement = SafeSearchEnforcementModel {
            domain: domain_name,
            enforcement_type: "domain_redirection".to_string(),
            target_host: Some(target_host_name),
            query_param_key: None,
            query_param_value: None,
        };

        diesel::insert_into(safe_search_enforcements::table)
            .values(&new_enforcement)
            .get_result::<SafeSearchEnforcementModel>(conn)
            .map_err(|error| DBError::InsertionError {
                table: "safe_search_enforcements".to_string(),
                message: error.to_string(),
            })
    }

    pub fn add_safe_search_query_param(
        &self,
        domain_name: String,
        key_name: String,
        value_name: String,
    ) -> Result<SafeSearchEnforcementModel, DBError> {
        use crate::schema_main_db::safe_search_enforcements;

        let conn = &mut self.get_db_connection()?;

        let new_enforcement = SafeSearchEnforcementModel {
            domain: domain_name,
            enforcement_type: "query_parameter".to_string(),
            target_host: None,
            query_param_key: Some(key_name),
            query_param_value: Some(value_name),
        };

        diesel::insert_into(safe_search_enforcements::table)
            .values(&new_enforcement)
            .get_result::<SafeSearchEnforcementModel>(conn)
            .map_err(|error| DBError::InsertionError {
                table: "safe_search_enforcements".to_string(),
                message: error.to_string(),
            })
    }
}
