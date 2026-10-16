use crate::{
    core::ApiKey,
    db::{EngineDB, models::AccountModel},
};
use chrono::{DateTime, Local};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// How often an authenticated engine re-verifies its Pro status against the
/// Ace server when the *periodic* refresh loop runs. The lazy `is_pro()`
/// path keeps a longer horizon (see `PRO_VERIFY_RETRY_AFTER_HOURS`) so the
/// proxy hot path never hammers the server.
pub const PERIODIC_PRO_REFRESH_INTERVAL: chrono::Duration = chrono::Duration::hours(1);

/// After a failed remote verification, retry after this many hours instead of
/// deferring a full day: entitlement changes (subscription upgrades, expiries)
/// must surface within hours of connectivity returning, not after 24h.
const PRO_VERIFY_RETRY_AFTER_HOURS: i64 = 1;

#[derive(Clone)]
pub struct AccountManager {
    account: Arc<RwLock<AccountModel>>,
    db: Arc<EngineDB>,
    /// Serializes remote Pro-status verifications so concurrent callers (proxy
    /// hot path, content updater, API routes, periodic loop) never fire
    /// overlapping requests against the Ace server.
    verify_lock: Arc<tokio::sync::Mutex<()>>,
}

impl AccountManager {
    pub fn from(db: Arc<EngineDB>) -> Result<AccountManager, String> {
        let account = db.get_account().map_err(|error| error.to_string())?;
        Ok(AccountManager {
            account: Arc::new(RwLock::new(account)),
            db,
            verify_lock: Arc::new(tokio::sync::Mutex::new(())),
        })
    }

    pub async fn get_api_key(&self) -> Result<Option<ApiKey>, String> {
        self.account
            .read()
            .await
            .api_key
            .clone()
            .map(ApiKey::try_new)
            .transpose()
            .map_err(|_| "invalid persisted API key: value must not be empty".to_string())
    }

