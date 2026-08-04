use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::{
    commands::{authorize_window, overlay::exact_matrix_approval, profiles::ProfileCommandState},
    error::CommandError,
    macos::presentation::{self, PresentationMode, PresentationState},
    state::AppState,
};

#[derive(Debug, Default)]
pub struct PresentationCommandState(pub Mutex<PresentationState>);

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationProfileSummary {
    id: &'static str,
    display_name: &'static str,
    mode: PresentationMode,
    available: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPresentationInput {
    meeting_id: String,
    profile_id: String,
    matrix_row_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationRuntimeSummary {
    profile_id: String,
    mode: PresentationMode,
    dock_visible: bool,
    audit_id: String,
}

#[tauri::command]
pub fn presentation_profiles_list(
    window: tauri::Window,
) -> Result<Vec<PresentationProfileSummary>, CommandError> {
    authorize_window(window.label(), &["main"])?;
    Ok(vec![
        PresentationProfileSummary {
            id: "standard",
            display_name: "Standard",
            mode: PresentationMode::Standard,
            available: true,
        },
        PresentationProfileSummary {
            id: "generic",
            display_name: "Generic",
            mode: PresentationMode::Generic,
            // No signed reference row or approved asset is bundled yet.
            available: false,
        },
    ])
}

#[tauri::command]
pub fn presentation_profile_apply(
    window: tauri::Window,
    app: tauri::AppHandle,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    presentation_state: tauri::State<'_, PresentationCommandState>,
    input: ApplyPresentationInput,
) -> Result<PresentationRuntimeSummary, CommandError> {
    authorize_window(window.label(), &["main"])?;
    let mode = match input.profile_id.as_str() {
        "standard" => PresentationMode::Standard,
        "generic" => PresentationMode::Generic,
        _ => {
            return Err(CommandError::new(
                "PRESENTATION_PROFILE_INVALID",
                "This presentation option is unavailable",
            ));
        }
    };
    let active = app_state
        .0
        .lock()
        .map_err(|_| state_error())?
        .active_meeting_id
        .clone();
    if active.as_deref() != Some(&input.meeting_id) {
        return Err(CommandError::new(
            "MEETING_NOT_RUNNING",
            "The meeting is unavailable",
        ));
    }
    if mode == PresentationMode::Generic
        && !exact_matrix_approval(
            &profile_state,
            &input.meeting_id,
            input.matrix_row_id.as_deref(),
        )?
    {
        return Err(CommandError::new(
            "ADVERSARIAL_MATRIX_UNSUPPORTED",
            "This presentation option is unavailable",
        ));
    }
    if mode == PresentationMode::Generic {
        let asset = app
            .path()
            .resource_dir()
            .map_err(CommandError::from)?
            .join("icons/presentation/generic.icns");
        if !presentation::generic_asset_approved(&asset) {
            return Err(CommandError::new(
                "PRESENTATION_ASSET_UNAPPROVED",
                "This presentation option is unavailable",
            ));
        }
    }

    presentation::apply_activation_policy(&app, mode).map_err(CommandError::from)?;
    let mut state = presentation_state.0.lock().map_err(|_| state_error())?;
    state
        .apply(mode)
        .map_err(|code| CommandError::new(code, "This presentation option is unavailable"))?;
    let mut runtime = app_state.0.lock().map_err(|_| state_error())?;
    let audit = runtime.audit.append(
        now_ms(),
        "presentation_profile",
        "succeeded",
        if mode == PresentationMode::Standard {
            "STANDARD_RESTORED"
        } else {
            "GENERIC_APPLIED"
        },
    );
    let audit_id = format!("audit-{}", audit.sequence);
    if !runtime.audit.verify() {
        drop(runtime);
        let _ = presentation::apply_activation_policy(&app, PresentationMode::Standard);
        state.restore_standard();
        return Err(CommandError::new(
            "AUDIT_INTEGRITY_FAILED",
            "This presentation option is unavailable",
        ));
    }
    Ok(PresentationRuntimeSummary {
        profile_id: input.profile_id,
        mode,
        dock_visible: mode == PresentationMode::Standard,
        audit_id,
    })
}

pub fn restore_standard(
    app: &tauri::AppHandle,
    state: &PresentationCommandState,
) -> Result<bool, CommandError> {
    let restored = state
        .0
        .lock()
        .map_err(|_| state_error())?
        .restore_standard();
    presentation::apply_activation_policy(app, PresentationMode::Standard)
        .map_err(CommandError::from)?;
    Ok(restored)
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn state_error() -> CommandError {
    CommandError::new(
        "STATE_UNAVAILABLE",
        "This presentation option is unavailable",
    )
}
