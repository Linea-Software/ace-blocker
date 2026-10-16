use crate::{
    db::{
        DBError, EngineDB,
        models::{NetworkBlockModel, UserDefinedRuleModel},
    },
    schema_main_db::{network_blocks, user_defined_rules},
};
use diesel::prelude::*;

impl EngineDB {
    pub fn get_user_defined_rules_of(
        &self,
        network_block_id: &uuid::Uuid,
    ) -> Result<Vec<UserDefinedRuleModel>, DBError> {
        let conn = &mut self.get_db_connection()?;

        user_defined_rules::table
            .select(UserDefinedRuleModel::as_select())
            .filter(user_defined_rules::network_block_id.eq(network_block_id.to_string()))
            .load(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "user_defined_rules".to_string(),
                message: error.to_string(),
            })
    }

    pub fn get_network_block_of_user_defined_rule(
        &self,
        user_defined_entry_id: &uuid::Uuid,
    ) -> Result<NetworkBlockModel, DBError> {
        let conn = &mut self.get_db_connection()?;

        network_blocks::table
            .inner_join(user_defined_rules::table)
            .filter(user_defined_rules::id.eq(user_defined_entry_id.to_string()))
            .select(NetworkBlockModel::as_select()) // Ensure we get a NetListModel
            .get_result(conn) // Returns Result<NetListModel, diesel::Error>
            .map_err(|error| DBError::DataFetchError {
                table: "user_defined_rules".to_string(),
                message: error.to_string(),
            })
    }

    pub fn add_user_defined_rules(
        &self,
        network_block_id: &uuid::Uuid,
        entries: &[crate::core::UserDefinedRule],
    ) -> Result<Vec<UserDefinedRuleModel>, DBError> {
        use crate::schema_main_db::user_defined_rules::dsl;

        let conn = &mut self.get_db_connection()?;

        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            let mut results = Vec::new();
            for entry in entries {
                let model = diesel::insert_into(dsl::user_defined_rules)
                    .values((
                        dsl::id.eq(uuid::Uuid::now_v7().to_string()),
                        dsl::network_block_id.eq(network_block_id.to_string()),
                        dsl::rule.eq(&entry.rule),
                        dsl::rule_type.eq(entry.rule_type.to_string()),
                    ))
                    .get_result::<UserDefinedRuleModel>(conn)?;
                results.push(model);
            }
            Ok(results)
        })
        .map_err(|error| DBError::InsertionError {
            table: "user_defined_rules".to_string(),
            message: error.to_string(),
        })
    }

    pub fn delete_user_defined_rule(
        &self,
        user_def_rule_id: &uuid::Uuid,
    ) -> Result<UserDefinedRuleModel, DBError> {
        use crate::schema_main_db::user_defined_rules::dsl::*;

        let conn = &mut self.get_db_connection()?;

        let model = diesel::delete(user_defined_rules.find(user_def_rule_id.to_string()))
            .get_result::<UserDefinedRuleModel>(conn)
            .map_err(|error| DBError::DeletionError {
                table: "user_defined_rules".to_string(),
                message: error.to_string(),
            })?;
        Ok(model)
    }

    pub fn delete_user_defined_rule_by_pattern(
        &self,
        network_block_id: &uuid::Uuid,
        rule_pattern: &str,
    ) -> Result<String, DBError> {
        use crate::schema_main_db::user_defined_rules::dsl;

        let conn = &mut self.get_db_connection()?;

        let model = diesel::delete(
            dsl::user_defined_rules
                .filter(dsl::network_block_id.eq(network_block_id.to_string()))
                .filter(dsl::rule.eq(rule_pattern)),
        )
        .get_result::<UserDefinedRuleModel>(conn)
        .map_err(|error| DBError::DeletionError {
            table: "user_defined_rules".to_string(),
            message: error.to_string(),
        })?;
        Ok(model.rule)
    }
}
