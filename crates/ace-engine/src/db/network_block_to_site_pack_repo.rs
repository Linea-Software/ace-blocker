use diesel::prelude::*;

use super::models::NetworkBlockToSitePackModel;
use crate::db::{DBError, EngineDB};

impl EngineDB {
    pub fn get_network_block_to_site_packs_of(
        &self,
        network_block_id: &uuid::Uuid,
    ) -> Result<Vec<NetworkBlockToSitePackModel>, DBError> {
        use crate::schema_main_db::network_block_to_site_pack::dsl;
        let conn = &mut self.get_db_connection()?;

        dsl::network_block_to_site_pack
            .filter(dsl::network_block_id.eq(network_block_id.to_string()))
            .load::<NetworkBlockToSitePackModel>(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "network_block_to_site_pack".to_string(),
                message: error.to_string(),
            })
    }

    pub fn add_network_block_to_site_pack(
        &self,
        network_block_id: &uuid::Uuid,
        site_pack_id: &str,
    ) -> Result<NetworkBlockToSitePackModel, DBError> {
        use crate::schema_main_db::network_block_to_site_pack::dsl;

        let conn = &mut self.get_db_connection()?;

        let new_conn = NetworkBlockToSitePackModel {
            network_block_id: network_block_id.to_string(),
            site_pack_id: site_pack_id.to_string(),
        };

        diesel::insert_into(dsl::network_block_to_site_pack)
            .values(&new_conn)
            .get_result(conn)
            .map_err(|error| DBError::InsertionError {
                table: "network_block_to_site_pack".to_string(),
                message: error.to_string(),
            })
    }

    pub fn remove_network_block_to_site_pack(
        &self,
        network_block_id_in: &uuid::Uuid,
        site_pack_database_id_in: &str,
    ) -> Result<(), DBError> {
        use crate::schema_main_db::network_block_to_site_pack::dsl::*;

        let conn = &mut self.get_db_connection()?;

        diesel::delete(
            network_block_to_site_pack
                .filter(network_block_id.eq(network_block_id_in.to_string()))
                .filter(site_pack_id.eq(site_pack_database_id_in)),
        )
        .execute(conn)
        .map_err(|error| DBError::DeletionError {
            table: "network_block_to_site_pack".to_string(),
            message: error.to_string(),
        })?;
        Ok(())
    }
}
