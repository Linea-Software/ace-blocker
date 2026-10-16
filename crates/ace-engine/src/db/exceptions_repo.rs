use crate::{
    db::{
        DBError, EngineDB,
        models::{ExceptionModel, NetworkBlockModel},
    },
    schema_main_db::{network_block_exceptions, network_blocks},
};
use diesel::prelude::*;
use uuid::Uuid;

impl EngineDB {
    pub fn get_exceptions_of(
        &self,
        network_block_id: &Uuid,
    ) -> Result<Vec<ExceptionModel>, DBError> {
        let conn = &mut self.get_db_connection()?;

        network_block_exceptions::table
            .select(ExceptionModel::as_select())
            .filter(network_block_exceptions::network_block_id.eq(network_block_id.to_string()))
            .load(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "network_block_exceptions".to_string(),
                message: error.to_string(),
            })
    }

    pub fn get_network_block_of_exception(
        &self,
        exception_id: &Uuid,
    ) -> Result<NetworkBlockModel, DBError> {
        let conn = &mut self.get_db_connection()?;

        network_blocks::table
            .inner_join(network_block_exceptions::table)
            .filter(network_block_exceptions::id.eq(exception_id.to_string()))
            .select(NetworkBlockModel::as_select()) // Ensure we get a NetListModel
            .get_result(conn) // Returns Result<NetListModel, diesel::Error>
            .map_err(|error| DBError::DataFetchError {
                table: "network_block_exceptions".to_string(),
                message: error.to_string(),
            })
    }

    pub fn add_exceptions(
        &self,
        network_block_id: &Uuid,
        entries: &Vec<String>,
    ) -> Result<Vec<ExceptionModel>, DBError> {
        use crate::schema_main_db::network_block_exceptions::dsl;

        let conn = &mut self.get_db_connection()?;

        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            let mut results = Vec::new();
            for pattern in entries {
                let model = diesel::insert_into(dsl::network_block_exceptions)
                    .values((
                        dsl::id.eq(uuid::Uuid::now_v7().to_string()),
                        dsl::network_block_id.eq(network_block_id.to_string()),
                        dsl::rule.eq(pattern),
                    ))
                    .get_result::<ExceptionModel>(conn)?;
                results.push(model);
            }
            Ok(results)
        })
        .map_err(|error| DBError::InsertionError {
            table: "network_block_exceptions".to_string(),
            message: error.to_string(),
        })
    }

    pub fn delete_exception(&self, exception_id: &Uuid) -> Result<String, DBError> {
        use crate::schema_main_db::network_block_exceptions::dsl;

        let conn = &mut self.get_db_connection()?;

        let model = diesel::delete(dsl::network_block_exceptions.find(exception_id.to_string()))
            .get_result::<ExceptionModel>(conn)
            .map_err(|error| DBError::DeletionError {
                table: "network_block_exceptions".to_string(),
                message: error.to_string(),
            })?;
        Ok(model.rule)
    }
}
