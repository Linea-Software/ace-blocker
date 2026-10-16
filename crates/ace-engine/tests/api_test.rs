#[path = "api_test/api_client.rs"]
pub mod api_client;

#[path = "api_test/common.rs"]
pub mod common;

#[path = "api_test/schedule_test.rs"]
mod schedule_test;

#[path = "api_test/network_block_test.rs"]
mod network_block_test;

#[path = "api_test/sites_list_test.rs"]
mod sites_list_test;

#[path = "api_test/user_defined_entries_test.rs"]
mod user_defined_entries_test;

#[path = "api_test/account_test.rs"]
mod account_test;

#[path = "api_test/resistance_mode_test.rs"]
mod resistance_mode_test;

#[path = "api_test/backup_test.rs"]
mod backup_test;

#[path = "api_test/misc_test.rs"]
mod misc_test;
