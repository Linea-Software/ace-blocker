//! Authenticated UTC for security-sensitive lock decisions.
//!
//! Roughtime's pinned Ed25519 root verifies delegation, response signature and
//! the per-request nonce's Merkle inclusion proof in the \`roughtime\` crate.
//! OS wall time is only compared for anomaly diagnostics, never for authority.
//! A persisted high-water mark is a rollback floor, NOT a reboot-spanning clock.

use chrono::{DateTime, Local, Utc};
use roughtime::client::{RoughtimeClientConfig, TokioClient};
use roughtime::servers::RoughtimeServer;
use roughtime::verify::VerifiedTime;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

const CLOUDFLARE: RoughtimeServer = RoughtimeServer {
    name: "cloudflare",
    host: "roughtime.cloudflare.com",
    port: 2003,
    // Published at https://developers.cloudflare.com/time-services/roughtime/usage/
    // The beta root can rotate: update only as a reviewed release change.
    pubkey_base64: "0GD7c3yP8xEc4Zl2zeuN2SlLvDVVocjsPSL8/Rl/7zg=",
};
const REFRESH: Duration = Duration::from_secs(60 * 60);
const MAX_RADIUS: u32 = 120_000_000;
const MAX_RTT: Duration = Duration::from_secs(8);
// Upper bound for plausible oscillator drift (500 parts per million).
const DRIFT_PPM: i128 = 500;

#[cfg(windows)]
#[derive(Debug, Clone, Copy)]
struct Monotonic(u64);

#[cfg(not(windows))]
#[derive(Debug, Clone, Copy)]
struct Monotonic(std::time::Instant);

impl Monotonic {
    fn now() -> Self {
        #[cfg(windows)]
        {
            // Unlike a process Instant that may exclude suspension, uptime
            // includes sleep/hibernation on Windows; never persist this tick.
            Self(unsafe { windows::Win32::System::SystemInformation::GetTickCount64() })
        }
        #[cfg(not(windows))]
        {
            Self(std::time::Instant::now())
        }
    }

