use super::models::ScriptStateModel;
use super::{DBError, EngineDB};
use crate::schema_main_db::script_states;
use diesel::prelude::*;

impl EngineDB {
    pub fn get_script_state(&self, script_id: &str) -> Result<Option<ScriptStateModel>, DBError> {
        let mut conn = self.get_db_connection()?;

        script_states::table
            .filter(script_states::script_id.eq(script_id))
            .first::<ScriptStateModel>(&mut conn)
            .optional()
            .map_err(|error| DBError::DataFetchError {
                table: "script_states".to_string(),
                message: error.to_string(),
            })
    }

    pub fn set_script_state(
        &self,
        script_id: &str,
        enabled: bool,
    ) -> Result<ScriptStateModel, DBError> {
        let mut conn = self.get_db_connection()?;

        let new_state = ScriptStateModel {
            script_id: script_id.to_string(),
            enabled,
        };

        diesel::insert_into(script_states::table)
            .values(&new_state)
            .on_conflict(script_states::script_id)
            .do_update()
            .set(script_states::enabled.eq(enabled))
            .get_result::<ScriptStateModel>(&mut conn)
            .map_err(|error| DBError::InsertionError {
                table: "script_states".to_string(),
                message: error.to_string(),
            })
    }

    pub fn get_all_script_states(&self) -> Result<Vec<ScriptStateModel>, DBError> {
        let mut conn = self.get_db_connection()?;

        script_states::table
            .load::<ScriptStateModel>(&mut conn)
            .map_err(|error| DBError::DataFetchError {
                table: "script_states".to_string(),
                message: error.to_string(),
            })
    }
}
