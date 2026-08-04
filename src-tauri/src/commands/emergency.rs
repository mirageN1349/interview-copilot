use serde::Serialize;
use tauri::Emitter;

use crate::{
    commands::{
        authorize_window, capture,
        meetings::MeetingRuntimeSummary,
        presentation::{self, PresentationCommandState},
        profiles::ProfileCommandState,
    },
    error::CommandError,
    state::AppState,
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CancelPendingEvent {
    reason: &'static str,
}

#[tauri::command]
pub fn emergency_stop_all_fail_closed(
    window: tauri::Window,
    app: tauri::AppHandle,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    presentation_state: tauri::State<'_, PresentationCommandState>,
) -> Result<Option<MeetingRuntimeSummary>, CommandError> {
    authorize_window(window.label(), &["main", "overlay"])?;
    app.emit(
        "meeting://cancel-pending",
        CancelPendingEvent {
            reason: "local_stop",
        },
    )?;
    let restoration = presentation::restore_standard(&app, &presentation_state);
    let stopped = capture::emergency_stop_all(window, app, app_state, profile_state);
    match (stopped, restoration) {
        (Ok(value), Ok(_)) => Ok(value),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}
