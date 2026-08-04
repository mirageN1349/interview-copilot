use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::storage::{Database, profiles::stable_id};

#[derive(Debug)]
pub enum MeetingStoreError {
    Database(rusqlite::Error),
    Invalid(&'static str),
    NotFound,
    InvalidTransition { from: String, to: String },
}

impl From<rusqlite::Error> for MeetingStoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMeetingInput {
    pub id: String,
    pub launch_policy_id: String,
    pub profile_id: String,
    pub title: String,
    pub mode: String,
    pub capture_configuration_id: String,
    pub display_id: i64,
    pub sound_threshold: f64,
    pub retention_expires_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meeting {
    pub id: String,
    pub launch_policy_id: String,
    pub profile_id: String,
    pub profile_revision: i64,
    pub model_snapshot: serde_json::Value,
    pub title: String,
    pub status: String,
    pub mode: String,
    pub capture_configuration_id: String,
    pub context_generation: i64,
    pub created_at_ms: i64,
    pub started_at_ms: Option<i64>,
    pub ended_at_ms: Option<i64>,
    pub retention_expires_at_ms: i64,
    pub failure_code: Option<String>,
}

pub fn create(
    database: &mut Database,
    owner_user_id: &str,
    input: CreateMeetingInput,
    now_ms: i64,
) -> Result<Meeting, MeetingStoreError> {
    validate_create(&input, now_ms)?;
    let (profile_revision, model_snapshot) = load_profile_snapshot(
        database,
        owner_user_id,
        &input.launch_policy_id,
        &input.profile_id,
    )?;
    let model_snapshot_json = serde_json::to_string(&model_snapshot)
        .map_err(|_| MeetingStoreError::Invalid("invalid model snapshot"))?;
    let capture_configuration_id = format!("{}:{}", input.capture_configuration_id, input.id);

    database
        .transaction(|transaction| {
            transaction.execute(
                "INSERT INTO meetings(id, launch_policy_id, profile_id, profile_revision, \
                    model_snapshot_json, title, status, mode, capture_configuration_id, \
                    created_at_ms, retention_expires_at_ms) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'prepared', ?7, ?8, ?9, ?10)",
                params![
                    input.id,
                    input.launch_policy_id,
                    input.profile_id,
                    profile_revision,
                    model_snapshot_json,
                    input.title.trim(),
                    input.mode,
                    input.capture_configuration_id,
                    now_ms,
                    input.retention_expires_at_ms
                ],
            )?;
            for kind in ["live", "side"] {
                transaction.execute(
                    "INSERT INTO chat_threads(id, meeting_id, kind, created_at_ms) VALUES (?1, ?2, ?3, ?4)",
                    params![stable_id("thread", &[input.id.as_bytes(), kind.as_bytes()]), input.id, kind, now_ms],
                )?;
            }
            transaction.execute(
                "INSERT INTO capture_configurations(id, meeting_id, display_id, backing_scale, \
                    capture_system_audio, capture_microphone, shows_cursor_in_own_artifacts, \
                    auto_screenshot_mode, sound_threshold) \
                 VALUES (?1, ?2, ?3, 1.0, 1, 1, 0, 'off', ?4)",
                params![capture_configuration_id, input.id, input.display_id, input.sound_threshold],
            )?;
            Ok(())
        })
        .map_err(MeetingStoreError::Database)?;
    get(database, owner_user_id, &input.id)
}

pub fn get(
    database: &Database,
    owner_user_id: &str,
    meeting_id: &str,
) -> Result<Meeting, MeetingStoreError> {
    database
        .connection()
        .query_row(
            "SELECT m.id, m.launch_policy_id, m.profile_id, m.profile_revision, \
                    m.model_snapshot_json, m.title, m.status, m.mode, \
                    m.capture_configuration_id, m.context_generation, m.created_at_ms, \
                    m.started_at_ms, m.ended_at_ms, m.retention_expires_at_ms, m.failure_code \
             FROM meetings m \
             JOIN launch_policies lp ON lp.id = m.launch_policy_id \
             JOIN ai_profiles p ON p.id = m.profile_id \
             WHERE m.id = ?1 AND lp.owner_user_id = ?2 AND p.owner_user_id = ?2",
            params![meeting_id, owner_user_id],
            meeting_from_row,
        )
        .optional()?
        .ok_or(MeetingStoreError::NotFound)
}

pub fn transition(
    database: &mut Database,
    owner_user_id: &str,
    meeting_id: &str,
    next_status: &str,
    now_ms: i64,
    failure_code: Option<&str>,
) -> Result<Meeting, MeetingStoreError> {
    let current = get(database, owner_user_id, meeting_id)?;
    if !valid_transition(&current.status, next_status) {
        return Err(MeetingStoreError::InvalidTransition {
            from: current.status,
            to: next_status.to_owned(),
        });
    }
    if next_status == "failed" && failure_code.is_none() {
        return Err(MeetingStoreError::Invalid("failure code is required"));
    }
    database
        .connection()
        .execute(
            "UPDATE meetings SET status = ?1, \
                started_at_ms = CASE WHEN ?1 = 'running' THEN COALESCE(started_at_ms, ?2) ELSE started_at_ms END, \
                ended_at_ms = CASE WHEN ?1 IN ('completed', 'failed', 'expired') THEN COALESCE(ended_at_ms, ?2) ELSE ended_at_ms END, \
                failure_code = CASE WHEN ?1 = 'failed' THEN ?3 ELSE failure_code END \
             WHERE id = ?4",
            params![next_status, now_ms, failure_code, meeting_id],
        )?;
    get(database, owner_user_id, meeting_id)
}

pub fn finalize(
    database: &mut Database,
    owner_user_id: &str,
    meeting_id: &str,
    ended_at_ms: i64,
) -> Result<Meeting, MeetingStoreError> {
    let current = get(database, owner_user_id, meeting_id)?;
    if matches!(current.status.as_str(), "completed" | "failed" | "expired") {
        return Ok(current);
    }
    if !matches!(current.status.as_str(), "running" | "stopping") {
        return Err(MeetingStoreError::InvalidTransition {
            from: current.status,
            to: "completed".into(),
        });
    }
    database
        .transaction(|transaction| {
            transaction.execute(
                "UPDATE meetings SET status = 'stopping' WHERE id = ?1 AND status = 'running'",
                [meeting_id],
            )?;
            transaction.execute(
                "UPDATE meetings SET status = 'completed', ended_at_ms = COALESCE(ended_at_ms, ?1) \
                 WHERE id = ?2 AND status = 'stopping'",
                params![ended_at_ms, meeting_id],
            )?;
            Ok(())
        })
        .map_err(MeetingStoreError::Database)?;
    get(database, owner_user_id, meeting_id)
}

pub fn reset_context(
    database: &mut Database,
    owner_user_id: &str,
    meeting_id: &str,
) -> Result<i64, MeetingStoreError> {
    let current = get(database, owner_user_id, meeting_id)?;
    if current.status != "running" {
        return Err(MeetingStoreError::Invalid("meeting is not running"));
    }
    database.connection().execute(
        "UPDATE meetings SET context_generation = context_generation + 1 WHERE id = ?1",
        [meeting_id],
    )?;
    Ok(current.context_generation + 1)
}

fn validate_create(input: &CreateMeetingInput, now_ms: i64) -> Result<(), MeetingStoreError> {
    if input.id.is_empty()
        || input.launch_policy_id.is_empty()
        || input.profile_id.is_empty()
        || input.capture_configuration_id.is_empty()
        || input.title.trim().is_empty()
        || input.display_id <= 0
        || !(0.0..=1.0).contains(&input.sound_threshold)
        || input.title.chars().count() > 160
        || !matches!(input.mode.as_str(), "standard_lab" | "adversarial_lab")
        || input.retention_expires_at_ms <= now_ms
    {
        return Err(MeetingStoreError::Invalid("invalid meeting"));
    }
    Ok(())
}

fn load_profile_snapshot(
    database: &Database,
    owner_user_id: &str,
    launch_policy_id: &str,
    profile_id: &str,
) -> Result<(i64, serde_json::Value), MeetingStoreError> {
    database
        .connection()
        .query_row(
            "SELECT p.revision, mc.response_model_id, mc.transcription_model_id, \
                    mc.translation_language, mc.answer_depth, mc.question_confidence_threshold, \
                    mc.processing_boundary_id \
             FROM ai_profiles p \
             JOIN model_configurations mc ON mc.id = p.model_configuration_id \
             JOIN launch_policies lp ON lp.id = ?1 \
             WHERE p.id = ?2 AND p.owner_user_id = ?3 AND lp.owner_user_id = ?3 \
                   AND p.status = 'ready' AND lp.status = 'active'",
            params![launch_policy_id, profile_id, owner_user_id],
            |row| {
                Ok((
                    row.get(0)?,
                    serde_json::json!({
                        "responseModelId": row.get::<_, String>(1)?,
                        "transcriptionModelId": row.get::<_, String>(2)?,
                        "translationLanguage": row.get::<_, String>(3)?,
                        "answerDepth": row.get::<_, String>(4)?,
                        "questionConfidenceThreshold": row.get::<_, f64>(5)?,
                        "processingBoundaryId": row.get::<_, String>(6)?,
                    }),
                ))
            },
        )
        .optional()?
        .ok_or(MeetingStoreError::NotFound)
}

fn meeting_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Meeting> {
    let model_snapshot: String = row.get(4)?;
    Ok(Meeting {
        id: row.get(0)?,
        launch_policy_id: row.get(1)?,
        profile_id: row.get(2)?,
        profile_revision: row.get(3)?,
        model_snapshot: serde_json::from_str(&model_snapshot).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        title: row.get(5)?,
        status: row.get(6)?,
        mode: row.get(7)?,
        capture_configuration_id: row.get(8)?,
        context_generation: row.get(9)?,
        created_at_ms: row.get(10)?,
        started_at_ms: row.get(11)?,
        ended_at_ms: row.get(12)?,
        retention_expires_at_ms: row.get(13)?,
        failure_code: row.get(14)?,
    })
}

fn valid_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("prepared", "gating")
            | ("gating", "running")
            | ("gating", "failed")
            | ("running", "stopping")
            | ("stopping", "completed")
            | ("stopping", "failed")
            | ("completed", "expired")
    )
}