    fn elapsed(self) -> Option<Duration> {
        #[cfg(windows)]
        {
            Monotonic::now()
                .0
                .checked_sub(self.0)
                .map(Duration::from_millis)
        }
        #[cfg(not(windows))]
        {
            Monotonic::now().0.checked_duration_since(self.0)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeQuality {
    Unavailable,
    Verified,
    Stale,
}

#[derive(Debug, Clone, Copy)]
pub struct UtcInterval {
    pub earliest: DateTime<Utc>,
    pub latest: DateTime<Utc>,
    pub quality: TimeQuality,
}

/// Pure diagnostic predicate; never feeds the policy clock.
fn wall_clock_differs(verified: UtcInterval, observed: DateTime<Utc>) -> bool {
    observed < verified.earliest - chrono::Duration::minutes(1)
        || observed > verified.latest + chrono::Duration::minutes(1)
}

#[derive(Debug, Clone, Copy)]
struct IntervalMicros {
    low: i64,
    high: i64,
}

fn interval_at(anchor: IntervalMicros, elapsed: Duration) -> Option<IntervalMicros> {
    let elapsed_us = i128::try_from(elapsed.as_micros()).ok()?;
    let drift_us = elapsed_us.checked_mul(DRIFT_PPM)? / 1_000_000;
    Some(IntervalMicros {
        low: i64::try_from(i128::from(anchor.low) + elapsed_us - drift_us).ok()?,
        high: i64::try_from(i128::from(anchor.high) + elapsed_us + drift_us).ok()?,
    })
}

fn sample_interval(sample: VerifiedTime, rtt: Duration) -> Option<IntervalMicros> {
    if sample.radius_micros > MAX_RADIUS || rtt > MAX_RTT {
        return None;
    }
    let midpoint = i64::try_from(sample.midpoint_micros).ok()?;
    let radius = i64::from(sample.radius_micros);
    let transit = i64::try_from(rtt.as_micros()).ok()?;
    Some(IntervalMicros {
        low: midpoint.checked_sub(radius)?,
        // Response was generated *within* the request/response interval.
        // At receive time at most the complete RTT has elapsed.
        high: midpoint.checked_add(radius)?.checked_add(transit)?,
    })
}

#[derive(Debug)]
struct Anchor {
    interval: IntervalMicros,
    monotonic: Monotonic,
}

#[derive(Debug)]
struct State {
    anchor: Option<Anchor>,
    persisted_floor_us: i64,
}

#[derive(Debug)]
pub struct TrustedClock {
    path: PathBuf,
    state: Mutex<State>,
    single_flight: tokio::sync::Mutex<()>,
    wall_anomaly_reported: AtomicBool,
}

impl TrustedClock {
    pub fn new(path: PathBuf) -> Self {
        let persisted_floor_us = match std::fs::read_to_string(&path) {
            Ok(raw) => match raw.trim().parse::<i64>() {
                Ok(value) if value >= 0 => value,
                _ => {
                    tracing::warn!(
                        "Invalid trusted-time high-water mark; fresh Roughtime required"
                    );
                    0
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
            Err(error) => {
                tracing::warn!("Cannot read trusted-time high-water mark: {error}");
                0
            }
        };
        Self {
            path,
            state: Mutex::new(State {
                anchor: None,
                persisted_floor_us,
            }),
            single_flight: tokio::sync::Mutex::new(()),
            wall_anomaly_reported: AtomicBool::new(false),
        }
    }

    pub fn now(&self) -> Option<UtcInterval> {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let anchor = state.anchor.as_ref()?;
        let elapsed = anchor.monotonic.elapsed()?;
        let range = interval_at(anchor.interval, elapsed)?;
        let earliest = DateTime::from_timestamp_micros(range.low)?;
        let latest = DateTime::from_timestamp_micros(range.high)?;
        Some(UtcInterval {
            earliest,
            latest,
            quality: if elapsed > REFRESH * 2 {
                TimeQuality::Stale
            } else {
                TimeQuality::Verified
            },
        })
    }

    fn accept(&self, verified: VerifiedTime, rtt: Duration) -> Result<(), String> {
        let range = sample_interval(verified, rtt).ok_or_else(|| {
            "Roughtime uncertainty or network latency exceeds safety limit".to_string()
        })?;
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        // Two processes can synchronize at once (engine and installer).
        // Serialize the update across processes; re-read the durable floor
        // while holding the advisory file lock to avoid rollback races.
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let lock_path = self.path.with_extension("lock");
        let lock_file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|e| e.to_string())?;
        lock_file.lock().map_err(|e| e.to_string())?;
        let persisted_floor = match std::fs::read_to_string(&self.path) {
            Ok(raw) => raw
                .trim()
                .parse::<i64>()
                .ok()
                .filter(|v| *v >= 0)
                .ok_or_else(|| {
                    "Invalid trusted-time floor file; refusing synchronization".to_string()
                })?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
            Err(error) => return Err(error.to_string()),
        };
        let prior_floor = state.persisted_floor_us.max(persisted_floor);
        if range.high < prior_floor {
            return Err("Authenticated sample is older than persisted time floor".into());
        }
        let floor = range.low.max(prior_floor);
        // Replace via a same-directory temp file instead of truncating the
        // existing floor, so an interrupted write cannot corrupt it.
        let temporary_path = self
            .path
            .with_extension(format!("pending-{}", uuid::Uuid::new_v4()));
        let mut temporary = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|e| e.to_string())?;
        use std::io::Write;
        writeln!(temporary, "{floor}").map_err(|e| e.to_string())?;
        temporary.sync_all().map_err(|e| e.to_string())?;
        drop(temporary);
        if let Err(error) = std::fs::rename(&temporary_path, &self.path) {
            let _ = std::fs::remove_file(&temporary_path);
            return Err(error.to_string());
        }
        state.persisted_floor_us = floor;
        state.anchor = Some(Anchor {
            interval: IntervalMicros {
                low: range.low.max(floor),
                high: range.high.max(floor),
            },
            monotonic: Monotonic::now(),
        });
        Ok(())
    }

    /// One query (up to two resolved IPs), never per lock / domain / request.
    pub async fn synchronize(&self) -> Result<(), String> {
        let _guard = self.single_flight.lock().await;
        let addresses = tokio::time::timeout(
            Duration::from_secs(3),
            tokio::net::lookup_host((CLOUDFLARE.host, CLOUDFLARE.port)),
        )
        .await
        .map_err(|_| "Roughtime DNS timeout".to_string())?
        .map_err(|error| error.to_string())?;
        // This client version currently binds a v4 UDP socket internally.
        // Never submit IPv6 addresses that it cannot contact.
        let resolved: Vec<_> = addresses
            .filter(|addr| addr.is_ipv4())
            .take(2)
            .map(|addr| (CLOUDFLARE, addr.ip()))
            .collect();
        if resolved.is_empty() {
            return Err("Roughtime hostname returned no addresses".into());
        }
        let client = TokioClient::new(RoughtimeClientConfig {
            timeout: Duration::from_secs(4),
            retries: 1,
            max_spread: Duration::from_secs(15),
        });
        let start = Monotonic::now();
        let result = client.query(&resolved).await.map_err(|e| e.to_string())?;
        if let Some(spread) = result.spread_warning {
            return Err(format!(
                "Roughtime authenticated replies disagree by {spread:?}"
            ));
        }
        let rtt = start
            .elapsed()
            .ok_or_else(|| "Monotonic tick regressed".to_string())?;
        self.accept(result.time, rtt)?;
        self.check_wall_clock_anomaly();
        Ok(())
    }

    /// Check OS wall-clock manipulation without ever promoting that clock to
    /// an authority. Deduplicated warnings avoid log storms on a bad clock.
    fn check_wall_clock_anomaly(&self) {
        let Some(interval) = self.now() else {
            return;
        };
        let anomalous = wall_clock_differs(interval, Utc::now());
        if anomalous && !self.wall_anomaly_reported.swap(true, Ordering::AcqRel) {
            tracing::warn!(
                "Windows/system wall clock differs from authenticated time; lock decisions ignore it"
            );
        } else if !anomalous {
            self.wall_anomaly_reported.store(false, Ordering::Release);
        }
    }

    /// An internal minute wakeup detects suspend/resume by Windows uptime;
    /// it does NOT send a network request until the hourly due time or retry.
    pub async fn run_refresh_loop(self: Arc<Self>) {
        let mut failures = 0u32;
        let mut last_attempt: Option<Monotonic> = None;
        let mut jitter = Duration::ZERO;
        loop {
            let interval = if failures == 0 {
                REFRESH
            } else {
                Duration::from_secs(60u64.saturating_mul(1u64 << failures.min(6))).min(REFRESH)
                    + jitter
            };
            if last_attempt
                .is_none_or(|attempt| attempt.elapsed().is_some_and(|age| age >= interval))
            {
                // Record the attempt before I/O; even a DNS failure is rate-limited.
                last_attempt = Some(Monotonic::now());
                match self.synchronize().await {
                    Ok(()) => {
                        failures = 0;
                        jitter = Duration::ZERO;
                        tracing::info!("Authenticated Cloudflare Roughtime clock refreshed");
                    }
                    Err(error) => {
                        failures = failures.saturating_add(1);
                        use rand::RngExt;
                        jitter = Duration::from_secs(rand::rng().random_range(0..30));
                        tracing::warn!(
                            "Roughtime unavailable ({error}); previous clock anchor retained"
                        );
                    }
                }
            }
            self.check_wall_clock_anomaly();
            // A minute of local wakeups checks uptime after resume; only
            // a due refresh/backoff sends packets. No per-lock network I/O.
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}

static ENGINE_CLOCK: OnceLock<Arc<TrustedClock>> = OnceLock::new();

pub fn initialize_engine_clock() -> Arc<TrustedClock> {
    ENGINE_CLOCK
        .get_or_init(|| {
            Arc::new(TrustedClock::new(
                crate::settings::Settings::get_app_data_dir().join("trusted_clock_floor"),
            ))
        })
        .clone()
}

pub fn current_interval() -> Option<UtcInterval> {
    ENGINE_CLOCK.get().and_then(|clock| clock.now())
}

/// Safe comparison time for existing lock guards. An unavailable trusted clock
/// always leaves timed locks active; UTC is converted to Local only for the
/// existing public data structures, *not* by reading the OS time.
pub fn locking_now() -> DateTime<Local> {
    #[cfg(test)]
    if ENGINE_CLOCK.get().is_none() {
        // Legacy unit fixtures inject explicit local times rather than
        // initializing service runtime state. Production never does this.
        return Local::now();
    }
    let earliest = current_interval()
        .map(|range| range.earliest)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).expect("epoch"));
    earliest.with_timezone(&Local)
}

/// Only a verified clock may authorize creation of a future timed lock.
pub fn trusted_deadline_limit() -> Option<DateTime<Utc>> {
    current_interval().map(|range| range.earliest + chrono::Duration::days(90))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(midpoint_micros: u64, radius_micros: u32) -> VerifiedTime {
        VerifiedTime {
            midpoint_micros,
            radius_micros,
        }
    }

    #[test]
    fn system_wall_clock_forward_or_backward_is_detected_not_trusted() {
        let now = chrono::DateTime::<chrono::Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let interval = UtcInterval {
            earliest: now - chrono::Duration::seconds(2),
            latest: now + chrono::Duration::seconds(2),
            quality: TimeQuality::Verified,
        };
        assert!(!wall_clock_differs(interval, now));
        assert!(wall_clock_differs(
            interval,
            now + chrono::Duration::days(10)
        ));
        assert!(wall_clock_differs(
            interval,
            now - chrono::Duration::days(10)
        ));
    }

    #[test]
    fn interval_tracks_monotonic_elapsed_not_wall_clock() {
        let base = IntervalMicros {
            low: 1_700_000_000_000_000,
            high: 1_700_000_001_000_000,
        };
        let after_hour = interval_at(base, Duration::from_secs(3600)).unwrap();
        assert!(after_hour.low >= base.low + 3_598_000_000);
        assert!(after_hour.high <= base.high + 3_602_000_000);
        assert!(after_hour.low < after_hour.high);
    }

    #[test]
    fn outlier_uncertainty_or_rtt_is_rejected() {
        assert!(
            sample_interval(
                response(1_700_000_000_000_000, MAX_RADIUS + 1),
                Duration::ZERO
            )
            .is_none()
        );
        assert!(
            sample_interval(
                response(1_700_000_000_000_000, 1),
                MAX_RTT + Duration::from_millis(1)
            )
            .is_none()
        );
    }

    #[test]
    fn two_processes_cannot_roll_back_a_newer_floor() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("floor");
        let early = TrustedClock::new(path.clone());
        let late = TrustedClock::new(path.clone());
        // Both instances boot before either accepts a sample, as happens
        // when the service and the MSI uninstall helper start concurrently.
        late.accept(response(1_800_000_000_000_000, 1_000_000), Duration::ZERO)
            .unwrap();
        assert!(
            early
                .accept(response(1_700_000_000_000_000, 1_000_000), Duration::ZERO)
                .is_err()
        );
        let newer = std::fs::read_to_string(&path)
            .unwrap()
            .trim()
            .parse::<i64>()
            .unwrap();
        assert!(newer > 1_799_999_000_000_000);
        assert!(early.now().is_none());
    }

    #[test]
    fn malformed_floor_cannot_be_reset_by_a_new_sample() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("floor");
        std::fs::write(&path, "garbled").unwrap();
        let clock = TrustedClock::new(path.clone());
        assert!(
            clock
                .accept(response(1_800_000_000_000_000, 1_000_000), Duration::ZERO)
                .is_err()
        );
        assert!(clock.now().is_none());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "garbled");
    }

