use crate::core::{NetworkBlockId, ScheduleEntry as CoreScheduleEntry, ScheduleEntryId};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, put},
};
use chrono::Weekday;

use crate::service::api::{AppState, error::ApiError, schemas::*};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/network-blocks/{network_block_id}/schedule",
            get(get_schedule_entries).post(add_schedule_entries),
        )
        .route(
            "/schedule/{schedule_entry_id}",
            put(edit_schedule_entry).delete(delete_schedule_entry),
        )
}

#[utoipa::path(
    get,
    path = "/network-blocks/{network_block_id}/schedule",
    tag = "Schedule",
    params(
        ("network_block_id" = uuid::Uuid, Path, description = "Network Block ID")
    ),
    responses(
        (status = 200, description = "Get schedule entries for a network list", body = GetScheduleEntriesResponse)
    )
)]
pub(super) async fn get_schedule_entries(
    State(state): State<AppState>,
    Path(network_block_id): Path<uuid::Uuid>,
) -> Result<Json<GetScheduleEntriesResponse>, ApiError> {
    let mut manager = state.network_manager.write().await;
    let list_id = NetworkBlockId::from(network_block_id);
    let entries_core = manager.get_schedule_entries_of(&list_id)?;

    let entries = entries_core.iter().map(ScheduleEntrySchema::from).collect();

    Ok(Json(GetScheduleEntriesResponse {
        schedule_entries: entries,
    }))
}

#[utoipa::path(
    post,
    path = "/network-blocks/{network_block_id}/schedule",
    tag = "Schedule",
    params(
        ("network_block_id" = uuid::Uuid, Path, description = "Network Block ID")
    ),
    request_body = AddScheduleEntriesRequest,
    responses(
        (status = 200, description = "Add schedule entries to a network list")
    )
)]
pub(super) async fn add_schedule_entries(
    State(state): State<AppState>,
    Path(network_block_id): Path<uuid::Uuid>,
    Json(payload): Json<AddScheduleEntriesRequest>,
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

#[utoipa::path(
    put,
    path = "/schedule/{schedule_entry_id}",
    tag = "Schedule",
    params(
        ("schedule_entry_id" = uuid::Uuid, Path, description = "Schedule Entry ID")
    ),
    request_body = BaseScheduleEntrySchema,
    responses(
        (status = 200, description = "Edit a schedule entry")
    )
)]
pub(super) async fn edit_schedule_entry(
    State(state): State<AppState>,
    Path(schedule_entry_id): Path<uuid::Uuid>,
    Json(entry): Json<BaseScheduleEntrySchema>,
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

#[utoipa::path(
    delete,
    path = "/schedule/{schedule_entry_id}",
    tag = "Schedule",
    params(
        ("schedule_entry_id" = uuid::Uuid, Path, description = "Schedule Entry ID")
    ),
    responses(
        (status = 200, description = "Delete a schedule entry")
    )
)]
pub(super) async fn delete_schedule_entry(
    State(state): State<AppState>,
    Path(schedule_entry_id): Path<uuid::Uuid>,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.delete_schedule_entry(ScheduleEntryId::from(schedule_entry_id))?;
    Ok(())
}
