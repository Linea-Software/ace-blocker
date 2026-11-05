use super::signed_entitlement;
use crate::{
    core::ApiKey,
    db::{EngineDB, models::AccountModel},
};
use chrono::{DateTime, Local, Utc};
use serde::Deserialize;
use std::sync::atomic::AtomicI64;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
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

fn pro_check_is_stale(
    last_check: Option<DateTime<Local>>,
    max_age: chrono::Duration,
    now: DateTime<Local>,
) -> bool {
    match last_check {
        None => true,
        Some(last) => last > now || now - last > max_age,
    }
}

#[derive(Clone)]
pub struct AccountManager {
    account: Arc<RwLock<AccountModel>>,
    db: Arc<EngineDB>,
    /// Shared with every clone; the database is authoritative on writes, but
    /// traffic decisions read only this atomic snapshot.
    cached_pro: Arc<AtomicBool>,
    cached_until: Arc<AtomicI64>,
    clock_high_watermark: Arc<AtomicI64>,
    /// Serializes remote Pro-status verifications so concurrent callers (proxy
    /// hot path, content updater, API routes, periodic loop) never fire
    /// overlapping requests against the Ace server.
    verify_lock: Arc<tokio::sync::Mutex<()>>,
}

impl AccountManager {
    pub fn from(db: Arc<EngineDB>) -> Result<AccountManager, String> {
        let account = db.get_account().map_err(|error| error.to_string())?;
        // An old SQLite is_pro flag is never proof of payment. Only a
        // verified, API-key-bound, non-expired server signature grants Pro.
        let now = Utc::now().timestamp();
        let clock_high_watermark = account
            .clock_high_watermark
            .as_deref()
            .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
            .map(|value| value.timestamp())
            .unwrap_or(0);
        let cached_until = account
            .entitlement_token
            .as_deref()
            .zip(account.api_key.as_deref())
            .zip(account.entitlement_public_key.as_deref())
            .and_then(|((token, key), public_key)| {
                signed_entitlement::verify_lease(token, key, now, public_key).ok()
            })
            .unwrap_or(0);
        let cached_pro = cached_until >= now && now + 300 >= clock_high_watermark;
        Ok(AccountManager {
            account: Arc::new(RwLock::new(account)),
            db,
            cached_pro: Arc::new(AtomicBool::new(cached_pro)),
            cached_until: Arc::new(AtomicI64::new(cached_until)),
            clock_high_watermark: Arc::new(AtomicI64::new(clock_high_watermark)),
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
        // The proxy hot path uses the long cache window.
        self.verify_pro_if_stale(chrono::Duration::days(1)).await
    }

    async fn verify_pro_if_stale(&self, max_age: chrono::Duration) -> bool {
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

        let needs_update = pro_check_is_stale(last_check, max_age, Local::now());

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
                if error_msg.contains("401")
                    || error_msg.contains("403")
                    || error_msg.contains("Unauthorized")
                {
                    warn!("[ACCOUNT MANAGER] API key is unauthorized. Clearing it.");
                    let _ = self.set_api_key(None).await;
                    let _ = self.update_is_pro_status(false);
                    return false;
                }

                // A malformed signed entitlement is not a transient outage.
                if error_msg.contains("invalid entitlement") {
                    let _ = self
                        .db
                        .update_entitlement_token(None, None, Utc::now().to_rfc3339());
                    self.cached_pro.store(false, Ordering::Release);
                    self.cached_until.store(0, Ordering::Release);
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

    /// Explicit refresh after returning from checkout/login, or from the
    /// periodic account task. Unlike proxy hot-path checks (24h cache), this
    /// uses a thirty-second freshness window so upgrades become visible promptly.
    /// Concurrent calls remain single-flight; transient errors retain the
    /// last known entitlement.
    pub async fn refresh_pro_status(&self) -> bool {
        self.verify_pro_if_stale(chrono::Duration::seconds(30))
            .await
    }

    pub fn get_cached_pro_status(&self) -> bool {
        let now = Utc::now().timestamp();
        self.cached_pro.load(Ordering::Acquire)
            && now <= self.cached_until.load(Ordering::Acquire)
            && now + 300 >= self.clock_high_watermark.load(Ordering::Acquire)
    }

    /// Unit-test fixture only: emulate a validated lease without contacting
    /// the website. This method is not compiled into production binaries.
    #[cfg(test)]
    pub(crate) fn grant_test_pro_entitlement(&self) {
        self.cached_until
            .store(Utc::now().timestamp() + 3600, Ordering::Release);
        self.cached_pro.store(true, Ordering::Release);
    }

    /// Fast, offline-safe entitlement snapshot for the initial UI state.
    /// This never verifies remotely or holds the network-manager lock. A
    /// logged-out account is never shown as Pro, even if its settings row
    /// still contains a cached entitlement from an earlier session.
    pub async fn get_cached_pro_entitlement(&self) -> bool {
        matches!(self.get_api_key().await, Ok(Some(_))) && self.get_cached_pro_status()
    }

    /// Verify a new account key and its signed Pro entitlement *before*
    /// replacing the local credential. A failed import leaves the previous
    /// account intact and propagates the failure to the protocol receiver.
    pub async fn import_api_key(&self, api_key: ApiKey) -> Result<(), String> {
        self.import_api_key_from(api_key, Self::entitlement_server_origin())
            .await
    }

    async fn import_api_key_from(&self, api_key: ApiKey, base_url: &str) -> Result<(), String> {
        let _guard = self.verify_lock.lock().await;
        let (token, public_key, until) =
            Self::fetch_verified_entitlement(&api_key, base_url).await?;
        let now = Utc::now();
        let mut account = self.account.write().await;
        let updated = self
            .db
            .commit_verified_account(api_key.into_inner(), now.to_rfc3339(), token, public_key)
            .map_err(|error| error.to_string())?;
        *account = updated;
        self.cached_until.store(until, Ordering::Release);
        self.clock_high_watermark
            .fetch_max(now.timestamp(), Ordering::AcqRel);
        self.update_is_pro_status(until >= now.timestamp())?;
        Ok(())
    }

    fn entitlement_server_origin() -> &'static str {
        #[cfg(debug_assertions)]
        {
            "http://localhost:5173"
        }
        #[cfg(not(debug_assertions))]
        {
            "https://aceblocker.com"
        }
    }

    /// Both importing and renewing a license use the same authentication and
    /// signature validation; this step never changes local account state.
    async fn fetch_verified_entitlement(
        api_key: &ApiKey,
        base_url: &str,
    ) -> Result<(Option<String>, Option<String>, i64), String> {
        #[derive(Deserialize)]
        struct Reply {
            token: Option<String>,
            #[serde(rename = "publicKey")]
            public_key: String,
        }
        let response = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|error| error.to_string())?
            .get(format!("{base_url}/api/v1/account/entitlement"))
            .header("x-api-key", api_key.as_ref())
            .send()
            .await
            .map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!(
                "Pro entitlement endpoint returned HTTP {}",
                response.status()
            ));
        }
        let reply: Reply = response.json().await.map_err(|error| error.to_string())?;
        let now = Utc::now();
        // Reject missing/malformed keys, even on Free accounts. The public
        // key is delivered in the authenticated server response (HTTPS in
        // release, loopback in development), never through desktop IPC.
        signed_entitlement::validate_public_key(&reply.public_key)
            .map_err(|error| format!("invalid entitlement public key: {error}"))?;
        let until = match reply.token.as_deref() {
            Some(token) => signed_entitlement::verify_lease(
                token,
                api_key.as_ref(),
                now.timestamp(),
                &reply.public_key,
            )
            .map_err(|error| format!("invalid entitlement: {error}"))?,
            None => 0,
        };
        Ok((reply.token, Some(reply.public_key), until))
    }

