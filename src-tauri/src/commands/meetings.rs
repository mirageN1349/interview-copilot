use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

use crate::{
    commands::{authorize_window, profiles::ProfileCommandState},
    error::CommandError,
    macos::capture::{CaptureConfiguration, CaptureRuntime, CaptureState},
    security::{
        policy::KillSwitch,
        run_gate::{self, GateContext, MeetingMode},
    },
    state::AppState,
    storage::{
        meetings::{self, CreateMeetingInput, Meeting},
        profiles,
    },
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunGateInput {
    launch_policy_id: String,
    requested_mode: MeetingMode,
    capture_configuration_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunGateResult {
    allowed: bool,
    reason_codes: Vec<&'static str>,
    policy_version: Option<String>,
    policy_expires_at: Option<String>,
    matrix_row_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingStartInput {
    launch_policy_id: String,
    profile_id: String,
    profile_revision: i64,
    capture_configuration_id: String,
    mode: MeetingMode,
    title: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingStopInput {
    meeting_id: String,
    reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingRuntimeSummary {
    id: String,
    launch_policy_id: String,
    profile_id: String,
    profile_revision: i64,
    title: String,
    status: String,
    mode: String,
    context_generation: i64,
    capture_phase: &'static str,
    display_id: u32,
    sound_threshold: f64,
    created_at_ms: i64,
    started_at_ms: Option<i64>,
    ended_at_ms: Option<i64>,
    failure_code: Option<String>,
}

impl MeetingRuntimeSummary {
    pub fn from_stopped(meeting: Meeting) -> Self {
        summary(meeting, "stopped")
    }
}

#[tauri::command]
pub fn run_gate_evaluate(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    input: RunGateInput,
) -> Result<RunGateResult, CommandError> {
    authorize_window(window.label(), &["main"])?;
    evaluate_gate(&app_state, &profile_state, &input)
}

#[tauri::command]
pub fn meeting_start(
    window: tauri::Window,
    app: tauri::AppHandle,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    input: MeetingStartInput,
) -> Result<MeetingRuntimeSummary, CommandError> {
    authorize_window(window.label(), &["main"])?;
    if let Some(active) = active_meeting_summary(&app_state, &profile_state)? {
        return Ok(active);
    }
    let gate = evaluate_gate(
        &app_state,
        &profile_state,
        &RunGateInput {
            launch_policy_id: input.launch_policy_id.clone(),
            requested_mode: input.mode,
            capture_configuration_id: Some(input.capture_configuration_id.clone()),
        },
    )?;
    let now = now_ms();
    if !gate.allowed {
        append_audit(
            &app_state,
            now,
            "meeting_start",
            "denied",
            gate.reason_codes
                .first()
                .copied()
                .unwrap_or("MEETING_UNAVAILABLE"),
        )?;
        return Err(CommandError::new(
            "MEETING_UNAVAILABLE",
            "The meeting is currently unavailable",
        ));
    }

    let owner = active_user(&app_state)?;
    let (display_id, sound_threshold) =
        parse_capture_configuration(&input.capture_configuration_id)?;
    let capture = CaptureConfiguration::new(display_id, true, true, 48_000, 2, true)
        .map_err(|message| CommandError::new("CAPTURE_CONFIGURATION_INVALID", message))?;
    let mut database = profile_state.database.lock().map_err(|_| state_error())?;
    let profile = profiles::get(&database, &owner, &input.profile_id).map_err(profile_error)?;
    if profile.revision != input.profile_revision || profile.status != "ready" {
        return Err(CommandError::new(
            "PROFILE_REVISION_CONFLICT",
            "Refresh the selected profile before starting",
        ));
    }
    let meeting_id = profiles::stable_id(
        "meeting",
        &[
            owner.as_bytes(),
            input.profile_id.as_bytes(),
            &now.to_be_bytes(),
        ],
    );
    let meeting = meetings::create(
        &mut database,
        &owner,
        CreateMeetingInput {
            id: meeting_id.clone(),
            launch_policy_id: input.launch_policy_id,
            profile_id: input.profile_id,
            title: input.title,
            mode: mode_name(input.mode).to_owned(),
            capture_configuration_id: input.capture_configuration_id,
            display_id: i64::from(display_id),
            sound_threshold,
            retention_expires_at_ms: now + 30 * 24 * 60 * 60_000,
        },
        now,
    )
    .map_err(meeting_error)?;
    let meeting = meetings::transition(&mut database, &owner, &meeting.id, "gating", now, None)
        .map_err(meeting_error)?;
    drop(database);

    append_audit(&app_state, now, "meeting_start", "succeeded", "OK")?;
    let mut database = profile_state.database.lock().map_err(|_| state_error())?;
    let meeting = meetings::transition(&mut database, &owner, &meeting.id, "running", now, None)
        .map_err(meeting_error)?;
    drop(database);
    {
        let mut runtime = app_state.0.lock().map_err(|_| state_error())?;
        runtime.active_meeting_id = Some(meeting.id.clone());
        runtime.capture_runtime = Some(CaptureRuntime::new(capture));
    }
    let overlay_window = app.get_webview_window("overlay").ok_or_else(|| {
        CommandError::new("OVERLAY_UNAVAILABLE", "The assistant panel is unavailable")
    });
    if overlay_window
        .and_then(|window| crate::macos::overlay::open_for_meeting(&window, &meeting.id))
        .is_err()
    {
        let mut runtime = app_state.0.lock().map_err(|_| state_error())?;
        if let Some(capture) = runtime.capture_runtime.as_mut() {
            capture.stop();
        }
        runtime.active_meeting_id = None;
        runtime.capture_runtime = None;
        drop(runtime);
        let mut database = profile_state.database.lock().map_err(|_| state_error())?;
        let _ = meetings::finalize(&mut database, &owner, &meeting.id, now);
        append_audit(
            &app_state,
            now,
            "meeting_start",
            "failed",
            "OVERLAY_UNAVAILABLE",
        )?;
        return Err(CommandError::new(
            "OVERLAY_UNAVAILABLE",
            "The assistant panel is unavailable",
        ));
    }
    let summary = summary(meeting, "listening");
    app.emit("meeting://state", &summary)
        .map_err(CommandError::from)?;
    Ok(summary)
}

fn active_meeting_summary(
    app_state: &AppState,
    profile_state: &ProfileCommandState,
) -> Result<Option<MeetingRuntimeSummary>, CommandError> {
    let runtime = app_state.0.lock().map_err(|_| state_error())?;
    let Some(meeting_id) = runtime.active_meeting_id.clone() else {
        return Ok(None);
    };
    let capture_phase = runtime
        .capture_runtime
        .as_ref()
        .map(|capture| capture_name(capture.state()))
        .unwrap_or("idle");
    let owner = runtime
        .active_user_id
        .clone()
        .ok_or_else(|| CommandError::new("AUTH_REQUIRED", "Sign in to manage meetings"))?;
    drop(runtime);
    let database = profile_state.database.lock().map_err(|_| state_error())?;
    let meeting = meetings::get(&database, &owner, &meeting_id).map_err(meeting_error)?;
    Ok(Some(summary(meeting, capture_phase)))
}

#[tauri::command]
pub fn meeting_get(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    meeting_id: String,
) -> Result<MeetingRuntimeSummary, CommandError> {
    authorize_window(window.label(), &["main", "overlay"])?;
    let owner = active_user(&app_state)?;
    let database = profile_state.database.lock().map_err(|_| state_error())?;
    let meeting = meetings::get(&database, &owner, &meeting_id).map_err(meeting_error)?;
    let capture_phase = app_state
        .0
        .lock()
        .map_err(|_| state_error())?
        .capture_runtime
        .as_ref()
        .map(|runtime| capture_name(runtime.state()))
        .unwrap_or("idle");
    Ok(summary(meeting, capture_phase))
}

#[tauri::command]
pub fn meeting_stop(
    window: tauri::Window,
    app: tauri::AppHandle,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    input: MeetingStopInput,
) -> Result<MeetingRuntimeSummary, CommandError> {
    authorize_window(window.label(), &["main", "overlay"])?;
    if !matches!(
        input.reason.as_str(),
        "user" | "kill_switch" | "policy_lost" | "error"
    ) {
        return Err(CommandError::new(
            "MEETING_STOP_REASON_INVALID",
            "The meeting stop reason is invalid",
        ));
    }
    let owner = active_user(&app_state)?;
    {
        let mut runtime = app_state.0.lock().map_err(|_| state_error())?;
        if runtime
            .active_meeting_id
            .as_deref()
            .is_some_and(|id| id != input.meeting_id)
        {
            return Err(CommandError::new(
                "MEETING_NOT_RUNNING",
                "This meeting is not active",
            ));
        }
        if let Some(capture) = runtime.capture_runtime.as_mut() {
            capture.stop();
        }
    }
    let now = now_ms();
    let mut database = profile_state.database.lock().map_err(|_| state_error())?;
    let meeting =
        meetings::finalize(&mut database, &owner, &input.meeting_id, now).map_err(meeting_error)?;
    crate::storage::history::index_owner(&mut database, &owner).map_err(|_| {
        CommandError::new(
            "HISTORY_INDEX_FAILED",
            "Meeting history could not be indexed",
        )
    })?;
    drop(database);
    append_audit(&app_state, now, "meeting_stop", "stopped", &input.reason)?;
    let mut runtime = app_state.0.lock().map_err(|_| state_error())?;
    runtime.active_meeting_id = None;
    runtime.capture_runtime = None;
    drop(runtime);
    let summary = summary(meeting, "stopped");
    app.emit("meeting://state", &summary)
        .map_err(CommandError::from)?;
    Ok(summary)
}

#[tauri::command]
pub fn meeting_context_reset(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    meeting_id: String,
) -> Result<i64, CommandError> {
    authorize_window(window.label(), &["main", "overlay"])?;
    let owner = active_user(&app_state)?;
    let mut database = profile_state.database.lock().map_err(|_| state_error())?;
    meetings::reset_context(&mut database, &owner, &meeting_id).map_err(meeting_error)
}

fn evaluate_gate(
    app_state: &AppState,
    profile_state: &ProfileCommandState,
    input: &RunGateInput,
) -> Result<RunGateResult, CommandError> {
    let now = now_ms();
    let runtime = app_state.0.lock().map_err(|_| state_error())?;
    let owner = runtime.active_user_id.clone();
    let policy = runtime.policy.clone();
    drop(runtime);
    let database = profile_state.database.lock().map_err(|_| state_error())?;
    let row = owner.as_deref().and_then(|owner_id| {
        database.connection().query_row(
            "SELECT u.status, lp.status, lp.starts_at_ms, lp.expires_at_ms, lp.environment_id, \
                    lp.approved_device_ids_json, lp.adversarial_approved, \
                    EXISTS(SELECT 1 FROM participant_consents pc WHERE pc.launch_policy_id = lp.id AND pc.revoked_at_ms IS NULL), \
                    EXISTS(SELECT 1 FROM ai_profiles p JOIN model_configurations mc ON mc.id = p.model_configuration_id \
                        WHERE p.owner_user_id = u.id AND mc.processing_boundary_id = 'mock-local-boundary') \
             FROM users u JOIN launch_policies lp ON lp.owner_user_id = u.id \
             WHERE u.id = ?1 AND lp.id = ?2",
            params![owner_id, input.launch_policy_id],
            |row| Ok((
                row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<i64>>(2)?,
                row.get::<_, Option<i64>>(3)?, row.get::<_, String>(4)?, row.get::<_, String>(5)?,
                row.get::<_, bool>(6)?, row.get::<_, bool>(7)?, row.get::<_, bool>(8)?,
            )),
        ).optional().ok().flatten()
    });
    let device_allowed = row
        .as_ref()
        .and_then(|value| serde_json::from_str::<Vec<String>>(&value.5).ok())
        .is_some_and(|ids| {
            policy
                .as_ref()
                .is_some_and(|snapshot| ids.contains(&snapshot.device_id))
        });
    let launch_valid = row.as_ref().is_some_and(|value| {
        value.1 == "active"
            && value.2.is_none_or(|starts| starts <= now)
            && value.3.is_some_and(|expires| expires > now)
    });
    let decision = run_gate::evaluate(&GateContext {
        session_bound: owner.is_some()
            && policy
                .as_ref()
                .is_some_and(|snapshot| Some(&snapshot.user_id) == owner.as_ref()),
        user_active: row.as_ref().is_some_and(|value| value.0 == "active"),
        device_allowed,
        environment_allowed: row.as_ref().is_some_and(|value| {
            policy
                .as_ref()
                .is_some_and(|snapshot| snapshot.environment_id == value.4)
        }),
        launch_policy_valid: launch_valid,
        policy_fresh: policy
            .as_ref()
            .is_some_and(|snapshot| snapshot.is_fresh(now)),
        kill_switch_clear: policy
            .as_ref()
            .is_some_and(|snapshot| snapshot.kill_switch == KillSwitch::Clear),
        consent_complete: row.as_ref().is_some_and(|value| value.7),
        processing_boundary_approved: row.as_ref().is_some_and(|value| value.8),
        mode: input.requested_mode,
        adversarial_role: false,
        adversarial_approved: row.as_ref().is_some_and(|value| value.6)
            && policy
                .as_ref()
                .is_some_and(|snapshot| snapshot.allow_adversarial),
        matrix_row_approved: false,
    });
    let policy_version = policy
        .as_ref()
        .map(|snapshot| snapshot.policy_version.clone());
    let policy_expires_at = policy.as_ref().map(|snapshot| {
        time::OffsetDateTime::from_unix_timestamp_nanos(
            i128::from(snapshot.expires_at_ms) * 1_000_000,
        )
        .ok()
        .and_then(|value| {
            value
                .format(&time::format_description::well_known::Rfc3339)
                .ok()
        })
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_owned())
    });
    let mut reason_codes = decision.reason_codes;
    if input
        .capture_configuration_id
        .as_deref()
        .is_some_and(|value| parse_capture_configuration(value).is_err())
    {
        reason_codes.push("CAPTURE_CONFIGURATION_INVALID");
    }
    Ok(RunGateResult {
        allowed: reason_codes.is_empty(),
        reason_codes,
        policy_version,
        policy_expires_at,
        matrix_row_id: None,
    })
}

fn parse_capture_configuration(value: &str) -> Result<(u32, f64), CommandError> {
    let rest = value.strip_prefix("display-").ok_or_else(|| {
        CommandError::new("CAPTURE_CONFIGURATION_INVALID", "Select a display again")
    })?;
    let (display, threshold) = rest.split_once("-vad-").ok_or_else(|| {
        CommandError::new("CAPTURE_CONFIGURATION_INVALID", "Select a display again")
    })?;
    let display_id = display
        .parse::<u32>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| {
            CommandError::new("CAPTURE_CONFIGURATION_INVALID", "Select a display again")
        })?;
    let threshold = threshold
        .parse::<f64>()
        .ok()
        .filter(|value| (0.0..=1.0).contains(value))
        .ok_or_else(|| {
            CommandError::new(
                "CAPTURE_CONFIGURATION_INVALID",
                "Choose a valid sound threshold",
            )
        })?;
    Ok((display_id, threshold))
}

fn append_audit(
    state: &AppState,
    at: i64,
    action: &str,
    outcome: &str,
    reason: &str,
) -> Result<(), CommandError> {
    let mut runtime = state.0.lock().map_err(|_| state_error())?;
    runtime.audit.append(at, action, outcome, reason);
    runtime.audit.verify().then_some(()).ok_or_else(|| {
        CommandError::new("AUDIT_INTEGRITY_FAILED", "The meeting action was blocked")
    })
}

fn active_user(state: &AppState) -> Result<String, CommandError> {
    state
        .0
        .lock()
        .map_err(|_| state_error())?
        .active_user_id
        .clone()
        .ok_or_else(|| CommandError::new("AUTH_REQUIRED", "Sign in to manage meetings"))
}

fn summary(meeting: Meeting, capture_phase: &'static str) -> MeetingRuntimeSummary {
    let (display_id, sound_threshold) =
        parse_capture_configuration(&meeting.capture_configuration_id).unwrap_or((0, 0.0));
    MeetingRuntimeSummary {
        id: meeting.id,
        launch_policy_id: meeting.launch_policy_id,
        profile_id: meeting.profile_id,
        profile_revision: meeting.profile_revision,
        title: meeting.title,
        status: meeting.status,
        mode: meeting.mode,
        context_generation: meeting.context_generation,
        capture_phase,
        display_id,
        sound_threshold,
        created_at_ms: meeting.created_at_ms,
        started_at_ms: meeting.started_at_ms,
        ended_at_ms: meeting.ended_at_ms,
        failure_code: meeting.failure_code,
    }
}

fn mode_name(mode: MeetingMode) -> &'static str {
    match mode {
        MeetingMode::StandardLab => "standard_lab",
        MeetingMode::AdversarialLab => "adversarial_lab",
    }
}
fn capture_name(state: CaptureState) -> &'static str {
    match state {
        CaptureState::Listening => "listening",
        CaptureState::Recording => "recording",
        CaptureState::PausedSourceLost => "paused",
        CaptureState::Stopped => "stopped",
    }
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
        "Meeting state is temporarily unavailable",
    )
    .retryable(None)
}
fn meeting_error(error: meetings::MeetingStoreError) -> CommandError {
    match error {
        meetings::MeetingStoreError::NotFound => {
            CommandError::new("MEETING_NOT_FOUND", "Meeting not found")
        }
        meetings::MeetingStoreError::Invalid(message) => {
            CommandError::new("MEETING_INVALID", message)
        }
        meetings::MeetingStoreError::InvalidTransition { .. } => {
            CommandError::new("MEETING_STATE_CONFLICT", "Meeting state changed")
        }
        meetings::MeetingStoreError::Database(_) => {
            CommandError::new("MEETING_STORAGE_FAILED", "The meeting could not be stored")
                .retryable(None)
        }
    }
}
fn profile_error(_: profiles::ProfileStoreError) -> CommandError {
    CommandError::new("PROFILE_NOT_READY", "The selected profile is unavailable")
}
