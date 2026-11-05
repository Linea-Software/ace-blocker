//! SQLite boundary for a complete Network Block editor save.
//! No other connection is used until this transaction returns.
use crate::db::{
    DBError, EngineDB,
    models::{
        NetworkBlockModel, NetworkBlockScheduleEntryModel, NetworkBlockToSitePackModel,
        UserDefinedRuleModel,
    },
};
use crate::schema_main_db::{
    network_block_schedule_entries, network_block_to_site_pack, network_blocks, user_defined_rules,
};
use diesel::prelude::*;

impl EngineDB {
    pub fn save_network_block_snapshot(
        &self,
        block: &NetworkBlockModel,
        rules: &[UserDefinedRuleModel],
        schedule: &[NetworkBlockScheduleEntryModel],
        packs: &[NetworkBlockToSitePackModel],
        create: bool,
    ) -> Result<(), DBError> {
        let mut conn = self.get_db_connection()?;
        conn.transaction::<(), diesel::result::Error, _>(|conn| {
            if create {
                diesel::insert_into(network_blocks::table)
                    .values(block)
                    .execute(conn)?;
            } else {
                let updated = diesel::update(network_blocks::table.find(&block.id))
                    .set((
                        network_blocks::name.eq(&block.name),
                        network_blocks::icon.eq(&block.icon),
                        network_blocks::safe_search.eq(block.safe_search),
                    ))
                    .execute(conn)?;
                if updated != 1 {
                    return Err(diesel::result::Error::NotFound);
                }
            }

            diesel::delete(
                user_defined_rules::table
                    .filter(user_defined_rules::network_block_id.eq(&block.id)),
            )
            .execute(conn)?;
            for rule in rules {
                diesel::insert_into(user_defined_rules::table)
                    .values((
                        user_defined_rules::id.eq(&rule.id),
                        user_defined_rules::network_block_id.eq(&block.id),
                        user_defined_rules::rule.eq(&rule.rule),
                        user_defined_rules::rule_type.eq(&rule.rule_type),
                    ))
                    .execute(conn)?;
            }

            diesel::delete(
                network_block_schedule_entries::table
                    .filter(network_block_schedule_entries::network_block_id.eq(&block.id)),
            )
            .execute(conn)?;
            for entry in schedule {
                diesel::insert_into(network_block_schedule_entries::table)
                    .values((
                        network_block_schedule_entries::id.eq(&entry.id),
                        network_block_schedule_entries::network_block_id.eq(&block.id),
                        network_block_schedule_entries::start_time.eq(&entry.start_time),
                        network_block_schedule_entries::end_time.eq(&entry.end_time),
                        network_block_schedule_entries::schedule_days_bitmask
                            .eq(entry.schedule_days_bitmask),
                    ))
                    .execute(conn)?;
            }

            diesel::delete(
                network_block_to_site_pack::table
                    .filter(network_block_to_site_pack::network_block_id.eq(&block.id)),
            )
            .execute(conn)?;
            for connection in packs {
                diesel::insert_into(network_block_to_site_pack::table)
                    .values(connection)
                    .execute(conn)?;
            }
            Ok(())
        })
        .map_err(|error| DBError::UpdateError {
            table: "network_block_snapshot".into(),
            message: error.to_string(),
        })
    }
}
