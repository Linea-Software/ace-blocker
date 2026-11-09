//! Read-only lock-state query for the installer's uninstall gate.
//!
//! `ace-engine.exe --check-locks` (or `cargo run -- --check-locks` in debug)
//! opens the engine database **read-only** — no migrations, no writes, no
//! HTTP, no service interaction — and reports whether any uninstall-blocking
//! lock is active:
//!
//! - exit code `0`: no active locks — uninstall may proceed;
//! - exit code `1`: at least one active lock — the message printed on stderr
//!   lists the locks with their exact expiration times;
//! - exit code `2`: engine state cannot be verified (database missing or
//!   unreadable) — callers must fail safe and refuse the uninstall.
//!
//! The engine service may be running while this runs; read-only SQLite
//! access is safe concurrently with the service's writer. A short
//! `busy_timeout` absorbs transient writer contention (SVC-08) so a
//! mid-commit engine cannot spuriously block the uninstall gate.
//!
//! # Behavior notes
//!
//! - **Engine unreachable:** this tool never contacts the engine at all — it
//!   reads the database directly, so "engine down" and "engine running"
//!   behave identically. A missing/unreadable database is the only
//!   cannot-verify case.
//! - **Scheduled locks (`active_during_schedule`):** treated as *always
//!   active* here, even though the engine's own `Lock::is_active` evaluates
//!   them only inside the block's schedule window (see `core/common/lock.rs`).
//!   The gate is deliberately conservative: it cannot evaluate schedules
//!   (and uninstall is irreversible), so the state alone blocks uninstall.
//!   A schedule lock therefore blocks uninstall until the user removes it
//!   from the app — it never "expires on its own" like a timed lock.

use chrono::{DateTime, Utc};
#[cfg(test)]
use chrono::{Local, NaiveDateTime, TimeZone};

#[cfg(test)]
use crate::core::constants::DATETIME_FORMAT;
use crate::settings::Settings;

pub const EXIT_NO_LOCKS: i32 = 0;
pub const EXIT_LOCKS_ACTIVE: i32 = 1;
pub const EXIT_CANNOT_VERIFY: i32 = 2;

/// How long a read-only query waits for a writer's lock before failing
/// (SVC-08: a mid-commit engine must not spuriously block uninstall).
const BUSY_TIMEOUT_MS: i32 = 2000;

/// True when a persisted lock row is currently active.
///
/// Mirrors the engine's `LockState` semantics: `inactive` (and the legacy
/// `INACTIVE` default row) is not active; `active_until_lock_end` is active
/// while `now <= end`; `soft_unlockable` and `active_during_schedule` are
/// treated as active (the engine's `is_locked()` does the same). Anything
/// unparseable is treated as active — fail safe.
fn lock_is_active_at(
    state: &str,
    lock_end: Option<&str>,
    verified_now: Option<DateTime<Utc>>,
) -> bool {
    match state {
        "INACTIVE" | "inactive" => false,
        "active_until_lock_end" => {
            // New stored deadlines carry a UTC offset, so system timezone
            // changes cannot change their absolute end.
            let end = lock_end.and_then(|raw| {
                DateTime::parse_from_rfc3339(raw)
                    .ok()
                    .map(|dt| dt.to_utc())
                    .or_else(|| {
                        // Pre-release local-only rows cannot be authenticated
                        // across a timezone change, so don't declare them expired.
                        #[cfg(test)]
                        {
                            NaiveDateTime::parse_from_str(raw, DATETIME_FORMAT)
                                .ok()
                                .and_then(|dt| Local.from_local_datetime(&dt).single())
                                .map(|dt| dt.to_utc())
                        }
                        #[cfg(not(test))]
                        {
                            let _ = raw;
                            None
                        }
                    })
            });
            match (verified_now, end) {
                (Some(now), Some(end)) => now <= end,
                _ => true, // never release based on untrusted wall time
            }
        }
        _ => true,
    }
}

#[cfg(test)]
fn lock_is_active(state: &str, lock_end: Option<&str>) -> bool {
    lock_is_active_at(state, lock_end, effective_time())
}

fn effective_time() -> Option<DateTime<Utc>> {
    #[cfg(test)]
    if crate::trusted_time::current_interval().is_none() {
        return Some(Utc::now()); // legacy fixture compatibility only
    }
    crate::trusted_time::current_interval().map(|interval| interval.earliest)
}

/// Human-readable message for an active lock row, or `None` when the row is
/// not active. `subject` names the locked thing ("a network block" or
/// "Resistance Mode"); a timed lock's message carries the exact `lock_end`
/// value so callers and users see the precise expiration time.
#[cfg(test)]
fn active_lock_message(subject: &str, state: &str, lock_end: Option<&str>) -> Option<String> {
    active_lock_message_at(subject, state, lock_end, effective_time())
}

