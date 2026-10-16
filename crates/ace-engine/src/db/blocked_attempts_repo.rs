use crate::db::{DBError, EngineDB, models::BlockedAttemptsModel};
use chrono::NaiveDate;
use diesel::prelude::*;

impl EngineDB {
    /// Increments the aggregate blocked-attempts counter for `date` by one,
    /// upserting a row on the first attempt of the day.
    pub fn increment_blocked_attempts(&self, date: NaiveDate) -> Result<(), DBError> {
        use crate::schema_main_db::blocked_attempts::dsl;

        let conn = &mut self.get_db_connection()?;
        let date_str = date.to_string();

        diesel::insert_into(dsl::blocked_attempts)
            .values((dsl::date.eq(date_str.clone()), dsl::attempts_count.eq(1)))
            .on_conflict(dsl::date)
            .do_update()
            .set(dsl::attempts_count.eq(dsl::attempts_count + 1))
            .execute(conn)
            .map_err(|error| DBError::InsertionError {
                table: "blocked_attempts".to_string(),
                message: error.to_string(),
            })?;

        Ok(())
    }

    /// Returns the aggregate blocked-attempts count for `date` (0 when the day
    /// has no recorded blocks).
    pub fn get_blocked_attempts_for_date(&self, date: NaiveDate) -> Result<u64, DBError> {
        use crate::schema_main_db::blocked_attempts::dsl;

        let conn = &mut self.get_db_connection()?;
        let date_str = date.to_string();

        let row = dsl::blocked_attempts
            .filter(dsl::date.eq(date_str))
            .select(BlockedAttemptsModel::as_select())
            .first::<BlockedAttemptsModel>(conn)
            .optional()
            .map_err(|error| DBError::DataFetchError {
                table: "blocked_attempts".to_string(),
                message: error.to_string(),
            })?;

        Ok(row.map(|r| r.attempts_count as u64).unwrap_or(0))
    }
}