    async fn verify_pro_status_remote(&self, api_key: &ApiKey) -> Result<bool, String> {
        let (token, public_key, until) =
            Self::fetch_verified_entitlement(api_key, Self::entitlement_server_origin()).await?;
        let now = Utc::now();
        // A renewal must not overwrite a key that changed during the request.
        let mut account = self.account.write().await;
        if account.api_key.as_deref() != Some(api_key.as_ref()) {
            return Err("API key changed while refreshing entitlement".to_owned());
        }
        let updated = self
            .db
            .commit_verified_account(
                api_key.as_ref().to_owned(),
                now.to_rfc3339(),
                token,
                public_key,
            )
            .map_err(|error| error.to_string())?;
        *account = updated;
        self.cached_until.store(until, Ordering::Release);
        self.clock_high_watermark
            .fetch_max(now.timestamp(), Ordering::AcqRel);
        let active = until >= now.timestamp();
        self.update_is_pro_status(active)?;
        Ok(active)
    }

    pub(crate) fn update_is_pro_status(&self, is_pro: bool) -> Result<(), String> {
        self.db
            .update_pro_status(is_pro)
            .map_err(|error| error.to_string())?;
        self.cached_pro.store(
            is_pro && self.cached_until.load(Ordering::Acquire) >= Utc::now().timestamp(),
            Ordering::Release,
        );
        Ok(())
    }