fn active_lock_message_at(
    subject: &str,
    state: &str,
    lock_end: Option<&str>,
    now: Option<DateTime<Utc>>,
) -> Option<String> {
    if !lock_is_active_at(state, lock_end, now) {
        return None;
    }
    Some(match lock_end {
        Some(end) => format!("{subject} is locked until {end}"),
        None => format!("{subject} is locked (state: {state})"),
    })
}

/// Runs the lock check against the engine's main database and returns the
/// process exit code.
pub async fn check_locks_main() -> i32 {
    // A separate installer process cannot reuse the engine's uptime anchor.
    // Perform one independent authenticated query; offline timed locks remain
    // active. The SQLite query itself stays strictly read-only.
    let clock = crate::trusted_time::TrustedClock::new(
        Settings::get_app_data_dir().join("trusted_clock_floor"),
    );
    if let Err(error) = clock.synchronize().await {
        eprintln!("Ace Blocker: authenticated uninstall-time unavailable ({error})");
    }
    check_locks_at_time(
        &Settings::get_main_db_path(),
        clock.now().map(|interval| interval.earliest),
    )
}

/// Runs the lock check against `db_path` (read-only). Public so tests and
/// tooling can point at an arbitrary database.
pub fn check_locks_at(db_path: &std::path::Path) -> i32 {
    check_locks_at_time(db_path, effective_time())
}

