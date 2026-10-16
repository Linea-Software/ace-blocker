pub const TIME_FORMAT: &str = "%H:%M:%S";
pub const DATE_FORMAT: &str = "%Y-%m-%d";
pub const DATETIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

/// Ace Engine version, read from Cargo.toml at compile time.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 500 entries
pub const RESULTS_CACHE_MAX_SIZE: u64 = 500;
