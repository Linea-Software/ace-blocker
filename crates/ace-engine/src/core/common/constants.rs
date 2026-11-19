pub const TIME_FORMAT: &str = "%H:%M:%S";
pub const DATE_FORMAT: &str = "%Y-%m-%d";
pub const DATETIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

/// Ace Engine version, read from Cargo.toml at compile time.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Per-pack host membership results; never the full list of site hashes.
pub const SITE_PACK_MEMBERSHIP_CACHE_MAX_SIZE: u64 = 2_048;

/// Per-block URI decisions, independently invalidated on policy changes.
pub const NETWORK_BLOCK_RESULTS_CACHE_MAX_SIZE: u64 = 1_024;