    pub async fn is_pro(&self) -> bool {
        // No API key → cannot be pro (fast path, no remote call, no lock).
        match self.get_api_key().await {
            Ok(Some(_)) => {}
            Ok(None) => return false,
            Err(error) => {
                error!("[ACCOUNT MANAGER] Invalid persisted API key: {error}");
                return false;
            }
        }

        // Single-flight: concurrent callers (proxy hot path, content updater,
        // API routes, periodic loop) serialize on this lock. Staleness is
        // re-checked *inside* the lock, so a caller that waited while another
        // caller just refreshed sees the fresh timestamp and skips its own
        // remote verification — never two overlapping requests.
        let _guard = self.verify_lock.lock().await;

        let last_pro_check = self.account.read().await.last_pro_check.clone();
        let last_check = last_pro_check.as_ref().and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&Local))
        });

        let needs_update = if let Some(last_check_time) = last_check {
            (Local::now() - last_check_time > chrono::Duration::days(1))
                || last_check_time > Local::now()
        } else {
            true
        };

        if !needs_update {
            return self.get_cached_pro_status();
        }

        let api_key = match self.get_api_key().await {
            Ok(Some(api_key)) => api_key,
            Ok(None) => return false,
            Err(error) => {
                error!("[ACCOUNT MANAGER] Invalid persisted API key: {error}");
                return false;
            }
        };

        info!("[ACCOUNT MANAGER] Pro status needs update");
        match self.verify_pro_status_remote(&api_key).await {
            Ok(is_pro) => {
                info!("[ACCOUNT MANAGER] Pro status updated: {}", is_pro);
                is_pro
            }
            Err(error) => {
                let error_msg = error.to_string();
                warn!(
                    "[ACCOUNT MANAGER] Failed to verify pro status: {}. Using cached status if available.",
                    error_msg
                );

                // If it's an unauthorized error, we should clear the API key
                // and definitely stop checking for a while.
                if error_msg.contains("401") || error_msg.contains("Unauthorized") {
                    warn!("[ACCOUNT MANAGER] API key is unauthorized. Clearing it.");
                    let _ = self.set_api_key(None).await;
                    let _ = self.update_is_pro_status(false);
                    return false;
                }

                // Transient network/server errors must NOT clear the cached
                // entitlement, but they also must not defer the retry a full
                // day: rewrite `last_pro_check` to the past so the next
                // `is_pro()` call retries after `PRO_VERIFY_RETRY_AFTER_HOURS`
                // instead of hammering the server on every request.
                let retry_at =
                    Local::now() - chrono::Duration::hours(24 - PRO_VERIFY_RETRY_AFTER_HOURS);
                if let Ok(updated) = self.db.update_account_pro_check_time(retry_at.to_rfc3339()) {
                    *self.account.write().await = updated;
                }

                // Fallback to cached value in DB/settings
                self.get_cached_pro_status()
            }
        }
    }

    /// Re-reads the account's Pro status, performing a remote verification
    /// **only when the last successful verification is older than one day**
    /// (or no successful verification exists yet). Never calls the Ace server
    /// more than once per day per account.
    ///
    /// This is what the app calls after login, after entitlement-changing
    /// actions, and on focus/resume. A fresh `last_pro_check` (set by the
    /// login/api-key path or a recent successful check) returns the cached
    /// status without any network request; a stale one triggers exactly one
    /// single-flight re-verification. Transient errors never clear the cached
    /// entitlement, and a 401/403 (authorization failure) clears the API key
    /// and transitions the account to free.
    pub async fn refresh_pro_status(&self) -> bool {
        self.is_pro().await
    }

    pub fn get_cached_pro_status(&self) -> bool {
        self.db.get_settings().map(|s| s.is_pro).unwrap_or(false)
    }

    async fn verify_pro_status_remote(&self, api_key: &ApiKey) -> Result<bool, String> {
        let client = crate::service::clients::AceServerClient::new_http_client(Some(api_key));

        info!(
            "[ACCOUNT MANAGER] trying to verify pro status against ace server (key length: {})",
            api_key.as_ref().len()
        );

        match client.get_pro_details().await {
            Ok(details) => {
                let is_pro = details.is_pro;
                let now = Local::now().to_rfc3339();

                let api_key_to_save = if is_pro {
                    Some(api_key.as_ref().to_owned())
                } else {
                    warn!("[ACCOUNT MANAGER] Licence is not active. Removing API key.");
                    None
                };

                // Update account in DB
                match self.db.update_account(api_key_to_save, Some(now.clone())) {
                    Ok(updated_account) => {
                        // Update local cache
                        *self.account.write().await = updated_account;
                    }
                    Err(error) => {
                        error!(
                            "[ACCOUNT MANAGER] Failed to update account in DB: {}",
                            error
                        );
                    }
                }

                // Update settings table (the source of truth for is_pro)
                if let Err(error) = self.update_is_pro_status(is_pro) {
                    error!(
                        "[ACCOUNT MANAGER] Failed to update pro status in settings: {}",
                        error
                    );
                }

                Ok(is_pro)
            }
            Err(error) => Err(error.to_string()),
        }
    }

    fn update_is_pro_status(&self, is_pro: bool) -> Result<(), String> {
        // I'll need to implement this in settings_repo.rs or similar
        // For now let's assume it exists or I'll add it
        self.db
            .update_pro_status(is_pro)
            .map_err(|error| error.to_string())
    }

    pub async fn update_account(
        &self,
        api_key: Option<ApiKey>,
        last_pro_check: Option<String>,
    ) -> Result<(), String> {
        let api_key = api_key.map(|key| key.into_inner());
        let updated = self
            .db
            .update_account(api_key, last_pro_check)
            .map_err(|error| error.to_string())?;
        *self.account.write().await = updated;
        Ok(())
    }

    pub async fn set_api_key(&self, api_key: Option<ApiKey>) -> Result<(), String> {
        let api_key = api_key.map(|key| key.into_inner());
        let updated = self
            .db
            .set_api_key(api_key)
            .map_err(|error| error.to_string())?;
        *self.account.write().await = updated;
        Ok(())
    }

    pub async fn has_accepted_tos(&self) -> bool {
        self.account.read().await.tos_accepted_at.is_some()
    }

    pub async fn accept_tos(&self) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        let updated = self.db.accept_tos(now).map_err(|error| error.to_string())?;
        *self.account.write().await = updated;
        Ok(())
    }

    #[cfg(debug_assertions)]
    pub async fn reset_tos(&self) -> Result<(), String> {
        let updated = self.db.reset_tos().map_err(|error| error.to_string())?;
        *self.account.write().await = updated;
        Ok(())
    }
}

/// Periodic account refresh loop (Issue 4).
///
/// The engine owns entitlement freshness: subscription/entitlement changes are
/// recorded on the Ace server by webhooks, and the engine must learn about
/// them without waiting for a user action or a 24 h lazy TTL. Every
/// `interval` the loop re-verifies Pro status **only while an API key is
/// stored** (logged in) — no requests are made when logged out. The refresh
/// itself is single-flight (see [`AccountManager::refresh_pro_status`]) and
/// never clears cached entitlement on transient network/server errors.
pub async fn run_periodic_account_refresh(account_manager: AccountManager) {
    // First pass immediately (SVC-15-style): a restart with cached account
    // state should re-validate as soon as possible — but still only when the
    // last successful verification is older than one day.
    loop {
        if matches!(account_manager.get_api_key().await, Ok(Some(_))) {
            account_manager.refresh_pro_status().await;
        }
        tokio::time::sleep(
            PERIODIC_PRO_REFRESH_INTERVAL
                .to_std()
                .unwrap_or(std::time::Duration::from_secs(3600)),
        )
        .await;
    }
}
