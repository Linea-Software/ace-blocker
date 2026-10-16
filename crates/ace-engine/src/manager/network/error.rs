use crate::core::{NetworkBlockId, ScheduleEntryId, SitePackId};
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum NetworkManagerOpErr {
    #[error("Net list with ID {id} not found.")]
    NetworkBlockNotFound { id: NetworkBlockId },

    #[error("Net list with ID {id} is locked and cannot be modified.")]
    NetworkBlockIsLocked { id: NetworkBlockId },

    #[error("Schedule entry with ID {id} not found.")]
    ScheduleEntryNotFound { id: ScheduleEntryId },

    #[error("Site pack with ID {id} is locked and cannot be removed.")]
    SitePackIsLocked { id: SitePackId },

    #[error("Site pack with ID {id} is currently syncing; retry after it finishes.")]
    SitePackBusy { id: SitePackId },

    #[error("A database error occurred: {message}")]
    DatabaseError { message: String },

    #[error("An internal error occurred: {message}")]
    InternalError { message: String },

    #[error("Site pack with ID {id} not found.")]
    SitePackNotFound { id: SitePackId },

    #[error("Site pack with ID {id} is not synced and cannot be added to network block.")]
    SitePackNotSynced { id: SitePackId },

    #[error("Poisoned RwLock.")]
    RwLockPoison,

    #[error("Ace Blocker Pro plan required.")]
    ProPlanRequired,

    #[error("Pro status check failed: {message}")]
    ProStatusCheckFailed { message: String },

    #[error("Resistance engine is locked and cannot be modified.")]
    ResistanceModeIsLocked,

    #[error("Resistance mode does not support the lock state '{state}'.")]
    UnsupportedLockState { state: String },

    #[error("The resistance script catalog is unavailable: {message}")]
    ScriptsUnavailable { message: String },

    #[error("Failed to download resistance scripts: {message}")]
    ScriptDownloadFailed { message: String },

    #[error("Invalid backup version: {message}")]
    InvalidBackupVersion { message: String },
}

// Temporary alias to avoid breaking everything at once if needed,
// but I will try to fix all call sites.
pub type NetworkMangerOpErr = NetworkManagerOpErr;

impl From<crate::db::DBError> for NetworkManagerOpErr {
    fn from(error: crate::db::DBError) -> Self {
        NetworkManagerOpErr::DatabaseError {
            message: error.to_string(),
        }
    }
}
