use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use tauri::Manager;

use crate::{
    commands::{authorize_window, profiles::ProfileCommandState},
    error::CommandError,
    macos::glass,
    state::AppState,
    storage::preferences::{self, AppPreferences},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceInput {
    theme: String,
    reduce_transparency: bool,
}

#[tauri::command]
pub fn appearance_get(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
) -> Result<Option<AppPreferences>, CommandError> {
    authorize_window(window.label(), &["main"])?;
    let owner = active_owner(&app_state)?;
    let database = profile_state.database.lock().map_err(|_| state_error())?;
    preferences::get(&database, &owner).map_err(|_| state_error())
}

#[tauri::command]
pub fn appearance_save(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    input: AppearanceInput,
) -> Result<AppPreferences, CommandError> {
    authorize_window(window.label(), &["main"])?;
    let owner = active_owner(&app_state)?;
    let database = profile_state.database.lock().map_err(|_| state_error())?;
    preferences::save_theme(
        &database,
        &owner,
        &input.theme,
        if input.reduce_transparency {
            "reduce"
        } else {
            "system"
        },
        now_ms(),
    )
    .map_err(|_| CommandError::new("PREFERENCE_INVALID", "Appearance preference is invalid"))
}

#[tauri::command]
pub fn overlay_apply_material(
    window: tauri::Window,
    app: tauri::AppHandle,
    appearance: AppearanceInput,
) -> Result<(), CommandError> {
    authorize_window(window.label(), &["main", "overlay"])?;
    for label in ["main", "overlay"] {
        if let Some(target) = app.get_webview_window(label) {
            glass::apply_material(&target, appearance.reduce_transparency)?;
        }
    }
    Ok(())
}

fn active_owner(state: &AppState) -> Result<String, CommandError> {
    state
        .0
        .lock()
        .map_err(|_| state_error())?
        .active_user_id
        .clone()
        .ok_or_else(|| CommandError::new("AUTH_REQUIRED", "Sign in is required"))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn state_error() -> CommandError {
    CommandError::new("STATE_UNAVAILABLE", "Appearance preference is unavailable")
}
