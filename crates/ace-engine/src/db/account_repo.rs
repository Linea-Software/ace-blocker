use crate::db::{DBError, EngineDB, models::AccountModel};
use diesel::prelude::*;

impl EngineDB {
    pub fn get_account(&self) -> Result<AccountModel, DBError> {
        use crate::schema_main_db::accounts::dsl::*;
        let conn = &mut self.get_db_connection()?;

        accounts
            .filter(table_lock.eq("0"))
            .select(AccountModel::as_select())
            .first(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "accounts".to_string(),
                message: error.to_string(),
            })
    }

    pub fn update_account(
        &self,
        api_key_val: Option<String>,
        last_pro_check_val: Option<String>,
    ) -> Result<AccountModel, DBError> {
        use crate::schema_main_db::accounts::dsl::*;
        let conn = &mut self.get_db_connection()?;

        diesel::update(accounts.filter(table_lock.eq("0")))
            .set((
                api_key.eq(api_key_val),
                last_pro_check.eq(last_pro_check_val),
                entitlement_token.eq(None::<String>),
            ))
            .get_result(conn)
            .map_err(|error| DBError::UpdateError {
                table: "accounts".to_string(),
                message: error.to_string(),
            })
    }

    pub fn set_api_key(&self, api_key_val: Option<String>) -> Result<AccountModel, DBError> {
        use crate::schema_main_db::accounts::dsl::*;
        let conn = &mut self.get_db_connection()?;

        diesel::update(accounts.filter(table_lock.eq("0")))
            .set((
                api_key.eq(api_key_val),
                entitlement_token.eq(None::<String>),
            ))
            .get_result(conn)
            .map_err(|error| DBError::UpdateError {
                table: "accounts".to_string(),
                message: error.to_string(),
            })
    }

    /// Persist only server-signed credentials, never an unsigned Pro boolean.
    pub fn update_entitlement_token(
        &self,
        token: Option<String>,
        observed_at: String,
    ) -> Result<AccountModel, DBError> {
        use crate::schema_main_db::accounts::dsl::*;
        let conn = &mut self.get_db_connection()?;
        diesel::update(accounts.filter(table_lock.eq("0")))
            .set((
                entitlement_token.eq(token),
                clock_high_watermark.eq(Some(observed_at)),
            ))
            .get_result(conn)
            .map_err(|error| DBError::UpdateError {
                table: "accounts".to_string(),
                message: error.to_string(),
            })
    }

    pub fn update_account_pro_check_time(
        &self,
        check_time: String,
    ) -> Result<AccountModel, DBError> {
        use crate::schema_main_db::accounts::dsl::*;
        let conn = &mut self.get_db_connection()?;

        diesel::update(accounts.filter(table_lock.eq("0")))
            .set(last_pro_check.eq(check_time))
            .get_result(conn)
            .map_err(|error| DBError::UpdateError {
                table: "accounts".to_string(),
                message: error.to_string(),
            })
    }

    pub fn accept_tos(&self, timestamp: String) -> Result<AccountModel, DBError> {
        use crate::schema_main_db::accounts::dsl::*;
        let conn = &mut self.get_db_connection()?;

        diesel::update(accounts.filter(table_lock.eq("0")))
            .set(tos_accepted_at.eq(timestamp))
            .get_result(conn)
            .map_err(|error| DBError::UpdateError {
                table: "accounts".to_string(),
                message: error.to_string(),
            })
    }

    pub fn reset_tos(&self) -> Result<AccountModel, DBError> {
        use crate::schema_main_db::accounts::dsl::*;
        let conn = &mut self.get_db_connection()?;

        diesel::update(accounts.filter(table_lock.eq("0")))
            .set(tos_accepted_at.eq(None::<String>))
            .get_result(conn)
            .map_err(|error| DBError::UpdateError {
                table: "accounts".to_string(),
                message: error.to_string(),
            })
    }
}
