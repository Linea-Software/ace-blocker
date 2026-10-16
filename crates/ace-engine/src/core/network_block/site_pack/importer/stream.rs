use std::pin::Pin;

use futures::Stream;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use strum::AsRefStr;
use strum::Display;
use utoipa::ToSchema;

use crate::core::{SitePackId, common::percentage::Percentage};

/// Represents the current phase of a site pack import/resync operation
#[derive(
    Serialize, Deserialize, JsonSchema, Display, AsRefStr, Clone, Debug, ToSchema, PartialEq,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SitePackImportPhase {
    /// Initial setup and preparation
    Initializing,
    /// Downloading file from remote server
    Downloading,
    /// Extracting downloaded archive
    Extracting,
    /// Parsing and importing data from source files
    Importing,
    /// Restoring database from backup
    Restoring,
    /// Finalizing and cleanup
    Finalizing,
    /// Operation completed successfully
    Completed,
}

#[derive(
    Serialize, Deserialize, JsonSchema, Display, AsRefStr, Clone, Debug, ToSchema, PartialEq,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SitePackCreationStatus {
    Started {
        site_pack_id: SitePackId,
        progress: Percentage,
        phase: SitePackImportPhase,
    },
    InProgress {
        site_pack_id: SitePackId,
        progress: Percentage,
        phase: SitePackImportPhase,
    },
    Completed {
        site_pack_id: SitePackId,
        progress: Percentage,
        phase: SitePackImportPhase,
    },
    Failed {
        error: String,
        phase: Option<SitePackImportPhase>,
    },
}

/// A stream of [`SitePackCreationStatus`] events emitted during a site pack
/// import or resync operation.
///
/// This is a type alias for a boxed, pinned, sendable stream.  Callers can
/// use it directly with `StreamExt` combinators or forward it to an SSE
/// endpoint.
pub type SitePackImportStream<'a> = Pin<Box<dyn Stream<Item = SitePackCreationStatus> + Send + 'a>>;
