use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use utoipa::ToSchema;

/// Represents the resistance intensity levels.
#[derive(
    Debug,
    Serialize,
    Deserialize,
    JsonSchema,
    ToSchema,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumString,
    Display,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[derive(Default)]
pub enum PunishmentIntensity {
    #[default]
    None,
    Low,
    Medium,
    High,
    Extreme,
}

/// The intensity scale is a static, user-configured level. There is no
/// automatic escalation/de-escalation ladder: the dynamic mode (and the whole
/// tracking pipeline that fed it) has been removed (MGR-24).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, ToSchema, PartialEq)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum PunishmentMode {
    Static {
        #[serde(default)]
        intensity: PunishmentIntensity,
    },
}

impl Default for PunishmentMode {
    fn default() -> Self {
        PunishmentMode::Static {
            intensity: PunishmentIntensity::None,
        }
    }
}

/// Represents the lock state of the resistance mode.
///
/// NOTE: the dedicated `PunishmentLock` type was removed as dead code
/// (MGR-24); the resistance mode uses the core `Lock` type (`LockState`)
/// instead, which is what the API and the uninstall gate read.
///
/// Encapsulates the complete punishment state including intensity mode.
#[derive(Debug, Clone)]
pub struct Punishment {
    pub mode: PunishmentMode,
    pub static_intensity: PunishmentIntensity,
}

impl Punishment {
    /// Creates a new punishment instance with static state and no lock.
    pub fn new_static(intensity: PunishmentIntensity) -> Self {
        Punishment {
            mode: PunishmentMode::Static { intensity },
            static_intensity: intensity,
        }
    }

    /// Returns the current active intensity.
    pub fn get_intensity(&self) -> PunishmentIntensity {
        self.static_intensity
    }

    /// Sets the intensity for the current mode.
    /// Should only be called by manual overrides or after a mode switch.
    pub fn set_intensity(&mut self, intensity: PunishmentIntensity) {
        self.static_intensity = intensity;
    }

    /// Returns true if in static mode.
    pub fn is_static(&self) -> bool {
        matches!(self.mode, PunishmentMode::Static { .. })
    }
}

impl Default for Punishment {
    fn default() -> Self {
        Punishment::new_static(PunishmentIntensity::None)
    }
}
