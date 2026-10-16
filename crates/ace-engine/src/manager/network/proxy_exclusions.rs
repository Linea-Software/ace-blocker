use crate::db::models::ProxyExlusionModel;

use super::{NetworkManager, error::NetworkManagerOpErr};

impl NetworkManager {
    pub fn get_proxy_exclusions(&mut self) -> Result<Vec<ProxyExlusionModel>, NetworkManagerOpErr> {
        self.db
            .get_proxy_exclusions()
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!("failed to fetch proxy exclusions: {error}"),
            })
    }
}
