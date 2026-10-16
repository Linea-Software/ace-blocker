//! Mock API client for integration tests
//! This module provides a type-safe HTTP client that works with axum-test

use uuid::Uuid;

pub use ace_engine::service::api::schemas::*;

/// HTTP Client for testing the API
pub struct HttpClient {
    base_url: String,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            base_url: String::new(),
        }
    }

    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    // --- Account Routes ---

    pub async fn get_account_info(
        &self,
    ) -> Result<AccountInfoResponse, Box<dyn std::error::Error>> {
        let response = reqwest::get(format!("{}/account/info", self.base_url)).await?;
        let info = response.json::<AccountInfoResponse>().await?;
        Ok(info)
    }

    pub async fn set_api_key(&self, api_key: String) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/account/set-api-key", self.base_url))
            .query(&[("api_key", &api_key)])
            .send()
            .await?;
        Ok(())
    }

    pub async fn logout(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/account/logout", self.base_url))
            .send()
            .await?;
        Ok(())
    }

    // --- Network Blocks Routes ---

    pub async fn create_network_block(
        &self,
        req: CreateNetworkBlockRequest,
    ) -> Result<CreateNetworkBlockResponse, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/network-blocks", self.base_url))
            .json(&req)
            .send()
            .await?;
        let body = response.json::<CreateNetworkBlockResponse>().await?;
        Ok(body)
    }

    pub async fn get_all_network_blocks(
        &self,
    ) -> Result<GetAllNetworkBlocksResponse, Box<dyn std::error::Error>> {
        let response = reqwest::get(format!("{}/network-blocks", self.base_url)).await?;
        let body = response.json::<GetAllNetworkBlocksResponse>().await?;
        Ok(body)
    }

    pub async fn delete_network_block(
        &self,
        block_id: &Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        client
            .delete(format!("{}/network-blocks/{}", self.base_url, block_id))
            .send()
            .await?;
        Ok(())
    }

    // --- User Rules Routes ---

    pub async fn add_user_rules(
        &self,
        block_id: &Uuid,
        req: AddUserDefinedRulesRequest,
    ) -> Result<AddUserDefinedRulesResponse, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!(
                "{}/network-blocks/{}/user-rules",
                self.base_url, block_id
            ))
            .json(&req)
            .send()
            .await?;
        let body = response.json::<AddUserDefinedRulesResponse>().await?;
        Ok(body)
    }

    pub async fn get_user_rules(
        &self,
        block_id: &Uuid,
    ) -> Result<GetUserDefinedRulesResponse, Box<dyn std::error::Error>> {
        let response = reqwest::get(format!(
            "{}/network-blocks/{}/user-rules",
            self.base_url, block_id
        ))
        .await?;
        let body = response.json::<GetUserDefinedRulesResponse>().await?;
        Ok(body)
    }

    // --- Backup Routes ---

    pub async fn export_backup(&self) -> Result<EngineBackup, Box<dyn std::error::Error>> {
        let response = reqwest::get(format!("{}/backup/export", self.base_url)).await?;
        let body = response.json::<EngineBackup>().await?;
        Ok(body)
    }

    pub async fn import_backup(
        &self,
        backup: EngineBackup,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/backup/import", self.base_url))
            .json(&backup)
            .send()
            .await?;
        Ok(())
    }

    // --- Resistance Mode Routes ---

    pub async fn get_resistance_mode_settings(
        &self,
    ) -> Result<ResistanceModeSettings, Box<dyn std::error::Error>> {
        let response = reqwest::get(format!("{}/resistance-mode/settings", self.base_url)).await?;
        let body = response.json::<ResistanceModeSettings>().await?;
        Ok(body)
    }

    pub async fn set_resistance_mode_enabled(
        &self,
        req: SetEnabledRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/resistance-mode/set-enabled", self.base_url))
            .json(&req)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(format!("Request failed with status: {}", response.status()).into());
        }
        Ok(())
    }

    pub async fn set_resistance_mode_lock(
        &self,
        req: SetNetworkLockRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/resistance-mode/set-lock", self.base_url))
            .json(&req)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(format!("Request failed with status: {}", response.status()).into());
        }
        Ok(())
    }

    /// Raw variant of `set_resistance_mode_lock` returning the full HTTP
    /// response so tests can assert on status codes and error envelopes.
    pub async fn set_resistance_mode_lock_raw(
        &self,
        req: SetNetworkLockRequest,
    ) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/resistance-mode/set-lock", self.base_url))
            .json(&req)
            .send()
            .await?;
        Ok(response)
    }

    pub async fn set_resistance_mode_type(
        &self,
        mode: &PunishmentMode,
    ) -> Result<ResistanceModeTypeResponse, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/resistance-mode/type", self.base_url))
            .json(mode)
            .send()
            .await?;
        let body = response.json::<ResistanceModeTypeResponse>().await?;
        Ok(body)
    }

    pub async fn get_all_scripts(
        &self,
    ) -> Result<GetAllScriptsResponse, Box<dyn std::error::Error>> {
        let response = reqwest::get(format!("{}/resistance-mode/scripts", self.base_url)).await?;
        let body = response.json::<GetAllScriptsResponse>().await?;
        Ok(body)
    }

    pub async fn get_script(
        &self,
        script_id: &str,
    ) -> Result<ScriptInfo, Box<dyn std::error::Error>> {
        let response = reqwest::get(format!(
            "{}/resistance-mode/scripts/{}",
            self.base_url, script_id
        ))
        .await?;
        let body = response.json::<ScriptInfo>().await?;
        Ok(body)
    }

    pub async fn set_script_enabled(
        &self,
        script_id: &str,
        req: SetEnabledRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!(
                "{}/resistance-mode/scripts/{}/set-enabled",
                self.base_url, script_id
            ))
            .json(&req)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(format!("Request failed with status: {}", response.status()).into());
        }
        Ok(())
    }

    // --- Dev Routes (debug only) ---

    pub async fn get_debug_pro_policy(&self) -> Result<String, Box<dyn std::error::Error>> {
        let response = reqwest::get(format!("{}/dev/pro-policy", self.base_url)).await?;
        let body = response.text().await?;
        Ok(body)
    }

    pub async fn get_debug_resistance_level(&self) -> Result<String, Box<dyn std::error::Error>> {
        let response = reqwest::get(format!("{}/dev/resistance-level", self.base_url)).await?;
        let body = response.text().await?;
        Ok(body)
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}
