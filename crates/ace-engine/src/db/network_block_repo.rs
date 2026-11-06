use super::{DBError, EngineDB};

use crate::db::models::NetworkBlockModel;
use crate::schema_main_db;

use diesel::prelude::*;

impl EngineDB {
    pub fn get_all_network_blocks(&self) -> Result<Vec<NetworkBlockModel>, DBError> {
        let conn = &mut self.get_db_connection()?;

        schema_main_db::network_blocks::table
            .select(NetworkBlockModel::as_select())
            .load::<NetworkBlockModel>(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "network_blocks".to_string(),
                message: error.to_string(),
            })
    }

    pub fn add_network_block(&self, block_list_name: String) -> Result<NetworkBlockModel, DBError> {
        use crate::schema_main_db::network_blocks::dsl::*;

        let conn = &mut self.get_db_connection()?;

        let new_net_list = (
            id.eq(uuid::Uuid::now_v7().to_string()),
            name.eq(block_list_name),
            enabled.eq(true),
            safe_search.eq(false),
            schedule_timezone.eq(crate::core::common::schedule::system_timezone().name()),
        );

        diesel::insert_into(network_blocks)
            .values(new_net_list)
            .get_result::<NetworkBlockModel>(conn)
            .map_err(|error: diesel::result::Error| DBError::InsertionError {
                table: "network_blocks".to_string(),
                message: error.to_string(),
            })
    }

    pub fn add_network_block_with_id(
        &self,
        block_id: &uuid::Uuid,
        block_list_name: String,
    ) -> Result<NetworkBlockModel, DBError> {
        use crate::schema_main_db::network_blocks::dsl::*;

        let conn = &mut self.get_db_connection()?;

        let new_net_list = (
            id.eq(block_id.to_string()),
            name.eq(block_list_name),
            enabled.eq(true),
            safe_search.eq(false),
            schedule_timezone.eq(crate::core::common::schedule::system_timezone().name()),
        );

        diesel::insert_into(network_blocks)
            .values(new_net_list)
            .get_result::<NetworkBlockModel>(conn)
            .map_err(|error: diesel::result::Error| DBError::InsertionError {
                table: "network_blocks".to_string(),
                message: error.to_string(),
            })
    }

    pub fn delete_network_block(&self, net_list_id: &uuid::Uuid) -> Result<(), DBError> {
        use crate::schema_main_db::network_blocks::dsl::*;
        let conn = &mut self.get_db_connection()?;

        diesel::delete(network_blocks.find(net_list_id.to_string()))
            .execute(conn)
            .map_err(|error| DBError::DeletionError {
                table: "network_blocks".to_string(),
                message: error.to_string(),
            })?;

        Ok(())
    }

    pub fn set_safe_search(
        &self,
        net_list_id: &uuid::Uuid,
        is_enabled: bool,
    ) -> Result<(), DBError> {
        use crate::schema_main_db::network_blocks::dsl::*;
        let conn = &mut self.get_db_connection()?;

        diesel::update(network_blocks.find(net_list_id.to_string()))
            .set(safe_search.eq(is_enabled))
            .execute(conn)
            .map_err(|error| DBError::UpdateError {
                table: "network_blocks".to_string(),
                message: error.to_string(),
            })?;

        Ok(())
    }

    pub fn set_network_block_enabled(
        &self,
        net_list_id: &uuid::Uuid,
        is_enabled: bool,
    ) -> Result<(), DBError> {
        use crate::schema_main_db::network_blocks::dsl::{enabled, network_blocks};
        let conn = &mut self.get_db_connection()?;

        diesel::update(network_blocks.find(net_list_id.to_string()))
            .set(enabled.eq(is_enabled))
            .execute(conn)
            .map_err(|error| DBError::UpdateError {
                table: "network_blocks".to_string(),
                message: error.to_string(),
            })?;

        Ok(())
    }

    pub fn rename_network_block(
        &self,
        net_list_id: &uuid::Uuid,
        new_name: String,
    ) -> Result<(), DBError> {
        use crate::schema_main_db::network_blocks::dsl::{name, network_blocks};
        let conn = &mut self.get_db_connection()?;

        diesel::update(network_blocks.find(net_list_id.to_string()))
            .set(name.eq(new_name))
            .execute(conn)
            .map_err(|error| DBError::UpdateError {
                table: "network_blocks".to_string(),
                message: error.to_string(),
            })?;

        Ok(())
    }

    pub fn set_network_block_icon(
        &self,
        net_list_id: &uuid::Uuid,
        icon_str: Option<String>,
    ) -> Result<(), DBError> {
        use crate::schema_main_db::network_blocks::dsl::{icon, network_blocks};
        let conn = &mut self.get_db_connection()?;

        diesel::update(network_blocks.find(net_list_id.to_string()))
            .set(icon.eq(icon_str))
            .execute(conn)
            .map_err(|error| DBError::UpdateError {
                table: "network_blocks".to_string(),
                message: error.to_string(),
            })?;

        Ok(())
    }
}
