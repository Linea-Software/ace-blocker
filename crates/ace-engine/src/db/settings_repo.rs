use crate::db::{DBError, EngineDB, models::SettingsModel};
use diesel::prelude::*;

impl EngineDB {
    pub fn get_settings(&self) -> Result<SettingsModel, DBError> {
        use crate::schema_main_db::settings::dsl::*;
        let conn = &mut self.get_db_connection()?;

        settings
            .filter(table_lock.eq("0"))
            .select(SettingsModel::as_select())
            .first(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "settings".to_string(),
                message: error.to_string(),
            })
    }

    pub fn update_pro_status(&self, is_pro_val: bool) -> Result<(), DBError> {
        use crate::schema_main_db::settings::dsl::*;
        let conn = &mut self.get_db_connection()?;

        diesel::update(settings.filter(table_lock.eq("0")))
            .set(is_pro.eq(is_pro_val))
            .execute(conn)
            .map_err(|error| DBError::UpdateError {
                table: "settings".to_string(),
                message: error.to_string(),
            })?;

        Ok(())
    }

    pub fn update_seeded_status(&self, seeded: bool) -> Result<(), DBError> {
        use crate::schema_main_db::settings::dsl;
        let conn = &mut self.get_db_connection()?;

        diesel::update(dsl::settings.filter(dsl::table_lock.eq("0")))
            .set(dsl::is_seeded.eq(seeded))
            .execute(conn)
            .map_err(|error| DBError::UpdateError {
                table: "settings".to_string(),
                message: error.to_string(),
            })?;

        Ok(())
    }

    pub fn update_latest_update_check_at(&self, date: String) -> Result<(), DBError> {
        use crate::schema_main_db::settings::dsl;
        let conn = &mut self.get_db_connection()?;

        diesel::update(dsl::settings.filter(dsl::table_lock.eq("0")))
            .set(dsl::latest_update_check_at.eq(date))
            .execute(conn)
            .map_err(|error| DBError::UpdateError {
                table: "settings".to_string(),
                message: error.to_string(),
            })?;

        Ok(())
    }
}
