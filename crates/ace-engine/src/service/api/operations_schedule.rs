use crate::core::{NetworkBlockId, ScheduleEntry as CoreScheduleEntry, ScheduleEntryId};
use chrono::Weekday;

use crate::service::api::{AppState, error::ApiError, schemas::*};

pub(super) async fn get_schedule_entries(
    state: AppState,
    network_block_id: uuid::Uuid,
) -> Result<GetScheduleEntriesResponse, ApiError> {
    let manager = state.network_manager.read().await;
    let list_id = NetworkBlockId::from(network_block_id);
    let entries_core = manager.get_schedule_entries_of(&list_id)?;

    let entries = entries_core.iter().map(ScheduleEntrySchema::from).collect();

    Ok(GetScheduleEntriesResponse {
        schedule_entries: entries,
    })
}

pub(super) async fn add_schedule_entries(
    state: AppState,
    network_block_id: uuid::Uuid,
    payload: AddScheduleEntriesRequest,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;

    let mut core_entries = Vec::new();
    for entry in payload.schedule_entries {
        let mut days = 0;
        if entry.monday {
            days |= 1 << Weekday::Mon as u8;
        }
        if entry.tuesday {
            days |= 1 << Weekday::Tue as u8;
        }
        if entry.wednesday {
            days |= 1 << Weekday::Wed as u8;
        }
        if entry.thursday {
            days |= 1 << Weekday::Thu as u8;
        }
        if entry.friday {
            days |= 1 << Weekday::Fri as u8;
        }
        if entry.saturday {
            days |= 1 << Weekday::Sat as u8;
        }
        if entry.sunday {
            days |= 1 << Weekday::Sun as u8;
        }

        let schedule_entry = CoreScheduleEntry::from_raw(
            uuid::Uuid::now_v7().to_string(), // New entry
            network_block_id.to_string(),
            &entry.start_time.format("%H:%M:%S").to_string(),
            &entry.end_time.format("%H:%M:%S").to_string(),
            days,
        )
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;

        core_entries.push(schedule_entry);
    }

    let list_id = NetworkBlockId::from(network_block_id);
    manager.add_schedule_entries(&list_id, core_entries)?;
    Ok(())
}

pub(super) async fn edit_schedule_entry(
    state: AppState,
    schedule_entry_id: uuid::Uuid,
    entry: BaseScheduleEntrySchema,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;

    let mut days = 0;
    if entry.monday {
        days |= 1 << Weekday::Mon as u8;
    }
    if entry.tuesday {
        days |= 1 << Weekday::Tue as u8;
    }
    if entry.wednesday {
        days |= 1 << Weekday::Wed as u8;
    }
    if entry.thursday {
        days |= 1 << Weekday::Thu as u8;
    }
    if entry.friday {
        days |= 1 << Weekday::Fri as u8;
    }
    if entry.saturday {
        days |= 1 << Weekday::Sat as u8;
    }
    if entry.sunday {
        days |= 1 << Weekday::Sun as u8;
    }

    let schedule_entry = CoreScheduleEntry::from_raw(
        schedule_entry_id.to_string(),
        uuid::Uuid::nil().to_string(),
        &entry.start_time.format("%H:%M:%S").to_string(),
        &entry.end_time.format("%H:%M:%S").to_string(),
        days,
    )
    .map_err(|error| ApiError::BadRequest(error.to_string()))?;

    manager.edit_schedule_entry(schedule_entry)?;
    Ok(())
}

pub(super) async fn delete_schedule_entry(
    state: AppState,
    schedule_entry_id: uuid::Uuid,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.delete_schedule_entry(ScheduleEntryId::from(schedule_entry_id))?;
    Ok(())
}
