use chrono::Local;

use crate::core::NetworkBlockId;
use crate::service::api::{AppState, error::ApiError, schemas::*};

pub(super) async fn get_dashboard_snapshot(
    state: AppState,
) -> Result<DashboardSnapshotResponse, ApiError> {
    let manager = state.network_manager.read().await;
    let db = manager.get_db();
    let now = Local::now();

    // ── Gather all network blocks with their nested data ──
    let block_ids: Vec<NetworkBlockId> = {
        let blocks = manager.get_all_network_blocks();
        blocks.iter().map(|b| b.get_id().clone()).collect()
    };

    let mut snapshots: Vec<NetworkBlockSnapshot> = Vec::with_capacity(block_ids.len());

    for block_id_ref in &block_ids {
        // Extract immutable data first, then drop the immutable borrow
        // before calling mutable methods on the manager.
        let block_data = {
            let network_block = manager
                .get_network_block_from(block_id_ref)
                .ok_or_else(|| {
                    ApiError::NotFound(format!(
                        "Network block {} vanished during snapshot",
                        block_id_ref
                    ))
                })?;

            (
                network_block.get_name(),
                network_block.get_icon(),
                network_block.is_enabled(),
                network_block.is_active(&now),
                network_block.is_locked(&now),
                network_block.is_safe_search_enabled(),
                network_block.get_lock().get_state(),
            )
        };
        // Immutable borrow of `manager` is now dropped.

        let (block_name, block_icon, enabled, is_active, is_locked, safe_search, core_lock_state) =
            block_data;

        // Convert block_id_ref to owned uuid::Uuid for schema usage.
        // NetworkBlockId: Deref<Target = Uuid> + Copy is not derived,
        // so we use AsRef<Uuid> and deref the result.
        let block_id_owned = block_id_ref.clone();
        let block_uuid: uuid::Uuid = *block_id_owned.as_ref();

        // Lock info from the same in-memory policy snapshot.
        let lock = manager.get_lock_of(block_id_ref).ok();
        let lock_schema =
            lock.map(|lock_ref| NetworkListLockSchema::from_lock(lock_ref, block_uuid, is_active));

        // Schedule entries
        let schedule_entries = manager
            .get_schedule_entries_of(block_id_ref)
            .map(|entries| {
                entries
                    .iter()
                    .map(ScheduleEntrySchema::from)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Site pack connections count — only packs with usable local rules
        // count as coverage. A connected-but-unsynced pack (metadata row with
        // no data yet) enforces nothing, so reporting it would be a "ghost"
        // claim in the UI (onboarding, block cards).
        let site_pack_connection_count = manager.count_synced_site_packs_of(block_id_ref);

        // User-defined rules count (already in memory)
        let user_rules_count = manager
            .get_network_block_from(block_id_ref)
            .map(|block| block.get_user_defined_rules().rules().len())
            .unwrap_or(0);

        snapshots.push(NetworkBlockSnapshot {
            block: NetworkBlockSchema {
                id: block_uuid,
                name: block_name,
                enabled,
                is_active,
                is_locked,
                lock_state: core_lock_state.into(),
                safe_search,
                icon: block_icon,
            },
            lock: lock_schema,
            schedule_entries,
            site_pack_connection_count,
            user_rules_count,
        });
    }

    // ── Resistance mode settings ──
    let resistance_mode_settings = {
        let engine = manager.get_resistance_mode();
        let punishment = engine.get_punishment();
        let current_intensity = manager.get_effective_intensity();
        let entries = engine.script_tree.get_all_script_entries();

        Some(ResistanceModeSettings {
            enabled: engine.enabled,
            is_locked: engine.is_locked(),
            lock_state: engine.lock.get_state().into(),
            lock_end: engine
                .lock
                .get_end_time()
                .map(|dt: chrono::DateTime<chrono::Local>| dt.naive_local().to_string()),
            mode: punishment.mode,
            current_intensity,
            static_intensity: punishment.static_intensity,
            scripts_version: engine.scripts_version.clone(),
            scripts_loaded: !entries.is_empty(),
            scripts_total: entries.len() as u32,
            scripts_enabled: entries.iter().filter(|entry| entry.enabled).count() as u32,
            pro_eligible: manager.is_pro_user_cached(),
            enforcement_status: manager.resistance_enforcement_status(),
        })
    };

    // Database I/O happens after releasing the policy lock entirely.
    drop(manager);
    let blocked_attempts = db
        .get_blocked_attempts_for_date(chrono::Utc::now().date_naive())
        .unwrap_or(0);

    Ok(DashboardSnapshotResponse {
        blocks: snapshots,
        blocked_attempts,
        resistance_mode_settings,
    })
}