    pub async fn update_account(
        &self,
        api_key: Option<ApiKey>,
        last_pro_check: Option<String>,
    ) -> Result<(), String> {
        let api_key = api_key.map(|key| key.into_inner());
        let mut account = self.account.write().await;
        let updated = self
            .db
            .update_account(api_key, last_pro_check)
            .map_err(|error| error.to_string())?;
        *account = updated;
        self.cached_pro.store(false, Ordering::Release);
        self.cached_until.store(0, Ordering::Release);
        Ok(())
    }

    pub async fn set_api_key(&self, api_key: Option<ApiKey>) -> Result<(), String> {
        let api_key = api_key.map(|key| key.into_inner());
        let mut account = self.account.write().await;
        let updated = self
            .db
            .set_api_key(api_key)
            .map_err(|error| error.to_string())?;
        *account = updated;
        self.cached_pro.store(false, Ordering::Release);
        self.cached_until.store(0, Ordering::Release);
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

#[cfg(test)]
mod cached_pro_tests {
    use super::*;

    #[test]
    fn explicit_refresh_can_detect_an_upgrade_before_the_proxy_cache_expires() {
        let now = Local::now();
        let forty_seconds_ago = now - chrono::Duration::seconds(40);
        assert!(pro_check_is_stale(
            Some(forty_seconds_ago),
            chrono::Duration::seconds(30),
            now,
        ));
        assert!(!pro_check_is_stale(
            Some(forty_seconds_ago),
            chrono::Duration::days(1),
            now,
        ));
        assert!(!pro_check_is_stale(
            Some(now),
            chrono::Duration::seconds(30),
            now,
        ));
        assert!(pro_check_is_stale(None, chrono::Duration::seconds(30), now,));
    }

    #[tokio::test]
    async fn initial_account_snapshot_uses_local_entitlement_and_requires_a_saved_key() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(EngineDB::from(&dir.path().join("snapshot.sqlite")).unwrap());
        let manager = AccountManager::from(db).unwrap();
        manager.update_is_pro_status(true).unwrap();

        assert!(!manager.get_cached_pro_entitlement().await);
        manager
            .update_account(
                Some(ApiKey::try_new("local-key".to_string()).unwrap()),
                None,
            )
            .await
            .unwrap();
        assert!(!manager.get_cached_pro_entitlement().await);
        manager.update_account(None, None).await.unwrap();
        assert!(!manager.get_cached_pro_entitlement().await);
    }

    #[test]
    fn cached_entitlement_is_shared_between_clones_and_reloaded_on_restart() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(EngineDB::from(&dir.path().join("account.sqlite")).unwrap());
        let manager = AccountManager::from(db.clone()).unwrap();
        let clone = manager.clone();
        assert!(!clone.get_cached_pro_status());

        manager.update_is_pro_status(true).unwrap();
        assert!(!clone.get_cached_pro_status());
        // A legacy settings.is_pro flag is not valid proof of entitlement.
        assert!(
            !AccountManager::from(db.clone())
                .unwrap()
                .get_cached_pro_status()
        );

        manager.update_is_pro_status(false).unwrap();
        assert!(!clone.get_cached_pro_status());
    }
}

