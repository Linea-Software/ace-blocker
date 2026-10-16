use crate::{
    db::{
        DBError, EngineDB,
        models::{NetworkBlockModel, NetworkBlockScheduleEntryModel},
    },
    schema_main_db::{network_block_schedule_entries, network_blocks},
};
use diesel::prelude::*;

use tracing::{debug, error};

impl EngineDB {
    // --- Net Schedule Entries ---

    pub fn get_schedule_entry_models_of(
        &self,
        network_block_id: &uuid::Uuid,
    ) -> Result<Vec<NetworkBlockScheduleEntryModel>, DBError> {
        debug!(
            "Attempting to get schedule entry models for net list ID: {}",
            network_block_id
        );
        let conn = &mut self.get_db_connection()?;

        network_block_schedule_entries::table
            .select(NetworkBlockScheduleEntryModel::as_select())
            .filter(
                network_block_schedule_entries::network_block_id.eq(network_block_id.to_string()),
            )
            .load::<NetworkBlockScheduleEntryModel>(conn)
            .map_err(|error| {
                error!(
                    "Failed to fetch schedule entry models for net list ID {}: {}",
                    network_block_id, error
                );
                DBError::DataFetchError {
                    table: "network_block_schedule_entries".to_string(),
                    message: error.to_string(),
                }
            })
    }

    pub fn get_network_block_of_schedule_entry(
        &self,
        schedule_entry_id: &uuid::Uuid,
    ) -> Result<NetworkBlockModel, DBError> {
        debug!(
            "Attempting to get net list for schedule entry ID: {}",
            schedule_entry_id
        );
        let conn = &mut self.get_db_connection()?;

        network_blocks::table
            .inner_join(network_block_schedule_entries::table)
            .filter(network_block_schedule_entries::id.eq(schedule_entry_id.to_string()))
            .select(NetworkBlockModel::as_select())
            .get_result(conn)
            .map_err(|error| {
                error!(
                    "Failed to fetch net list for schedule entry ID {}: {}",
                    schedule_entry_id, error
                );
                DBError::DataFetchError {
                    table: "network_blocks".to_string(),
                    message: error.to_string(),
                }
            })
    }

    pub fn replace_schedule_entries(
        &self,
        network_block_id: &uuid::Uuid,
        schedule_entries: Vec<NetworkBlockScheduleEntryModel>,
    ) -> Result<(), DBError> {
        use crate::schema_main_db::network_block_schedule_entries::dsl;
        let conn = &mut self.get_db_connection()?;

        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            diesel::delete(
                dsl::network_block_schedule_entries
                    .filter(dsl::network_block_id.eq(network_block_id.to_string())),
            )
            .execute(conn)?;

            if !schedule_entries.is_empty() {
                let records: Vec<_> = schedule_entries
                    .into_iter()
                    .map(|entry| {
                        (
                            dsl::id.eq(uuid::Uuid::now_v7().to_string()),
                            dsl::network_block_id.eq(entry.network_block_id),
                            dsl::start_time.eq(entry.start_time),
                            dsl::end_time.eq(entry.end_time),
                            dsl::schedule_days_bitmask.eq(entry.schedule_days_bitmask),
                        )
                    })
                    .collect();

                diesel::insert_into(network_block_schedule_entries::table)
                    .values(records)
                    .execute(conn)?;
            }
            Ok(())
        })
        .map_err(|error| {
            error!("Failed to replace schedule entries: {}", error);
            DBError::UpdateError {
                table: "network_block_schedule_entries".to_string(),
                message: error.to_string(),
            }
        })?;

        Ok(())
    }
}
