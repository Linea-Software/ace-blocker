// The inner module is intentionally named after its directory
// (`importer/importer.rs`); the lint is a false positive here.
#![expect(clippy::module_inception)]

pub mod importer;
pub mod stream;

pub use importer::Importer;
pub use stream::{SitePackCreationStatus, SitePackImportPhase, SitePackImportStream};

use nutype::nutype;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use thiserror::Error;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(
    Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Display, EnumString, ToSchema, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum SitePackFormat {
    #[strum(serialize = "standard")]
    Standard,
    #[strum(serialize = "wildcard")]
    WildCardDomains,
    /// Let the engine detect the format by sampling the file content.
    #[serde(rename = "auto")]
    #[strum(serialize = "auto")]
    Auto,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Display, EnumString, ToSchema, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum SitePackSourceType {
    Url,
    File,
}

#[nutype(
    derive(
        Debug, Clone, PartialEq, Eq, Hash, From, Into, Display, Deref, AsRef, Serialize,
        Deserialize, JsonSchema,
    ),
    derive_unchecked(ToSchema)
)]
pub struct SitePackId(String);

#[nutype(derive(Debug, Clone, PartialEq, Eq, From, Into, Display, Deref, AsRef))]
pub struct SitePackSourceId(Uuid);

#[derive(Debug, Clone)]
pub struct SitePackSource {
    pub id: SitePackSourceId,
    pub site_pack_id: SitePackId,
    pub source: String,
    pub source_type: SitePackSourceType,
    pub format: SitePackFormat,
}

#[derive(Debug, Clone, Error)]
pub enum SitePackImportError {
    #[error("Failed to download the source: {message}")]
    DownloadFailure { message: String },
    #[error("Failed to read or write a file: {message}")]
    IoFailure { message: String },
    #[error("Failed to parse a line of the source: {message}")]
    ParsingFailure { message: String },
    #[error("Failed to access the site pack database: {message}")]
    DatabaseFailure { message: String },
    #[error("Failed to restore the site pack from backup: {message}")]
    BackupFailure { message: String },
}

impl From<crate::db::error::DBError> for SitePackImportError {
    fn from(error: crate::db::error::DBError) -> Self {
        SitePackImportError::DatabaseFailure {
            message: error.to_string(),
        }
    }
}

impl From<ace_domain_blocklist::DomainLineParsingError> for SitePackImportError {
    fn from(error: ace_domain_blocklist::DomainLineParsingError) -> Self {
        SitePackImportError::ParsingFailure {
            message: error.to_string(),
        }
    }
}
