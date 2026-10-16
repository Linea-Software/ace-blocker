use super::models::ResistanceModeSettingsModel;
use super::{DBError, EngineDB};
use crate::schema_main_db::resistance_mode_settings;
use diesel::prelude::*;

impl EngineDB {
    pub fn get_resistance_mode_settings(&self) -> Result<ResistanceModeSettingsModel, DBError> {
        let mut conn = self.get_db_connection()?;

        resistance_mode_settings::table
            .first::<ResistanceModeSettingsModel>(&mut conn)
            .map_err(|error| DBError::DataFetchError {
                table: "resistance_mode_settings".to_string(),
                message: error.to_string(),
            })
    }

    pub fn set_resistance_mode_settings(
        &self,
        settings: ResistanceModeSettingsModel,
    ) -> Result<ResistanceModeSettingsModel, DBError> {
        let mut conn = self.get_db_connection()?;

        diesel::insert_into(resistance_mode_settings::table)
            .values(&settings)
            .on_conflict(resistance_mode_settings::table_lock)
            .do_update()
            .set(&settings)
            .get_result::<ResistanceModeSettingsModel>(&mut conn)
            .map_err(|error| DBError::InsertionError {
                table: "resistance_mode_settings".to_string(),
                message: error.to_string(),
            })
    }

    pub fn update_scripts_version(&self, version: String) -> Result<(), DBError> {
        let mut conn = self.get_db_connection()?;
        use crate::schema_main_db::resistance_mode_settings::dsl;

        diesel::update(dsl::resistance_mode_settings.filter(dsl::table_lock.eq("0")))
            .set(dsl::scripts_version.eq(version))
            .execute(&mut conn)
            .map_err(|error| DBError::UpdateError {
                table: "resistance_mode_settings".to_string(),
                message: error.to_string(),
            })?;

        Ok(())
    }
}
