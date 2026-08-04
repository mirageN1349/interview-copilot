use serde::Serialize;

use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{Emitter, Manager};

use crate::{
    commands::{authorize_window, meetings::MeetingRuntimeSummary, profiles::ProfileCommandState},
    error::CommandError,
    state::AppState,
    storage::meetings,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayDescriptor {
    display_id: u32,
    label: String,
    width: usize,
    height: usize,
    backing_scale: f64,
    is_primary: bool,
}

#[tauri::command]
pub fn displays_list(window: tauri::Window) -> Result<Vec<DisplayDescriptor>, CommandError> {
    authorize_window(window.label(), &["main", "overlay"])?;
    list_displays()
}

#[cfg(target_os = "macos")]
fn list_displays() -> Result<Vec<DisplayDescriptor>, CommandError> {
    use objc2_core_graphics::{
        CGDisplayBounds, CGDisplayPixelsHigh, CGDisplayPixelsWide, CGError, CGGetActiveDisplayList,
        CGMainDisplayID,
    };
    let mut ids = [0_u32; 16];
    let mut count = 0_u32;
    let result = unsafe { CGGetActiveDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut count) };
    if result != CGError::Success {
        return Err(CommandError::new(
            "DISPLAY_LIST_UNAVAILABLE",
            "Displays could not be listed",
        ));
    }
    let primary = CGMainDisplayID();
    Ok(ids[..count as usize]
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let bounds = CGDisplayBounds(*id);
            let logical_width = bounds.size.width.max(1.0);
            DisplayDescriptor {
                display_id: *id,
                label: (index + 1).to_string(),
                width: CGDisplayPixelsWide(*id),
                height: CGDisplayPixelsHigh(*id),
                backing_scale: CGDisplayPixelsWide(*id) as f64 / logical_width,
                is_primary: *id == primary,
            }
        })
        .collect())
}

#[cfg(not(target_os = "macos"))]
fn list_displays() -> Result<Vec<DisplayDescriptor>, CommandError> {
    Err(CommandError::new(
        "DISPLAY_LIST_UNAVAILABLE",
        "Display capture is available only on macOS",
    ))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRuntimeStatus {
    phase: &'static str,
    speech_detected: bool,
    artifact_persisted: bool,
    screen_samples: u64,
    system_audio_samples: u64,
    microphone_samples: u64,
}

#[tauri::command]
pub fn capture_start(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<CaptureRuntimeStatus, CommandError> {
    authorize_window(window.label(), &["main"])?;
    let mut runtime = app_state
        .0
        .lock()
        .map_err(|_| CommandError::new("STATE_UNAVAILABLE", "Capture state is unavailable"))?;
    if runtime.active_meeting_id.as_deref() != Some(&meeting_id)
        || runtime.capture_runtime.is_none()
    {
        return Err(CommandError::new(
            "MEETING_NOT_RUNNING",
            "The meeting capture is not active",
        ));
    }
    let owner = runtime
        .active_user_id
        .clone()
        .ok_or_else(|| CommandError::new("AUTH_REQUIRED", "Sign in is required"))?;
    let capture = runtime.capture_runtime.as_mut().expect("checked above");
    #[cfg(target_os = "macos")]
    capture
        .start_native(window.app_handle().clone(), owner, meeting_id)
        .map_err(|_| {
            CommandError::new(
                "CAPTURE_START_FAILED",
                "Screen and audio capture could not be started",
            )
        })?;
    #[cfg(not(target_os = "macos"))]
    let _ = (capture, owner, meeting_id);
    #[cfg(target_os = "macos")]
    let (screen_samples, system_audio_samples, microphone_samples) = capture.delivered_samples();
    #[cfg(not(target_os = "macos"))]
    let (screen_samples, system_audio_samples, microphone_samples) = (0, 0, 0);
    Ok(CaptureRuntimeStatus {
        phase: "listening",
        speech_detected: false,
        artifact_persisted: false,
        screen_samples,
        system_audio_samples,
        microphone_samples,
    })
}

#[tauri::command]
pub fn emergency_stop_all(
    window: tauri::Window,
    app: tauri::AppHandle,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
) -> Result<Option<MeetingRuntimeSummary>, CommandError> {
    authorize_window(window.label(), &["main", "overlay"])?;
    let (meeting_id, owner) = {
        let mut runtime = app_state.0.lock().map_err(|_| state_error())?;
        if let Some(capture) = runtime.capture_runtime.as_mut() {
            capture.stop();
        }
        (
            runtime.active_meeting_id.clone(),
            runtime.active_user_id.clone(),
        )
    };
    let (Some(meeting_id), Some(owner)) = (meeting_id, owner) else {
        return Ok(None);
    };
    let now = now_ms();
    let mut database = profile_state.database.lock().map_err(|_| state_error())?;
    let meeting = meetings::finalize(&mut database, &owner, &meeting_id, now).map_err(|_| {
        CommandError::new("MEETING_STOP_FAILED", "The meeting could not be finalized")
    })?;
    crate::storage::history::index_owner(&mut database, &owner).map_err(|_| {
        CommandError::new(
            "HISTORY_INDEX_FAILED",
            "Meeting history could not be indexed",
        )
    })?;
    drop(database);
    let mut runtime = app_state.0.lock().map_err(|_| state_error())?;
    runtime
        .audit
        .append(now, "emergency_stop", "stopped", "LOCAL_STOP");
    if !runtime.audit.verify() {
        return Err(CommandError::new(
            "AUDIT_INTEGRITY_FAILED",
            "The stop result could not be verified",
        ));
    }
    runtime.active_meeting_id = None;
    runtime.capture_runtime = None;
    drop(runtime);
    let summary = MeetingRuntimeSummary::from_stopped(meeting);
    app.emit("meeting://state", &summary)
        .map_err(CommandError::from)?;
    Ok(Some(summary))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn state_error() -> CommandError {
    CommandError::new("STATE_UNAVAILABLE", "Capture state is unavailable").retryable(None)
}