fn check_locks_at_time(db_path: &std::path::Path, now: Option<DateTime<Utc>>) -> i32 {
    if !db_path.exists() {
        eprintln!(
            "Ace Blocker: cannot verify lock state — engine database not found at {}",
            db_path.display()
        );
        return EXIT_CANNOT_VERIFY;
    }

    // Read-only open: never migrate, never write. `file:` URI + `mode=ro`
    // (rusqlite enables SQLITE_OPEN_URI by default).
    let url = format!(
        "file:{}?mode=ro",
        db_path.to_string_lossy().replace('\\', "/")
    );
    let mut conn = match diesel::SqliteConnection::establish(&url) {
        Ok(conn) => conn,
        Err(error) => {
            eprintln!(
                "Ace Blocker: cannot verify lock state — failed to open database read-only: {}",
                error
            );
            return EXIT_CANNOT_VERIFY;
        }
    };

    // Absorb transient writer contention (the engine may be mid-commit) so a
    // busy database is waited out instead of misreported as cannot-verify
    // (SVC-08). `busy_timeout` is connection state, not a write, so it is
    // legal on a read-only connection.
    if let Err(error) =
        diesel::sql_query(format!("PRAGMA busy_timeout = {BUSY_TIMEOUT_MS}")).execute(&mut conn)
    {
        eprintln!(
            "Ace Blocker: cannot verify lock state — failed to configure busy timeout: {}",
            error
        );
        return EXIT_CANNOT_VERIFY;
    }

    use crate::schema_main_db::{network_block_locks, resistance_mode_settings};
    use diesel::prelude::*;

    let mut active_locks: Vec<String> = Vec::new();

    // Network-block locks.
    match network_block_locks::table
        .select((
            network_block_locks::lock_state,
            network_block_locks::lock_end,
        ))
        .load::<(String, Option<String>)>(&mut conn)
    {
        Ok(rows) => {
            for (state, lock_end) in rows {
                if let Some(message) =
                    active_lock_message_at("a network block", &state, lock_end.as_deref(), now)
                {
                    active_locks.push(message);
                }
            }
        }
        Err(error) => {
            eprintln!(
                "Ace Blocker: cannot verify lock state — failed to read network block locks: {}",
                error
            );
            return EXIT_CANNOT_VERIFY;
        }
    }

    // Resistance-Mode lock (single settings row).
    match resistance_mode_settings::table
        .select((
            resistance_mode_settings::lock_state,
            resistance_mode_settings::lock_end,
        ))
        .first::<(String, Option<String>)>(&mut conn)
    {
        Ok((state, lock_end)) => {
            if let Some(message) =
                active_lock_message_at("Resistance Mode", &state, lock_end.as_deref(), now)
            {
                active_locks.push(message);
            }
        }
        Err(diesel::result::Error::NotFound) => {}
        Err(error) => {
            eprintln!(
                "Ace Blocker: cannot verify lock state — failed to read Resistance Mode settings: {}",
                error
            );
            return EXIT_CANNOT_VERIFY;
        }
    }

    if active_locks.is_empty() {
        println!("Ace Blocker: no active locks — uninstall may proceed.");
        EXIT_NO_LOCKS
    } else {
        eprintln!("Ace Blocker: uninstall refused — the following lock(s) are active:");
        for lock in &active_locks {
            eprintln!("  - {lock}");
        }
        eprintln!("Active locks cannot be removed or shortened; they expire on their own.");
        EXIT_LOCKS_ACTIVE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::constants::DATETIME_FORMAT;
    use crate::db::EngineDB;
    use crate::db::models::ResistanceModeSettingsModel;

    fn later(hours: i64) -> String {
        (chrono::Local::now() + chrono::Duration::hours(hours))
            .format(DATETIME_FORMAT)
            .to_string()
    }

    fn open_db(dir: &std::path::Path) -> EngineDB {
        EngineDB::from(&dir.join("engine_db.sql")).expect("test db")
    }

    /// Inserts a network block; the DB trigger creates its inactive lock row.
    fn insert_block(db: &EngineDB, block_id: &uuid::Uuid) {
        use diesel::prelude::*;
        let conn = &mut db.get_db_connection().expect("conn");
        diesel::sql_query(
            "INSERT INTO network_blocks (id, name, safe_search) VALUES (?, 'test', 0)",
        )
        .bind::<diesel::sql_types::Text, _>(block_id.to_string())
        .execute(conn)
        .expect("insert block");
    }

    #[test]
    fn test_no_locks_means_uninstall_may_proceed() {
        let temp = tempfile::tempdir().unwrap();
        let db = open_db(temp.path());
        let db_path = temp.path().join("engine_db.sql");

        // Default state: inactive network lock, inactive resistance lock.
        let block_id = uuid::Uuid::now_v7();
        insert_block(&db, &block_id);
        db.set_resistance_mode_settings(ResistanceModeSettingsModel {
            table_lock: "0".to_string(),
            enabled: true,
            lock_state: "inactive".to_string(),
            lock_end: None,
            scripts_version: None,
        })
        .unwrap();

        assert_eq!(check_locks_at(&db_path), EXIT_NO_LOCKS);
    }

    #[test]
    fn test_active_network_block_lock_refuses_uninstall() {
        let temp = tempfile::tempdir().unwrap();
        let db = open_db(temp.path());
        let db_path = temp.path().join("engine_db.sql");

        let block_id = uuid::Uuid::now_v7();
        insert_block(&db, &block_id);
        db.set_lock(crate::db::models::NetworkBlockLockModel {
            network_block_id: block_id.to_string(),
            lock_state: "active_until_lock_end".to_string(),
            lock_end: Some(later(24)),
        })
        .unwrap();

        assert_eq!(check_locks_at(&db_path), EXIT_LOCKS_ACTIVE);
    }

    #[test]
    fn test_active_resistance_lock_refuses_uninstall() {
        let temp = tempfile::tempdir().unwrap();
        let db = open_db(temp.path());
        let db_path = temp.path().join("engine_db.sql");

        db.set_resistance_mode_settings(ResistanceModeSettingsModel {
            table_lock: "0".to_string(),
            enabled: true,
            lock_state: "active_until_lock_end".to_string(),
            lock_end: Some(later(48)),
            scripts_version: None,
        })
        .unwrap();

        assert_eq!(check_locks_at(&db_path), EXIT_LOCKS_ACTIVE);
    }

    #[test]
    fn test_expired_locks_allow_uninstall() {
        let temp = tempfile::tempdir().unwrap();
        let db = open_db(temp.path());
        let db_path = temp.path().join("engine_db.sql");

        let block_id = uuid::Uuid::now_v7();
        insert_block(&db, &block_id);
        db.set_lock(crate::db::models::NetworkBlockLockModel {
            network_block_id: block_id.to_string(),
            lock_state: "active_until_lock_end".to_string(),
            lock_end: Some(later(-1)),
        })
        .unwrap();
        db.set_resistance_mode_settings(ResistanceModeSettingsModel {
            table_lock: "0".to_string(),
            enabled: true,
            lock_state: "active_until_lock_end".to_string(),
            lock_end: Some(later(-2)),
            scripts_version: None,
        })
        .unwrap();

        assert_eq!(check_locks_at(&db_path), EXIT_NO_LOCKS);
    }

    #[test]
    fn test_missing_database_fails_safe() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("does-not-exist.sql");
        assert_eq!(check_locks_at(&missing), EXIT_CANNOT_VERIFY);
    }

    #[test]
    fn test_unparseable_lock_state_fails_safe_as_active() {
        // The DB CHECK rejects unknown states at write time, so exercise the
        // decision function directly: anything unrecognized is active.
        assert!(lock_is_active("some_future_state", None));
        assert!(lock_is_active("", None));
        // A timed lock without a parseable end time is treated as active.
        assert!(lock_is_active("active_until_lock_end", None));
        assert!(lock_is_active("active_until_lock_end", Some("not-a-date")));
        // Expired timed locks are not active.
        assert!(!lock_is_active("active_until_lock_end", Some(&later(-1))));
        assert!(!lock_is_active("inactive", None));
        assert!(!lock_is_active("INACTIVE", None));
    }

    #[test]
    fn offline_reboot_does_not_expire_a_timed_lock() {
        let end = "2020-01-01T00:00:00+00:00";
        assert!(lock_is_active_at("active_until_lock_end", Some(end), None));
        assert!(!lock_is_active_at(
            "active_until_lock_end",
            Some(end),
            Some(Utc.with_ymd_and_hms(2026, 11, 5, 12, 0, 0).unwrap()),
        ));
    }

    #[test]
    fn independent_uninstall_gate_uses_authenticated_instant() {
        let temp = tempfile::tempdir().unwrap();
        let db = open_db(temp.path());
        let db_path = temp.path().join("engine_db.sql");
        let block_id = uuid::Uuid::now_v7();
        insert_block(&db, &block_id);
        db.set_lock(crate::db::models::NetworkBlockLockModel {
            network_block_id: block_id.to_string(),
            lock_state: "active_until_lock_end".into(),
            lock_end: Some("2020-01-01T00:00:00+00:00".into()),
        })
        .unwrap();
        assert_eq!(check_locks_at_time(&db_path, None), EXIT_LOCKS_ACTIVE);
        let verified = Utc.with_ymd_and_hms(2026, 11, 5, 12, 0, 0).unwrap();
        assert_eq!(check_locks_at_time(&db_path, Some(verified)), EXIT_NO_LOCKS);
    }

    #[test]
    fn test_active_lock_message_prints_exact_expiration() {
        // The uninstall gate must surface the exact end time of a timed lock
        // (spec §5 / LIF-01): the user has to see precisely when the lock
        // expires, not a paraphrase.
        let end = later(24);
        let message = active_lock_message("a network block", "active_until_lock_end", Some(&end))
            .expect("an active timed lock must produce a message");
        assert!(
            message.contains(&end),
            "the message must carry the exact expiration, got: {message}"
        );
        assert!(message.contains("locked until"));
        assert_eq!(
            active_lock_message("Resistance Mode", "active_until_lock_end", Some(&end)),
            Some(format!("Resistance Mode is locked until {end}"))
        );

        // Inactive and expired rows produce no message.
        assert!(active_lock_message("a network block", "inactive", None).is_none());
        assert!(
            active_lock_message("a network block", "active_until_lock_end", Some(&later(-1)))
                .is_none()
        );
        // Non-timed active states still produce a message without an end time.
        assert_eq!(
            active_lock_message("Resistance Mode", "soft_unlockable", None),
            Some("Resistance Mode is locked (state: soft_unlockable)".to_string())
        );
    }

    #[test]
    fn test_soft_unlockable_and_schedule_locks_are_active() {
        for state in ["soft_unlockable", "active_during_schedule"] {
            let temp = tempfile::tempdir().unwrap();
            let db = open_db(temp.path());
            let db_path = temp.path().join("engine_db.sql");

            let block_id = uuid::Uuid::now_v7();
            insert_block(&db, &block_id);
            db.set_lock(crate::db::models::NetworkBlockLockModel {
                network_block_id: block_id.to_string(),
                lock_state: state.to_string(),
                lock_end: None,
            })
            .unwrap();

            assert_eq!(
                check_locks_at(&db_path),
                EXIT_LOCKS_ACTIVE,
                "{state} must block uninstall"
            );
        }
    }

    #[test]
    fn test_writer_contention_does_not_fail_the_gate() {
        // SVC-08: an engine mid-commit must not spuriously turn the gate into
        // "cannot verify". Hold an exclusive write lock for ~400 ms in a
        // background thread and verify the gate waits it out (busy_timeout)
        // and proceeds.
        let temp = tempfile::tempdir().unwrap();
        let db = open_db(temp.path());
        let db_path = temp.path().join("engine_db.sql");

        let block_id = uuid::Uuid::now_v7();
        insert_block(&db, &block_id);

        let (locked_tx, locked_rx) = std::sync::mpsc::channel();
        let conn_path = db_path.clone();
        let holder = std::thread::spawn(move || {
            use diesel::prelude::*;
            let mut conn =
                diesel::SqliteConnection::establish(conn_path.to_str().unwrap()).unwrap();
            diesel::sql_query("BEGIN EXCLUSIVE")
                .execute(&mut conn)
                .expect("begin exclusive");
            locked_tx.send(()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(400));
            diesel::sql_query("ROLLBACK")
                .execute(&mut conn)
                .expect("rollback");
        });

        locked_rx.recv().unwrap();
        assert_eq!(
            check_locks_at(&db_path),
            EXIT_NO_LOCKS,
            "transient writer contention must be waited out, not treated as cannot-verify"
        );
        holder.join().unwrap();
    }
}