#[cfg(test)]
mod license_import_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn test_public_key() -> String {
        use base64::Engine as _;
        let signing = ed25519_dalek::SigningKey::from_bytes(&[17; 32]);
        base64::engine::general_purpose::STANDARD.encode(signing.verifying_key().as_bytes())
    }

    // Local synthetic HTTP responses, never real site accounts or credentials.
    async fn mock_entitlement_server(status: &str, body: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let status = status.to_owned();
        let body = body.to_owned();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request).await.unwrap();
            let reply = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(reply.as_bytes()).await.unwrap();
        });
        format!("http://{address}")
    }

    fn test_manager() -> (tempfile::TempDir, AccountManager) {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(EngineDB::from(&dir.path().join("license.sqlite")).unwrap());
        (dir, AccountManager::from(db).unwrap())
    }

    #[tokio::test]
    async fn rejected_key_preserves_the_previous_login() {
        let (_dir, manager) = test_manager();
        manager
            .update_account(
                Some(ApiKey::try_new("previous-working-key".to_owned()).unwrap()),
                None,
            )
            .await
            .unwrap();
        let endpoint =
            mock_entitlement_server("401 Unauthorized", r#"{"message":"Invalid API key."}"#).await;
        let error = manager
            .import_api_key_from(
                ApiKey::try_new("rejected-key".to_owned()).unwrap(),
                &endpoint,
            )
            .await
            .unwrap_err();
        assert!(error.contains("HTTP 401"));
        assert_eq!(
            manager.get_api_key().await.unwrap().unwrap().as_ref(),
            "previous-working-key"
        );
    }

    #[tokio::test]
    async fn unsigned_pro_lease_cannot_replace_the_previous_login() {
        let (_dir, manager) = test_manager();
        manager
            .update_account(
                Some(ApiKey::try_new("previous-working-key".to_owned()).unwrap()),
                None,
            )
            .await
            .unwrap();
        let response = format!(
            r#"{{"token":"invalid-token","publicKey":"{}"}}"#,
            test_public_key()
        );
        let endpoint = mock_entitlement_server("200 OK", &response).await;
        let error = manager
            .import_api_key_from(
                ApiKey::try_new("candidate-key".to_owned()).unwrap(),
                &endpoint,
            )
            .await
            .unwrap_err();
        assert!(error.contains("invalid entitlement"));
        assert_eq!(
            manager.get_api_key().await.unwrap().unwrap().as_ref(),
            "previous-working-key"
        );
    }

    #[tokio::test]
    async fn signed_pro_lease_survives_restart_without_build_time_key() {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        use ed25519_dalek::{Signer, SigningKey};
        use sha2::{Digest, Sha256};
        let (dir, manager) = test_manager();
        let signing = SigningKey::from_bytes(&[17; 32]);
        let key = "verified-pro-key";
        let now = Utc::now().timestamp();
        let header = URL_SAFE_NO_PAD.encode(
            serde_json::json!({"alg":"EdDSA","typ":"ACE-ENTITLEMENT","kid":"ace-v1"}).to_string(),
        );
        let payload = URL_SAFE_NO_PAD.encode(
            serde_json::json!({
                "iss":"aceblocker.com", "sub":"test-user",
                "key_sha256": hex::encode(Sha256::digest(key.as_bytes())),
                "iat": now, "exp": now + 86400,
                "offline_until": now + 86400 + signed_entitlement::OFFLINE_GRACE_SECONDS,
                "features":["custom_site_packs","premium_site_packs","resistance_mode","cloud_backup"]
            }).to_string()
        );
        let message = format!("{header}.{payload}");
        let signature = URL_SAFE_NO_PAD.encode(signing.sign(message.as_bytes()).to_bytes());
        let response = serde_json::json!({
            "token": format!("{message}.{signature}"),
            "publicKey": test_public_key()
        })
        .to_string();
        let endpoint = mock_entitlement_server("200 OK", &response).await;
        manager
            .import_api_key_from(ApiKey::try_new(key.to_owned()).unwrap(), &endpoint)
            .await
            .unwrap();
        assert!(manager.get_cached_pro_status());
        let db = Arc::new(EngineDB::from(&dir.path().join("license.sqlite")).unwrap());
        let restarted = AccountManager::from(db).unwrap();
        assert!(restarted.get_cached_pro_status());
    }

    #[tokio::test]
    async fn authenticated_free_account_can_be_imported_without_pro_grant() {
        let (_dir, manager) = test_manager();
        let response = format!(r#"{{"token":null,"publicKey":"{}"}}"#, test_public_key());
        let endpoint = mock_entitlement_server("200 OK", &response).await;
        manager
            .import_api_key_from(
                ApiKey::try_new("valid-free-key".to_owned()).unwrap(),
                &endpoint,
            )
            .await
            .unwrap();
        assert_eq!(
            manager.get_api_key().await.unwrap().unwrap().as_ref(),
            "valid-free-key"
        );
        assert!(!manager.get_cached_pro_status());
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
    // First pass immediately (SVC-15-style): refresh an authenticated
    // account when its last successful verification is older than thirty
    // seconds, while retaining the lazy 24-hour window for proxy checks.
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
