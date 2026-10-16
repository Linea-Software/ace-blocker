//! Small, versioned protocol shared by the engine and the userspace watchdog.
//!
//! The channel intentionally carries only availability metadata. Application
//! secrets, URLs, domains, user activity, and certificate material do not
//! belong here.

use nutype::nutype;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// Uniquely identifies one engine process lifetime, not an installation.
#[nutype(derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Into, Display, Serialize, Deserialize))]
pub struct EngineInstanceId(Uuid);

/// Per-ping challenge echoed by the engine to bind each pong to its ping.
#[nutype(derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Into, Display, Serialize, Deserialize))]
pub struct WatchdogNonce(Uuid);

pub const ENGINE_SERVICE_NAME: &str = "AceEngine";
pub const WATCHDOG_SERVICE_NAME: &str = "AceEngineWatchdog";
pub const WATCHDOG_PIPE_NAME: &str = r"\\.\pipe\AceBlocker\WatchdogV1";

pub const LINUX_ENGINE_SERVICE_NAME: &str = "ace-engine.service";
pub const LINUX_WATCHDOG_SERVICE_NAME: &str = "ace-watchdog.service";
pub const LINUX_WATCHDOG_RUNTIME_DIR: &str = "/run/ace-blocker";
pub const LINUX_WATCHDOG_SOCKET_PATH: &str = "/run/ace-blocker/watchdog.sock";
pub const LINUX_WATCHDOG_PAUSE_PATH: &str = "/run/ace-blocker/watchdog-heartbeat.pause";

pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(3);
pub const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(2);
pub const HEARTBEAT_FAILURE_THRESHOLD: u32 = 3;
pub const ENGINE_STARTUP_GRACE: Duration = Duration::from_secs(20);
pub const DEFAULT_MAINTENANCE_TTL: Duration = Duration::from_secs(180);
pub const MIN_MAINTENANCE_TTL: Duration = Duration::from_secs(120);
pub const MAX_MAINTENANCE_TTL: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceReason {
    UnlockedShutdown,
}

/// Engine heartbeats and bounded maintenance requests.
/// Maintenance is requested only after the authorized shutdown lock checks;
/// the watchdog does not independently evaluate the blocking policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineToWatchdog {
    Hello {
        engine_instance_id: EngineInstanceId,
    },
    Pong {
        sequence: u64,
        nonce: WatchdogNonce,
        engine_instance_id: EngineInstanceId,
    },
    BeginMaintenance {
        engine_instance_id: EngineInstanceId,
        ttl_ms: u64,
        reason: MaintenanceReason,
    },
}

/// Watchdog challenges and responses to maintenance requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WatchdogToEngine {
    Ping { sequence: u64, nonce: WatchdogNonce },
    MaintenanceAccepted { ttl_ms: u64 },
    MaintenanceRejected { reason: String },
}

/// Clamps a maintenance lease to the shared lower and upper protocol bounds.
pub fn clamp_maintenance_ttl(requested: Duration) -> Duration {
    requested.clamp(MIN_MAINTENANCE_TTL, MAX_MAINTENANCE_TTL)
}

/// Bounded recovery schedule used after watchdog-level recovery attempts.
///
/// SCM handles the first service failures. This schedule prevents the
/// watchdog itself from creating a rapid respawn loop when recovery keeps
/// failing.
pub fn recovery_delay(failure_count: u32) -> Duration {
    match failure_count {
        0 | 1 => Duration::from_secs(1),
        2 => Duration::from_secs(2),
        3 => Duration::from_secs(5),
        4 => Duration::from_secs(10),
        5 => Duration::from_secs(30),
        _ => Duration::from_secs(60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maintenance_ttl_is_bounded() {
        assert_eq!(
            clamp_maintenance_ttl(Duration::from_secs(5)),
            MIN_MAINTENANCE_TTL
        );
        assert_eq!(
            clamp_maintenance_ttl(Duration::from_secs(240)),
            Duration::from_secs(240)
        );
        assert_eq!(
            clamp_maintenance_ttl(Duration::from_secs(900)),
            MAX_MAINTENANCE_TTL
        );
    }

    #[test]
    fn recovery_delay_caps_at_one_minute() {
        assert_eq!(recovery_delay(1), Duration::from_secs(1));
        assert_eq!(recovery_delay(3), Duration::from_secs(5));
        assert_eq!(recovery_delay(100), Duration::from_secs(60));
    }

    #[test]
    fn heartbeat_message_round_trips_as_json() {
        let message = EngineToWatchdog::Pong {
            sequence: 42,
            nonce: WatchdogNonce::from(Uuid::new_v4()),
            engine_instance_id: EngineInstanceId::from(Uuid::new_v4()),
        };
        let encoded = serde_json::to_string(&message).unwrap();
        let decoded: EngineToWatchdog = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, message);
    }
}
