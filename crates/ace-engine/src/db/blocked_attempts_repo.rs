use crate::db::{DBError, EngineDB, models::BlockedAttemptsModel};
use chrono::NaiveDate;
use diesel::prelude::*;

impl EngineDB {
    /// Increments the aggregate blocked-attempts counter for `date` by one.
    pub fn increment_blocked_attempts(&self, date: NaiveDate) -> Result<(), DBError> {
        self.increment_blocked_attempts_by(date, 1)
    }

    /// Best-effort batched increment. Called by the event-driven proxy statistics
    /// worker, not by the traffic enforcement path.
    pub(crate) fn increment_blocked_attempts_by(
        &self,
        date: NaiveDate,
        amount: i32,
    ) -> Result<(), DBError> {
        use crate::schema_main_db::blocked_attempts::dsl;

        let conn = &mut self.get_db_connection()?;
        let date_str = date.to_string();

        diesel::insert_into(dsl::blocked_attempts)
            .values((
                dsl::date.eq(date_str.clone()),
                dsl::attempts_count.eq(amount),
            ))
            .on_conflict(dsl::date)
            .do_update()
            .set(dsl::attempts_count.eq(dsl::attempts_count + amount))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batched_and_single_increments_share_daily_aggregate() {
        let directory = tempfile::tempdir().unwrap();
        let db = EngineDB::from(&directory.path().join("statistics.sqlite")).unwrap();
        let first_day = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let second_day = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();

        assert_eq!(db.get_blocked_attempts_for_date(first_day).unwrap(), 0);
        db.increment_blocked_attempts_by(first_day, 25).unwrap();
        db.increment_blocked_attempts(first_day).unwrap();
        db.increment_blocked_attempts_by(first_day, 75).unwrap();
        db.increment_blocked_attempts_by(second_day, 3).unwrap();

        assert_eq!(db.get_blocked_attempts_for_date(first_day).unwrap(), 101);
        assert_eq!(db.get_blocked_attempts_for_date(second_day).unwrap(), 3);
    }
}
