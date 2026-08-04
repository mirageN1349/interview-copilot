use std::collections::HashMap;

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::{
    commands::{authorize_window, profiles::ProfileCommandState},
    error::CommandError,
    macos::{hotkeys, overlay},
    state::AppState,
    storage::capture_matrix,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingOverlayInput {
    meeting_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveOverlayInput {
    meeting_id: String,
    dx: i32,
    dy: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractiveInput {
    interactive: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureProtectionInput {
    meeting_id: String,
    enabled: bool,
    matrix_row_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayRuntimeSummary {
    meeting_id: String,
    visible: bool,
    interactive: bool,
}

#[tauri::command]
pub fn overlay_open(
    window: tauri::Window,
    app: tauri::AppHandle,
    app_state: tauri::State<'_, AppState>,
    input: MeetingOverlayInput,
) -> Result<OverlayRuntimeSummary, CommandError> {
    authorize_window(window.label(), &["main"])?;
    authorize_meeting(&app_state, &input.meeting_id)?;
    let overlay_window = app.get_webview_window("overlay").ok_or_else(|| {
        CommandError::new("OVERLAY_UNAVAILABLE", "The assistant panel is unavailable")
    })?;
    overlay::open_for_meeting(&overlay_window, &input.meeting_id)?;
    Ok(OverlayRuntimeSummary {
        meeting_id: input.meeting_id,
        visible: true,
        interactive: true,
    })
}

#[tauri::command]
pub fn overlay_show(
    window: tauri::Window,
    app: tauri::AppHandle,
    app_state: tauri::State<'_, AppState>,
    input: MeetingOverlayInput,
) -> Result<OverlayRuntimeSummary, CommandError> {
    authorize_window(window.label(), &["main", "overlay"])?;
    authorize_meeting(&app_state, &input.meeting_id)?;
    let overlay_window = app.get_webview_window("overlay").ok_or_else(|| {
        CommandError::new("OVERLAY_UNAVAILABLE", "The assistant panel is unavailable")
    })?;
    overlay::set_interactive(&overlay_window, true)?;
    Ok(OverlayRuntimeSummary {
        meeting_id: input.meeting_id,
        visible: true,
        interactive: true,
    })
}

#[tauri::command]
pub fn overlay_ready(
    window: tauri::Window,
    app: tauri::AppHandle,
    app_state: tauri::State<'_, AppState>,
    input: MeetingOverlayInput,
) -> Result<OverlayRuntimeSummary, CommandError> {
    authorize_window(window.label(), &["overlay"])?;
    authorize_meeting(&app_state, &input.meeting_id)?;
    let overlay_window = app.get_webview_window("overlay").ok_or_else(|| {
        CommandError::new("OVERLAY_UNAVAILABLE", "The assistant panel is unavailable")
    })?;
    overlay::show_ready(&overlay_window)?;
    Ok(OverlayRuntimeSummary {
        meeting_id: input.meeting_id,
        visible: true,
        interactive: true,
    })
}

#[tauri::command]
pub fn overlay_hide(
    window: tauri::Window,
    app: tauri::AppHandle,
    app_state: tauri::State<'_, AppState>,
    input: MeetingOverlayInput,
) -> Result<OverlayRuntimeSummary, CommandError> {
    authorize_window(window.label(), &["main", "overlay"])?;
    authorize_meeting(&app_state, &input.meeting_id)?;
    app.get_webview_window("overlay")
        .ok_or_else(|| {
            CommandError::new("OVERLAY_UNAVAILABLE", "The assistant panel is unavailable")
        })?
        .hide()?;
    Ok(OverlayRuntimeSummary {
        meeting_id: input.meeting_id,
        visible: false,
        interactive: false,
    })
}

#[tauri::command]
pub fn overlay_move(
    window: tauri::Window,
    app: tauri::AppHandle,
    app_state: tauri::State<'_, AppState>,
    input: MoveOverlayInput,
) -> Result<overlay::OverlayPosition, CommandError> {
    authorize_window(window.label(), &["overlay"])?;
    authorize_meeting(&app_state, &input.meeting_id)?;
    let overlay_window = app.get_webview_window("overlay").ok_or_else(|| {
        CommandError::new("OVERLAY_UNAVAILABLE", "The assistant panel is unavailable")
    })?;
    overlay::move_by(
        &overlay_window,
        input.dx.clamp(-96, 96),
        input.dy.clamp(-96, 96),
    )
}

#[tauri::command]
pub fn overlay_set_interactive(
    window: tauri::Window,
    app: tauri::AppHandle,
    input: InteractiveInput,
) -> Result<(), CommandError> {
    authorize_window(window.label(), &["overlay"])?;
    let overlay_window = app.get_webview_window("overlay").ok_or_else(|| {
        CommandError::new("OVERLAY_UNAVAILABLE", "The assistant panel is unavailable")
    })?;
    overlay::set_interactive(&overlay_window, input.interactive)
}

#[tauri::command]
pub fn overlay_set_capture_protection(
    window: tauri::Window,
    app: tauri::AppHandle,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    input: CaptureProtectionInput,
) -> Result<overlay::CaptureProtectionState, CommandError> {
    authorize_window(window.label(), &["main"])?;
    authorize_meeting(&app_state, &input.meeting_id)?;
    let exact_approved = if input.enabled {
        exact_matrix_approval(
            &profile_state,
            &input.meeting_id,
            input.matrix_row_id.as_deref(),
        )?
    } else {
        true
    };
    let overlay_window = app.get_webview_window("overlay").ok_or_else(|| {
        CommandError::new("OVERLAY_UNAVAILABLE", "The assistant panel is unavailable")
    })?;
    overlay::set_capture_protection(&overlay_window, input.enabled, exact_approved)
}

#[tauri::command]
pub fn hotkeys_register(
    window: tauri::Window,
    app: tauri::AppHandle,
    bindings: HashMap<String, String>,
) -> Result<Vec<hotkeys::HotkeyRegistration>, CommandError> {
    authorize_window(window.label(), &["main"])?;
    hotkeys::register(&app, bindings)
}

#[tauri::command]
pub fn hotkeys_unregister_all(
    window: tauri::Window,
    app: tauri::AppHandle,
) -> Result<(), CommandError> {
    authorize_window(window.label(), &["main"])?;
    hotkeys::unregister_all(&app)
}

fn authorize_meeting(state: &AppState, meeting_id: &str) -> Result<(), CommandError> {
    let runtime = state
        .0
        .lock()
        .map_err(|_| CommandError::new("STATE_UNAVAILABLE", "Overlay state is unavailable"))?;
    (runtime.active_meeting_id.as_deref() == Some(meeting_id))
        .then_some(())
        .ok_or_else(|| {
            CommandError::new(
                "MEETING_NOT_RUNNING",
                "The panel is not bound to this meeting",
            )
        })
}

pub(crate) fn exact_matrix_approval(
    state: &ProfileCommandState,
    meeting_id: &str,
    requested_row_id: Option<&str>,
) -> Result<bool, CommandError> {
    let Some(requested_row_id) = requested_row_id else {
        return Ok(false);
    };
    let database = state.database.lock().map_err(|_| {
        CommandError::new("STATE_UNAVAILABLE", "The assistant panel is unavailable")
    })?;
    let selected: Option<(String, Option<String>)> = database
        .connection()
        .query_row(
            "SELECT m.mode, cc.matrix_row_id FROM meetings m
             JOIN capture_configurations cc ON cc.meeting_id = m.id WHERE m.id = ?1",
            [meeting_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| CommandError::new("MEETING_NOT_RUNNING", "The meeting is unavailable"))?;
    let Some((mode, selected_row_id)) = selected else {
        return Ok(false);
    };
    if mode != "adversarial_lab" {
        return Ok(true);
    }
    if selected_row_id.as_deref() != Some(requested_row_id) {
        return Ok(false);
    }
    let Some(recorded) =
        capture_matrix::find(database.connection(), requested_row_id).map_err(|_| {
            CommandError::new(
                "ADVERSARIAL_MATRIX_UNSUPPORTED",
                "This presentation mode is unavailable",
            )
        })?
    else {
        return Ok(false);
    };
    let Some(runtime) =
        crate::macos::capture_probe::runtime_environment_from(&recorded.environment)
    else {
        return Ok(false);
    };
    capture_matrix::find_exact_approved(database.connection(), requested_row_id, &runtime)
        .map(|row| row.is_some())
        .map_err(|_| {
            CommandError::new(
                "ADVERSARIAL_MATRIX_UNSUPPORTED",
                "This presentation mode is unavailable",
            )
        })
}
