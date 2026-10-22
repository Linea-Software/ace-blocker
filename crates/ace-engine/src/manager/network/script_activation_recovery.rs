//! Crash recovery for the resistance script directory / SQLite version boundary.
//! A durable activation marker is written *before* moving live scripts aside.
//! Once SQLite holds the target version the new catalog is committed; before
//! that, recovery prefers the previously working directory.
use crate::{db::EngineDB, manager::NetworkManagerOpErr, settings::Settings};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tracing::warn;
use uuid::Uuid;

const JOURNAL_FILE: &str = ".resistance_scripts.activation.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ActivationJournal {
    id: Uuid,
    target_version: String,
    previous_version: Option<String>,
    had_live: bool,
}

fn error(context: &str, cause: impl std::fmt::Display) -> NetworkManagerOpErr {
    NetworkManagerOpErr::InternalError {
        message: format!("resistance script activation {context}: {cause}"),
    }
}

fn paths(live_dir: &Path, id: Uuid) -> Result<(PathBuf, PathBuf, PathBuf), NetworkManagerOpErr> {
    let parent = live_dir
        .parent()
        .ok_or_else(|| error("path", "missing parent"))?;
    Ok((
        parent.join(JOURNAL_FILE),
        parent.join(format!(".resistance_scripts.old-{id}")),
        parent.join(format!(".resistance_scripts.failed-{id}")),
    ))
}

pub(super) fn begin_activation(
    db: &EngineDB,
    live_dir: &Path,
    id: Uuid,
    target_version: &str,
    had_live: bool,
) -> Result<(), NetworkManagerOpErr> {
    // Retry a previously interrupted transition before starting another.
    recover_pending_at(db, live_dir)?;
    let (journal_path, _, _) = paths(live_dir, id)?;
    let previous_version = db.get_resistance_mode_settings()?.scripts_version;
    let journal = ActivationJournal {
        id,
        target_version: target_version.to_string(),
        previous_version,
        had_live,
    };
    let encoded = serde_json::to_vec(&journal).map_err(|e| error("encode", e))?;
    // create_new avoids overwriting a journal from a different update.
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&journal_path)
        .map_err(|e| error("journal create", e))?;
    output
        .write_all(&encoded)
        .map_err(|e| error("journal write", e))?;
    output.sync_all().map_err(|e| error("journal sync", e))?;
    Ok(())
}

pub(super) fn clear_completed(live_dir: &Path) -> Result<(), NetworkManagerOpErr> {
    let parent = live_dir
        .parent()
        .ok_or_else(|| error("path", "missing parent"))?;
    match fs::remove_file(parent.join(JOURNAL_FILE)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(error("journal cleanup", e)),
    }
}

pub(super) fn recover_pending(db: &EngineDB) -> Result<(), NetworkManagerOpErr> {
    recover_pending_at(db, &Settings::get_resistance_scripts_dir())
}

