use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{
    commands::{authorize_window, profiles::ProfileCommandState},
    error::CommandError,
    state::AppState,
    storage::{
        StorageKey,
        history::{self, HistoryExport, HistoryPage, HistorySearch, MeetingHistoryDetail},
        retention,
    },
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingIdInput {
    pub meeting_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionResult {
    pub meeting_id: String,
    pub files_pending: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionResult {
    pub meetings_processed: usize,
    pub files_pending: usize,
    pub orphan_count: usize,
}

#[tauri::command]
pub fn meeting_search(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    input: HistorySearch,
) -> Result<HistoryPage, CommandError> {
    authorize_window(window.label(), &["main"])?;
    let owner = active_user(&app_state)?;
    let database = profile_state.database.lock().map_err(|_| state_error())?;
    history::search(&database, &owner, &input, now_ms()).map_err(history_error)
}

#[tauri::command]
pub fn meeting_history_get(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    meeting_id: String,
) -> Result<MeetingHistoryDetail, CommandError> {
    authorize_window(window.label(), &["main"])?;
    let owner = active_user(&app_state)?;
    let database = profile_state.database.lock().map_err(|_| state_error())?;
    history::detail(&database, &owner, &meeting_id, now_ms()).map_err(history_error)
}

#[tauri::command]
pub fn meeting_delete_content(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    input: MeetingIdInput,
) -> Result<DeletionResult, CommandError> {
    authorize_window(window.label(), &["main"])?;
    let owner = active_user(&app_state)?;
    let mut database = profile_state.database.lock().map_err(|_| state_error())?;
    let plan =
        retention::prepare_meeting_deletion(&mut database, &owner, &input.meeting_id, now_ms())
            .map_err(history_error)?;
    let (deleted, files_pending) = delete_files(&profile_state, &plan.storage_keys);
    retention::complete_file_cleanup(&mut database, &plan.meeting_id, &deleted)
        .map_err(history_error)?;
    Ok(DeletionResult {
        meeting_id: plan.meeting_id,
        files_pending,
    })
}

#[tauri::command]
pub fn meeting_export(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    input: MeetingIdInput,
) -> Result<HistoryExport, CommandError> {
    authorize_window(window.label(), &["main"])?;
    let (owner, policy_allows) = {
        let runtime = app_state.0.lock().map_err(|_| state_error())?;
        let owner = runtime.active_user_id.clone().ok_or_else(|| {
            CommandError::new("AUTH_REQUIRED", "Sign in to export meeting history")
        })?;
        let policy_allows = runtime.policy.as_ref().is_some_and(|policy| {
            policy.user_id == owner && policy.is_fresh(now_ms()) && policy.allow_export
        });
        (owner, policy_allows)
    };
    let mut database = profile_state.database.lock().map_err(|_| state_error())?;
    history::export(
        &mut database,
        &owner,
        &input.meeting_id,
        policy_allows,
        now_ms(),
    )
    .map_err(history_error)
}

#[tauri::command]
pub fn retention_run(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
) -> Result<RetentionResult, CommandError> {
    authorize_window(window.label(), &["main"])?;
    active_user(&app_state)?;
    let mut database = profile_state.database.lock().map_err(|_| state_error())?;
    let plans = retention::prepare_due(&mut database, now_ms()).map_err(history_error)?;
    let mut files_pending = 0;
    for plan in &plans {
        let (deleted, pending) = delete_files(&profile_state, &plan.storage_keys);
        files_pending += pending;
        retention::complete_file_cleanup(&mut database, &plan.meeting_id, &deleted)
            .map_err(history_error)?;
    }
    let orphan_count = retention::verify_orphans(&database)
        .map_err(history_error)?
        .len();
    Ok(RetentionResult {
        meetings_processed: plans.len(),
        files_pending,
        orphan_count,
    })
}

#[tauri::command]
pub fn audit_verify_chain(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
) -> Result<bool, CommandError> {
    authorize_window(window.label(), &["main"])?;
    active_user(&app_state)?;
    let database = profile_state.database.lock().map_err(|_| state_error())?;
    history::verify_audit_chain(&database).map_err(history_error)
}

fn delete_files(
    profile_state: &ProfileCommandState,
    storage_keys: &[String],
) -> (Vec<String>, usize) {
    let mut deleted = Vec::new();
    let mut pending = 0;
    for raw_key in storage_keys {
        let Ok(key) = StorageKey::parse(raw_key.clone()) else {
            pending += 1;
            continue;
        };
        match profile_state.files.delete(&key) {
            Ok(()) => deleted.push(raw_key.clone()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                deleted.push(raw_key.clone());
            }
            Err(_) => pending += 1,
        }
    }
    (deleted, pending)
}

fn active_user(state: &AppState) -> Result<String, CommandError> {
    state
        .0
        .lock()
        .map_err(|_| state_error())?
        .active_user_id
        .clone()
        .ok_or_else(|| CommandError::new("AUTH_REQUIRED", "Sign in to manage meeting history"))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn state_error() -> CommandError {
    CommandError::new(
        "STATE_UNAVAILABLE",
        "Meeting history is temporarily unavailable",
    )
    .retryable(None)
}

fn history_error(error: history::HistoryError) -> CommandError {
    match error {
        history::HistoryError::Invalid(message) => CommandError::new("HISTORY_INVALID", message),
        history::HistoryError::NotFound => {
            CommandError::new("MEETING_NOT_FOUND", "Meeting history was not found")
        }
        history::HistoryError::Forbidden => CommandError::new(
            "EXPORT_NOT_ALLOWED",
            "This account cannot export meeting history",
        ),
        history::HistoryError::AuditIntegrity => CommandError::new(
            "AUDIT_INTEGRITY_FAILED",
            "Export is unavailable until local history integrity is restored",
        ),
        history::HistoryError::ExportTooLarge => CommandError::new(
            "EXPORT_TOO_LARGE",
            "This meeting is too large for the bounded export",
        ),
        history::HistoryError::Database(_) => CommandError::new(
            "HISTORY_STORAGE_FAILED",
            "The meeting history operation could not be completed",
        )
        .retryable(None),
    }
}
