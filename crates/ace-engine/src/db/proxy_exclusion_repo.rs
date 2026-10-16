use diesel::RunQueryDsl;

use crate::{
    db::{DBError, models::ProxyExlusionModel},
    schema_main_db::proxy_exclusions,
};

use super::EngineDB;

impl EngineDB {
    pub fn get_proxy_exclusions(&self) -> Result<Vec<ProxyExlusionModel>, DBError> {
        let conn = &mut self.get_db_connection()?;

        proxy_exclusions::table
            .load::<ProxyExlusionModel>(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "proxy_exclusions".to_string(),
                message: error.to_string(),
            })
    }
}
