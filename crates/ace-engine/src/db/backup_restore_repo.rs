//! Atomic persistence operations for restoring current-format engine backups.
//! The manager owns authorization and validation; this repository owns SQL
//! ordering and per-resource transactions.

use super::{DBError, EngineDB};
use crate::core::{SitePackId, UserDefinedRule};
use crate::db::models::{NetworkBlockScheduleEntryModel, SitePackModel, SitePackSourceModel};
use crate::schema_main_db::{
    network_block_schedule_entries, network_block_to_site_pack, network_blocks, site_pack_sources,
    site_packs, user_defined_rules,
};
use diesel::prelude::*;
use uuid::Uuid;

/// A validated block to restore in one SQLite transaction.
pub struct RestoredNetworkBlock<'a> {
    pub id: Uuid,
    pub name: &'a str,
    pub enabled: bool,
    pub safe_search: bool,
    pub icon: Option<&'a str>,
    pub schedules: &'a [NetworkBlockScheduleEntryModel],
    pub user_rules: &'a [UserDefinedRule],
    pub site_pack_ids: &'a [SitePackId],
}

impl EngineDB {
    /// Distinguishes an absent pack row from a broken SQLite read.
    pub fn site_pack_exists(&self, pack_id: &SitePackId) -> Result<bool, DBError> {
        let mut connection = self.get_db_connection()?;
        site_packs::table
            .find(pack_id.as_ref())
            .select(site_packs::id)
            .first::<String>(&mut connection)
            .optional()
            .map(|row| row.is_some())
            .map_err(|error| DBError::DataFetchError {
                table: "site_packs".to_string(),
                message: error.to_string(),
            })
    }

    /// Adds a missing pack and all its sources atomically; an existing row is
    /// never deleted or altered (especially a seeded/remote pack).
    pub fn restore_missing_site_pack(
        &self,
        pack: &SitePackModel,
        sources: &[SitePackSourceModel],
    ) -> Result<bool, DBError> {
        let mut connection = self.get_db_connection()?;
        connection
            .transaction::<bool, diesel::result::Error, _>(|connection| {
                let exists = site_packs::table
                    .find(&pack.id)
                    .select(site_packs::id)
                    .first::<String>(connection)
                    .optional()?
                    .is_some();
                if exists {
                    return Ok(false);
                }
                diesel::insert_into(site_packs::table)
                    .values(pack)
                    .execute(connection)?;

                for source in sources {
                    diesel::insert_into(site_pack_sources::table)
                        .values(source)
                        .execute(connection)?;
                }
                Ok(true)
            })
            .map_err(|error| DBError::InsertionError {
                table: "site_packs / site_pack_sources".to_string(),
                message: format!("backup site-pack transaction failed: {error}"),
            })
    }

    /// Existing block data is replaced entirely in one SQL transaction.
    /// Failed rule/schedule/link insertion rolls back deletion and all writes.
    /// Lock authorization must be checked by the manager *before* invoking.
    pub fn restore_network_block(&self, block: RestoredNetworkBlock<'_>) -> Result<(), DBError> {
        let mut connection = self.get_db_connection()?;
        let block_id = block.id.to_string();
        connection
            .transaction::<(), diesel::result::Error, _>(|connection| {
                // Associations are mandatory, not a warning-only partial restore.
                for site_pack_id in block.site_pack_ids {
                    site_packs::table
                        .find(site_pack_id.as_ref())
                        .select(site_packs::id)
                        .first::<String>(connection)?;
                }

                diesel::delete(network_blocks::table.find(&block_id)).execute(connection)?;
                diesel::insert_into(network_blocks::table)
                    .values((
                        network_blocks::id.eq(&block_id),
                        network_blocks::name.eq(block.name),
                        network_blocks::enabled.eq(block.enabled),
                        network_blocks::safe_search.eq(block.safe_search),
                        network_blocks::icon.eq(block.icon),
                    ))
                    .execute(connection)?;

                for schedule in block.schedules {
                    diesel::insert_into(network_block_schedule_entries::table)
                        .values((
                            network_block_schedule_entries::id.eq(Uuid::now_v7().to_string()),
                            network_block_schedule_entries::network_block_id.eq(&block_id),
                            network_block_schedule_entries::start_time.eq(&schedule.start_time),
                            network_block_schedule_entries::end_time.eq(&schedule.end_time),
                            network_block_schedule_entries::schedule_days_bitmask
                                .eq(schedule.schedule_days_bitmask),
                        ))
                        .execute(connection)?;
                }

                for rule in block.user_rules {
                    diesel::insert_into(user_defined_rules::table)
                        .values((
                            user_defined_rules::id.eq(Uuid::now_v7().to_string()),
                            user_defined_rules::network_block_id.eq(&block_id),
                            user_defined_rules::rule.eq(&rule.rule),
                            user_defined_rules::rule_type.eq(rule.rule_type.to_string()),
                        ))
                        .execute(connection)?;
                }

                for pack_id in block.site_pack_ids {
                    diesel::insert_into(network_block_to_site_pack::table)
                        .values((
                            network_block_to_site_pack::network_block_id.eq(&block_id),
                            network_block_to_site_pack::site_pack_id.eq(pack_id.as_ref()),
                        ))
                        .execute(connection)?;
                }
                Ok(())
            })
            .map_err(|error| DBError::UpdateError {
                table: "network_blocks (backup restore)".to_string(),
                message: error.to_string(),
            })
    }
}
