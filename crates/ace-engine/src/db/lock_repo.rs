use crate::{
    db::{DBError, models::NetworkBlockLockModel},
    schema_main_db::network_block_locks,
};
use diesel::prelude::*;

use super::EngineDB;

impl EngineDB {
    // --- Net List Locks ---

    pub fn get_lock_of(
        &self,
        network_block_id: &uuid::Uuid,
    ) -> Result<NetworkBlockLockModel, DBError> {
        let conn = &mut self.get_db_connection()?;
        network_block_locks::table
            .filter(network_block_locks::network_block_id.eq(network_block_id.to_string()))
            .select(NetworkBlockLockModel::as_select())
            .get_result(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "network_block_locks".to_string(),
                message: error.to_string(),
            })
    }

    pub fn set_lock(
        &self,
        lock_model: NetworkBlockLockModel,
    ) -> Result<NetworkBlockLockModel, DBError> {
        use crate::schema_main_db::network_block_locks::dsl;
        let conn = &mut self.get_db_connection()?;

        diesel::update(network_block_locks::table)
            .filter(dsl::network_block_id.eq(lock_model.network_block_id))
            .set((
                dsl::lock_state.eq(lock_model.lock_state),
                dsl::lock_end.eq(lock_model.lock_end),
            ))
            .get_result(conn)
            .map_err(|error| DBError::UpdateError {
                table: "network_block_locks".to_string(),
                message: error.to_string(),
            })
    }
}
