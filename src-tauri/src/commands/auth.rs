use serde::Deserialize;

use crate::{
    commands::{authorize_window, profiles::ProfileCommandState},
    error::CommandError,
    security::policy::{KillSwitch, SafetyPolicySnapshot},
    state::AppState,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSessionInput {
    user_id: String,
    email: String,
    name: String,
}

#[tauri::command]
pub fn auth_session_sync(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    input: NativeSessionInput,
) -> Result<(), CommandError> {
    authorize_window(window.label(), &["main"])?;
    validate_demo_identity(&input)?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let database = profile_state.database.lock().map_err(|_| state_error())?;
    database.connection().execute(
            "INSERT INTO users(id, email, display_name, roles_json, device_ids_json, status, last_authenticated_at_ms) \
             VALUES (?1, ?2, ?3, '[]', '[]', 'active', CAST(unixepoch('subsec') * 1000 AS INTEGER)) \
             ON CONFLICT(id) DO UPDATE SET email = excluded.email, display_name = excluded.display_name, \
             status = 'active', last_authenticated_at_ms = excluded.last_authenticated_at_ms",
            (&input.user_id, &input.email, &input.name),
        )
        .map_err(|_| CommandError::new("AUTH_SESSION_SYNC_FAILED", "The local session could not be prepared"))?;
    database.connection().execute(
        "INSERT INTO launch_policies(id, title, purpose, owner_user_id, status, environment_id, \
            approved_device_ids_json, adversarial_approved, retention_days, starts_at_ms, expires_at_ms, approved_by, approved_at_ms) \
         VALUES ('default-meeting-policy', 'Standard meetings', 'Allow standard interview meetings for the signed-in demo user', \
            ?1, 'active', 'local-demo', '[\"managed-mac-01\"]', 0, 30, ?2, ?3, ?1, ?2) \
         ON CONFLICT(id) DO UPDATE SET owner_user_id = excluded.owner_user_id, status = 'active', \
            starts_at_ms = excluded.starts_at_ms, expires_at_ms = excluded.expires_at_ms",
        (&input.user_id, now_ms - 60_000, now_ms + 30 * 24 * 60 * 60_000),
    ).map_err(|_| CommandError::new("AUTH_SESSION_SYNC_FAILED", "The local meeting setup could not be prepared"))?;
    database.connection().execute(
        "INSERT OR REPLACE INTO participant_consents(id, launch_policy_id, participant_label, consent_artifact_id, scope_json, signed_at_ms, revoked_at_ms) \
         VALUES ('default-consent', 'default-meeting-policy', 'Meeting participant', 'default-consent-artifact', \
            '[\"audio\",\"screen\",\"transcript\",\"model_processing\"]', ?1, NULL)",
        [now_ms],
    ).map_err(|_| CommandError::new("AUTH_SESSION_SYNC_FAILED", "The local meeting setup could not be prepared"))?;
    drop(database);

    app_state
        .0
        .lock()
        .map_err(|_| state_error())?
        .active_user_id = Some(input.user_id.clone());
    app_state.0.lock().map_err(|_| state_error())?.policy = Some(SafetyPolicySnapshot {
        policy_version: "demo-v1".to_owned(),
        user_id: input.user_id,
        device_id: "managed-mac-01".to_owned(),
        environment_id: "local-demo".to_owned(),
        allow_adversarial: false,
        allow_export: false,
        kill_switch: KillSwitch::Clear,
        expires_at_ms: now_ms + 30 * 24 * 60 * 60_000,
        verified: true,
    });
    Ok(())
}

#[tauri::command]
pub fn auth_session_clear(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    authorize_window(window.label(), &["main"])?;
    app_state
        .0
        .lock()
        .map_err(|_| state_error())?
        .active_user_id = None;
    let mut runtime = app_state.0.lock().map_err(|_| state_error())?;
    runtime.policy = None;
    runtime.active_meeting_id = None;
    runtime.capture_runtime = None;
    Ok(())
}

fn validate_demo_identity(input: &NativeSessionInput) -> Result<(), CommandError> {
    if input.user_id != "019-user"
        || !input.email.trim().eq_ignore_ascii_case("user@example.test")
        || input.name.trim().is_empty()
        || input.name.chars().count() > 120
    {
        return Err(CommandError::new(
            "AUTH_SESSION_INVALID",
            "The signed-in identity is not available in this demo",
        ));
    }
    Ok(())
}

fn state_error() -> CommandError {
    CommandError::new(
        "STATE_UNAVAILABLE",
        "Session state is temporarily unavailable",
    )
    .retryable(None)
}