    #[test]
    fn validated_anchor_does_not_survive_reboot_without_new_verification() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("floor");
        let clock = TrustedClock::new(path.clone());
        assert!(clock.now().is_none());
        clock
            .accept(
                response(1_700_000_000_000_000, 1_000_000),
                Duration::from_millis(10),
            )
            .unwrap();
        assert!(clock.now().is_some());
        let rebooted = TrustedClock::new(path);
        assert!(rebooted.now().is_none());
        assert!(
            rebooted
                .accept(response(1_699_999_000_000_000, 1_000_000), Duration::ZERO)
                .is_err()
        );
        assert!(rebooted.now().is_none());
    }

    #[test]
    fn independent_clock_processes_cannot_roll_back_durable_floor() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("trusted-clock-floor");
        let first = TrustedClock::new(path.clone());
        let second = TrustedClock::new(path.clone());
        first
            .accept(response(1_700_000_000_000_000, 1_000_000), Duration::ZERO)
            .unwrap();
        let newer_sample = response(1_700_010_000_000_000, 1_000_000);
        second.accept(newer_sample, Duration::ZERO).unwrap();
        assert!(
            first
                .accept(response(1_700_000_000_000_000, 1_000_000), Duration::ZERO)
                .is_err(),
            "another process's advanced high-watermark must be re-read"
        );
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .trim()
                .parse::<i64>()
                .unwrap()
                >= 1_700_009_999_000_000
        );
    }

    #[test]
    fn invalid_unsigned_response_is_not_a_clock_sample() {
        let nonce = roughtime::request::Nonce::random().unwrap();
        assert!(
            roughtime::verify::verify_response(b"tampered", &nonce, CLOUDFLARE.pubkey_base64)
                .is_err()
        );
    }

    #[test]
    fn verified_interval_never_promotes_early_expiry() {
        let temp = tempfile::tempdir().unwrap();
        let clock = TrustedClock::new(temp.path().join("floor"));
        clock
            .accept(
                response(1_700_000_000_000_000, 10_000_000),
                Duration::from_millis(100),
            )
            .unwrap();
        let interval = clock.now().unwrap();
        assert!(interval.earliest < interval.latest);
        let deadline = interval.latest;
        assert!(interval.earliest <= deadline);
    }
}
