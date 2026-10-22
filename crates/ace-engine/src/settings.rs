use app_dirs2::*;
use rand::prelude::*;
use std::path::PathBuf;
use tracing::error;

static TEST_SITES_DIR: std::sync::RwLock<Option<PathBuf>> = std::sync::RwLock::new(None);
static TEST_RESISTANCE_SCRIPTS_DIR: std::sync::RwLock<Option<PathBuf>> =
    std::sync::RwLock::new(None);
static TEST_BUNDLED_PACKS_DIR: std::sync::RwLock<Option<PathBuf>> = std::sync::RwLock::new(None);

pub const APP_INFO: AppInfo = AppInfo {
    name: "AceBlocker",
    author: "LineaSoftware",
};
pub struct Settings {}

impl Settings {
    pub fn get_install_dir() -> PathBuf {
        // The installation directory is the parent of the running executable.
        // In debug builds this is the build output dir; in release builds
        // it's the actual install folder (error.g. Program Files).
        let exe = std::env::current_exe().unwrap_or_default();
        exe.parent().map(|p| p.to_path_buf()).unwrap_or_default()
    }

    pub fn set_test_sites_dir(path: PathBuf) {
        *TEST_SITES_DIR.write().unwrap() = Some(path);
    }

    /// One process-wide temp dir shared by ALL unit tests that need a sites
    /// dir (TESTINFRA-01).
    ///
    /// The override in [`Settings::get_site_packs_dir`] is process-global and
    /// tests run in parallel, so a per-test dir would race: a fast test
    /// finishing mid-way through a slow test would delete the dir the slow
    /// test's pack I/O resolves to. Sharing a single leaked dir is safe
    /// because pack directories are UUID-named and no test asserts on the
    /// dir's own contents.
    #[cfg(test)]
    pub fn shared_test_sites_dir() -> PathBuf {
        // Own the fixture in OnceLock instead of deliberately forgetting it.
        // Shared lifetime remains process-wide until path injection is available.
        static SHARED: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
        SHARED
            .get_or_init(|| tempfile::tempdir().expect("failed to create shared test sites dir"))
            .path()
            .to_path_buf()
    }

    pub fn set_test_resistance_scripts_dir(path: PathBuf) {
        *TEST_RESISTANCE_SCRIPTS_DIR.write().unwrap() = Some(path);
    }

    /// Test-only override for the bundled-packs directory. Ungated on purpose
    /// (see `get_site_packs_dir` / TESTINFRA-01): the bundled-pack import runs
    /// at startup in the release binary, so integration tests must be able to
    /// point it at a temp tree.
    pub fn set_test_bundled_packs_dir(path: PathBuf) {
        *TEST_BUNDLED_PACKS_DIR.write().unwrap() = Some(path);
    }

    /// Returns the test override for the bundled-packs directory, if set.
    pub fn get_bundled_packs_dir_override() -> Option<PathBuf> {
        TEST_BUNDLED_PACKS_DIR.read().unwrap().clone()
    }

    pub fn get_app_dir() -> PathBuf {
        match app_dir(AppDataType::SharedData, &APP_INFO, "") {
            Ok(dir) => dir,
            Err(error) => {
                error!(
                    "CRITICAL ERROR: Failed to access/create sites list directory: {}",
                    error
                );
                std::process::exit(1);
            }
        }
    }

    pub fn get_site_packs_dir() -> PathBuf {
        // NOTE: deliberately NOT `#[cfg(test)]`-gated. Integration tests
        // (tests/) compile the library without `cfg(test)`, so a gated
        // override would silently send their site-pack I/O to the real
        // ProgramData directory. The override only takes effect when
        // `set_test_sites_dir` was called — which only tests do (TESTINFRA-01).
        if let Some(path) = TEST_SITES_DIR.read().unwrap().clone() {
            return path;
        }

        Self::get_app_dir().join("sites_db")
    }

    pub fn get_main_db_path() -> PathBuf {
        Self::get_app_data_dir().join("engine_db.sql")
    }

    pub fn get_cert_dir() -> PathBuf {
        Self::get_app_dir().join("certs")
    }

    pub fn get_resistance_scripts_dir() -> PathBuf {
        // Not `#[cfg(test)]`-gated: see `get_site_packs_dir` (TESTINFRA-01).
        if let Some(path) = TEST_RESISTANCE_SCRIPTS_DIR.read().unwrap().clone() {
            return path;
        }

        Self::get_app_dir().join("resistance_scripts")
    }

    pub fn get_resistance_scripts_json_path() -> PathBuf {
        let mut dir = Self::get_resistance_scripts_dir();
        dir.push("scripts-data.json");
        dir
    }

    pub fn get_site_packs_temp_dir() -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push("AceBlocker");
        dir.push("temp_sites");

        if let Err(error) = std::fs::create_dir_all(&dir) {
            error!(
                "CRITICAL ERROR: Failed to access/create temp sites directory: {}",
                error
            );
        }
        dir
    }

    pub fn get_site_pack_path(id: &crate::core::SitePackId) -> PathBuf {
        let mut dir = Settings::get_site_packs_dir();
        dir.push(id.to_string());
        dir
    }

    pub fn get_ace_blocker_server_base_url() -> String {
        #[cfg(debug_assertions)]
        return "http://localhost:5173".to_string();

        #[cfg(not(debug_assertions))]
        return "https://aceblocker.com".to_string();
    }

    pub fn get_app_data_dir() -> PathBuf {
        match app_dir(AppDataType::SharedData, &APP_INFO, "") {
            Ok(d) => d,
            Err(error) => {
                error!(
                    "CRITICAL ERROR: Failed to access/create app data directory: {}",
                    error
                );
                std::process::exit(1);
            }
        }
    }

    /// Path of the per-install capability token file.
    ///
    /// The token authorizes capability-protected remoc calls over the local
    /// Named Pipe/Unix socket. It is stored in app data with restrictive
    /// permissions and hidden at startup by `hide_important_files`.
    pub fn get_capability_token_path() -> PathBuf {
        let mut path = Self::get_app_data_dir();
        path.push("capability.token");
        path
    }

    /// Loads the per-install capability token, creating and persisting a new
    /// 256-bit random token on first run (or if the file is unreadable/empty).
    pub fn load_or_create_capability_token() -> String {
        let path = Self::get_capability_token_path();
        if let Ok(existing) = std::fs::read_to_string(&path) {
            let trimmed = existing.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }

        let mut rng = rand::rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.random::<u8>()).collect();
        let token = hex::encode(bytes);

        if let Err(error) = std::fs::write(&path, &token) {
            error!(
                "Failed to persist capability token at {:?}: {}",
                path, error
            );
        }
        token
    }
}
