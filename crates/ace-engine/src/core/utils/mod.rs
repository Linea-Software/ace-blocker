pub mod file_download;

use std::io::ErrorKind;
use std::path::Path;

/// Best-effort removal of a superseded (`.old-*`) directory after an atomic
/// live→old directory swap.
///
/// The `.old-*` directory only exists when a live directory was actually
/// renamed aside. When no live directory existed at swap time (e.g. the
/// pack/scripts directory was never materialized), `remove_dir_all` fails
/// with `NotFound` (os error 2) — an expected, harmless outcome of the
/// idempotent swap pattern, not a cleanup failure.
///
/// `NotFound` is therefore swallowed. Genuine failures (permissions, in-use
/// handles, ...) are returned so the caller can decide whether to warn.
pub fn remove_superseded_dir(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        // The superseded directory was never created (nothing to clean) —
        // treat as success instead of a misleading cleanup warning.
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn removing_a_missing_superseded_dir_is_ok() {
        let parent = tempdir().unwrap();
        // A path that never existed — the exact "os error 2" case that used
        // to produce the misleading "failed to delete superseded" warning.
        let missing = parent
            .path()
            .join("never-created.old-00000000-0000-0000-0000-000000000000");
        assert_eq!(remove_superseded_dir(&missing).unwrap(), ());
    }

    #[test]
    fn removing_an_existing_dir_succeeds_and_cleans_it() {
        let parent = tempdir().unwrap();
        let old = parent
            .path()
            .join(".pack.old-11111111-1111-1111-1111-111111111111");
        std::fs::create_dir(&old).unwrap();
        assert_eq!(remove_superseded_dir(&old).unwrap(), ());
        assert!(!old.exists());
    }

    #[test]
    fn genuine_cleanup_failures_are_still_reported() {
        let parent = tempdir().unwrap();
        // remove_dir_all on a regular file is a genuine error (not
        // NotFound) and must propagate so the caller can warn.
        let file_path = parent.path().join("not-a-directory");
        std::fs::write(&file_path, b"x").unwrap();
        let result = remove_superseded_dir(&file_path);
        assert!(result.is_err());
        assert_ne!(result.unwrap_err().kind(), ErrorKind::NotFound);
    }
}
