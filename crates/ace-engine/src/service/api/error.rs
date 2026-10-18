use crate::manager::NetworkManagerOpErr;
use thiserror::Error;

/// Error envelope contract.
///
/// Failed remoc RPC operations return `RpcError` containing numeric `status`,
/// stable `code` and human-readable `error`. Callers must branch on the code.
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Forbidden: {0}")]
    Forbidden(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Ace Blocker Pro plan required")]
    ProPlanRequired,
    #[error("Internal error: {0}")]
    Internal(String),
    /// The item cannot be deleted because it is in use by other resources.
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Locked: {0}")]
    Locked(String),
    /// Resistance Mode action refused because the resistance lock is active.
    #[error("Resistance engine is locked: {0}")]
    ResistanceLocked(String),
    /// Resistance Mode only supports the given lock states (inactive, active_until_lock_end).
    #[error("Unsupported resistance lock state: {0}")]
    UnsupportedLockState(String),
    /// Pro verification failed (missing/invalid API key or the server reported
    /// the account is not entitled). Kept distinct from a generic `forbidden`
    /// so the app can route users to login/upgrade.
    #[error("Pro status check failed: {0}")]
    ProStatusCheckFailed(String),
    /// The resistance script catalog is missing, corrupt, or failed to load.
    #[error("Resistance scripts unavailable: {0}")]
    ScriptsUnavailable(String),
    /// Downloading/verifying/extracting resistance scripts failed.
    #[error("Resistance script download failed: {0}")]
    ScriptDownloadFailed(String),
    /// The request body failed to parse (serde/validation).
    #[error("Invalid request body: {0}")]
    InvalidRequestBody(String),
    /// A required capability token is missing or invalid.
    #[error("Capability token required: {0}")]
    CapabilityRequired(String),
}

impl ApiError {
    /// Stable machine-readable code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            ApiError::NotFound(_) => "not_found",
            ApiError::Forbidden(_) => "forbidden",
            ApiError::BadRequest(_) => "bad_request",
            ApiError::ProPlanRequired => "pro_plan_required",
            ApiError::Internal(_) => "internal",
            ApiError::Conflict(_) => "conflict",
            ApiError::Unauthorized(_) => "unauthorized",
            ApiError::Locked(_) => "locked",
            ApiError::ResistanceLocked(_) => "resistance_locked",
            ApiError::UnsupportedLockState(_) => "unsupported_lock_state",
            ApiError::ProStatusCheckFailed(_) => "pro_status_check_failed",
            ApiError::ScriptsUnavailable(_) => "scripts_unavailable",
            ApiError::ScriptDownloadFailed(_) => "script_download_failed",
            ApiError::InvalidRequestBody(_) => "invalid_request_body",
            ApiError::CapabilityRequired(_) => "capability_required",
        }
    }

    pub(crate) fn status(&self) -> u16 {
        match self {
            ApiError::NotFound(_) => 404,
            ApiError::Forbidden(_) => 403,
            ApiError::Conflict(_) => 409,
            ApiError::BadRequest(_) => 400,
            ApiError::ProPlanRequired => 403,
            ApiError::Internal(_) => 500,
            ApiError::Unauthorized(_) => 401,
            ApiError::Locked(_) => 423,
            ApiError::ResistanceLocked(_) => 403,
            ApiError::UnsupportedLockState(_) => 400,
            ApiError::ProStatusCheckFailed(_) => 403,
            ApiError::ScriptsUnavailable(_) => 503,
            ApiError::ScriptDownloadFailed(_) => 502,
            ApiError::InvalidRequestBody(_) => 400,
            ApiError::CapabilityRequired(_) => 401,
        }
    }

    pub(crate) fn message(&self) -> String {
        match self {
            ApiError::NotFound(msg) => msg.clone(),
            ApiError::Forbidden(msg) => msg.clone(),
            ApiError::Conflict(msg) => msg.clone(),
            ApiError::BadRequest(msg) => msg.clone(),
            ApiError::ProPlanRequired => "Ace Blocker Pro plan required".to_string(),
            ApiError::Internal(msg) => msg.clone(),
            ApiError::Unauthorized(msg) => msg.clone(),
            ApiError::Locked(msg) => msg.clone(),
            ApiError::ResistanceLocked(msg) => msg.clone(),
            ApiError::UnsupportedLockState(msg) => msg.clone(),
            ApiError::ProStatusCheckFailed(msg) => msg.clone(),
            ApiError::ScriptsUnavailable(msg) => msg.clone(),
            ApiError::ScriptDownloadFailed(msg) => msg.clone(),
            ApiError::InvalidRequestBody(msg) => msg.clone(),
            ApiError::CapabilityRequired(msg) => msg.clone(),
        }
    }
}

impl From<NetworkManagerOpErr> for ApiError {
    fn from(error: NetworkManagerOpErr) -> Self {
        match error {
            NetworkManagerOpErr::NetworkBlockNotFound { id } => {
                ApiError::NotFound(format!("Network block {} not found", id))
            }
            NetworkManagerOpErr::NetworkBlockIsLocked { id } => {
                ApiError::Forbidden(format!("Network block {} is locked", id))
            }
            NetworkManagerOpErr::ScheduleEntryNotFound { id } => {
                ApiError::NotFound(format!("Schedule entry {} not found", id))
            }
            NetworkManagerOpErr::SitePackIsLocked { id } => {
                ApiError::Locked(format!("Site pack {} is locked", id))
            }
            NetworkManagerOpErr::SitePackBusy { id } => {
                ApiError::Conflict(format!("Site pack {} is currently syncing", id))
            }
            NetworkManagerOpErr::SitePackNotFound { id } => {
                ApiError::NotFound(format!("Site pack {} not found", id))
            }
            NetworkManagerOpErr::DatabaseError { message } => {
                ApiError::Internal(format!("Database error: {}", message))
            }
            NetworkManagerOpErr::InternalError { message } => {
                ApiError::Internal(format!("Internal error: {}", message))
            }
            NetworkManagerOpErr::SitePackNotSynced { id } => {
                ApiError::BadRequest(format!("Site pack {} is not synced", id))
            }
            NetworkManagerOpErr::RwLockPoison => ApiError::Internal("Lock poison".to_string()),
            NetworkManagerOpErr::ProPlanRequired => ApiError::ProPlanRequired,
            NetworkManagerOpErr::ProStatusCheckFailed { message } => {
                ApiError::ProStatusCheckFailed(message)
            }
            NetworkManagerOpErr::ResistanceModeIsLocked => {
                ApiError::ResistanceLocked("Resistance engine is locked".to_string())
            }
            NetworkManagerOpErr::UnsupportedLockState { state } => {
                ApiError::UnsupportedLockState(format!(
                    "Resistance Mode supports only 'inactive' and 'active_until_lock_end' lock states, got '{state}'"
                ))
            }
            NetworkManagerOpErr::ScriptsUnavailable { message } => {
                ApiError::ScriptsUnavailable(message)
            }
            NetworkManagerOpErr::ScriptDownloadFailed { message } => {
                ApiError::ScriptDownloadFailed(message)
            }
            NetworkManagerOpErr::InvalidBackupVersion { message } => ApiError::BadRequest(message),
        }
    }
}

impl From<String> for ApiError {
    fn from(error: String) -> Self {
        ApiError::Internal(error)
    }
}

impl From<&str> for ApiError {
    fn from(error: &str) -> Self {
        ApiError::Internal(error.to_string())
    }
}