pub(super) fn recover_pending_at(
    db: &EngineDB,
    live_dir: &Path,
) -> Result<(), NetworkManagerOpErr> {
    let parent = live_dir
        .parent()
        .ok_or_else(|| error("path", "missing parent"))?;
    let journal_path = parent.join(JOURNAL_FILE);
    let encoded = match fs::read(&journal_path) {
        Ok(data) => data,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(error("journal read", e)),
    };
    let journal: ActivationJournal =
        serde_json::from_slice(&encoded).map_err(|e| error("invalid journal", e))?;
    let (_, old_dir, failed_dir) = paths(live_dir, journal.id)?;
    let persisted_version = db.get_resistance_mode_settings()?.scripts_version;
    // If a same-version refresh crashes, always choose the known-good old tree.
    let committed = persisted_version.as_deref() == Some(journal.target_version.as_str())
        && journal.previous_version.as_deref() != Some(journal.target_version.as_str());

    if committed {
        if !live_dir.exists() {
            return Err(error(
                "recover committed scripts",
                "new live directory is missing",
            ));
        }
        // SQLite committed the new version; old directory is cleanup only.
        if old_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&old_dir) {
                warn!(%e, path = %old_dir.display(), "Unable to remove superseded scripts");
                return Err(error("cleanup committed scripts", e));
            }
        }
    } else if old_dir.exists() {
        // SQLite still contains the prior version, or committed live files are
        // missing: put the known-good old directory back.
        if live_dir.exists() {
            fs::rename(live_dir, &failed_dir)
                .map_err(|e| error("quarantine uncommitted scripts", e))?;
        }
        if let Err(e) = fs::rename(&old_dir, live_dir) {
            if failed_dir.exists() {
                let _ = fs::rename(&failed_dir, live_dir);
            }
            return Err(error("restore previous scripts", e));
        }
        if failed_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&failed_dir) {
                warn!(%e, path = %failed_dir.display(), "Unable to remove uncommitted scripts");
            }
        }
    } else if !committed && !journal.had_live && live_dir.exists() {
        // First-ever activation was interrupted; there is no prior tree to restore.
        fs::remove_dir_all(live_dir).map_err(|e| error("remove uncommitted first install", e))?;
    }
    clear_completed(live_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::EngineDB;

    fn setup() -> (tempfile::TempDir, EngineDB, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let db = EngineDB::from(&temp.path().join("main.sqlite")).unwrap();
        db.update_scripts_version("1.0".to_string()).unwrap();
        let live_dir = temp.path().join("resistance_scripts");
        fs::create_dir(&live_dir).unwrap();
        fs::write(live_dir.join("scripts-data.json"), "previous").unwrap();
        (temp, db, live_dir)
    }

    #[test]
    fn crash_before_swap_keeps_previous_scripts() {
        let (_temp, db, live) = setup();
        let id = Uuid::new_v4();
        begin_activation(&db, &live, id, "2.0", true).unwrap();
        recover_pending_at(&db, &live).unwrap();
        assert_eq!(
            fs::read_to_string(live.join("scripts-data.json")).unwrap(),
            "previous"
        );
        assert!(!live.parent().unwrap().join(JOURNAL_FILE).exists());
    }

    #[test]
    fn crash_between_renaming_old_and_installing_new_restores_previous() {
        let (_temp, db, live) = setup();
        let id = Uuid::new_v4();
        begin_activation(&db, &live, id, "2.0", true).unwrap();
        let (_, backup, _) = paths(&live, id).unwrap();
        fs::rename(&live, &backup).unwrap();
        assert!(!live.exists());

        recover_pending_at(&db, &live).unwrap();
        assert_eq!(
            fs::read_to_string(live.join("scripts-data.json")).unwrap(),
            "previous"
        );
        assert!(!backup.exists());
    }

    #[test]
    fn crash_after_swap_before_sqlite_commit_restores_previous() {
        let (_temp, db, live) = setup();
        let id = Uuid::new_v4();
        begin_activation(&db, &live, id, "2.0", true).unwrap();
        let (_, backup, _) = paths(&live, id).unwrap();
        fs::rename(&live, &backup).unwrap();
        fs::create_dir(&live).unwrap();
        fs::write(live.join("scripts-data.json"), "uncommitted").unwrap();

        recover_pending_at(&db, &live).unwrap();
        assert_eq!(
            fs::read_to_string(live.join("scripts-data.json")).unwrap(),
            "previous"
        );
        assert!(!backup.exists());
        assert!(!live.parent().unwrap().join(JOURNAL_FILE).exists());
        recover_pending_at(&db, &live).unwrap(); // Idempotent.
    }

    #[test]
    fn crash_after_sqlite_commit_keeps_new_scripts() {
        let (_temp, db, live) = setup();
        let id = Uuid::new_v4();
        begin_activation(&db, &live, id, "2.0", true).unwrap();
        let (_, backup, _) = paths(&live, id).unwrap();
        fs::rename(&live, &backup).unwrap();
        fs::create_dir(&live).unwrap();
        fs::write(live.join("scripts-data.json"), "committed").unwrap();
        db.update_scripts_version("2.0".to_string()).unwrap();

        recover_pending_at(&db, &live).unwrap();
        assert_eq!(
            fs::read_to_string(live.join("scripts-data.json")).unwrap(),
            "committed"
        );
        assert!(!backup.exists());
        assert!(!live.parent().unwrap().join(JOURNAL_FILE).exists());
    }

    #[test]
    fn first_install_crash_removes_uncommitted_scripts() {
        let (temp, db, _live) = setup();
        let live = temp.path().join("brand_new_scripts");
        let id = Uuid::new_v4();
        begin_activation(&db, &live, id, "2.0", false).unwrap();
        fs::create_dir(&live).unwrap();
        fs::write(live.join("scripts-data.json"), "uncommitted").unwrap();
        recover_pending_at(&db, &live).unwrap();
        assert!(!live.exists());
    }
}
