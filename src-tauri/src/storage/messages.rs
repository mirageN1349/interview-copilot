use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::storage::Database;

#[derive(Debug)]
pub enum MessageStoreError {
    Database(rusqlite::Error),
    Invalid(&'static str),
    NotFound,
    Conflict,
    FinalTranscriptImmutable,
}

impl From<rusqlite::Error> for MessageStoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatThread {
    pub id: String,
    pub meeting_id: String,
    pub kind: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendMessageInput {
    pub id: String,
    pub role: String,
    pub content: String,
    pub status: String,
    pub reply_to_segment_id: Option<String>,
    pub artifact_ids: Vec<String>,
    pub profile_source_ids: Vec<String>,
    pub context_generation: i64,
    pub confidence: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub thread_id: String,
    pub sequence: i64,
    pub role: String,
    pub content: String,
    pub status: String,
    pub reply_to_segment_id: Option<String>,
    pub artifact_ids: Vec<String>,
    pub profile_source_ids: Vec<String>,
    pub context_generation: i64,
    pub confidence: Option<f64>,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptInput {
    pub id: String,
    pub sequence: i64,
    pub speaker: String,
    pub text: String,
    pub confidence: f64,
    pub is_final: bool,
    pub is_question: bool,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
}

pub fn threads(
    database: &Database,
    owner_user_id: &str,
    meeting_id: &str,
) -> Result<Vec<ChatThread>, MessageStoreError> {
    authorize(database, owner_user_id, meeting_id, false)?;
    let mut statement = database.connection().prepare(
        "SELECT id, meeting_id, kind, created_at_ms FROM chat_threads \
         WHERE meeting_id = ?1 ORDER BY CASE kind WHEN 'live' THEN 0 ELSE 1 END",
    )?;
    Ok(statement
        .query_map([meeting_id], |row| {
            Ok(ChatThread {
                id: row.get(0)?,
                meeting_id: row.get(1)?,
                kind: row.get(2)?,
                created_at_ms: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn append(
    database: &mut Database,
    owner_user_id: &str,
    meeting_id: &str,
    thread_kind: &str,
    input: AppendMessageInput,
    now_ms: i64,
) -> Result<ChatMessage, MessageStoreError> {
    validate_message(&input, thread_kind)?;
    let context_generation = authorize(database, owner_user_id, meeting_id, true)?;
    if input.context_generation != context_generation {
        return Err(MessageStoreError::Conflict);
    }
    let thread_id = thread_id(database, meeting_id, thread_kind)?;
    if let Some(existing) = message_by_id(database, &thread_id, &input.id)? {
        return message_matches(&existing, &input)
            .then_some(existing)
            .ok_or(MessageStoreError::Conflict);
    }
    validate_attachments(
        database,
        meeting_id,
        &input.artifact_ids,
        &input.profile_source_ids,
    )?;
    let sequence = database.connection().query_row(
        "SELECT COALESCE(MAX(sequence) + 1, 0) FROM chat_messages WHERE thread_id = ?1",
        [&thread_id],
        |row| row.get::<_, i64>(0),
    )?;
    database.connection().execute(
        "INSERT INTO chat_messages(id, thread_id, sequence, role, content, status, \
            reply_to_segment_id, artifact_ids_json, profile_source_ids_json, \
            context_generation, confidence, created_at_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            input.id,
            thread_id,
            sequence,
            input.role,
            input.content,
            input.status,
            input.reply_to_segment_id,
            serde_json::to_string(&input.artifact_ids).expect("serializable IDs"),
            serde_json::to_string(&input.profile_source_ids).expect("serializable IDs"),
            input.context_generation,
            input.confidence,
            now_ms
        ],
    )?;
    message_by_id(database, &thread_id, &input.id)?.ok_or(MessageStoreError::NotFound)
}

pub fn list(
    database: &Database,
    owner_user_id: &str,
    meeting_id: &str,
    thread_kind: &str,
) -> Result<Vec<ChatMessage>, MessageStoreError> {
    authorize(database, owner_user_id, meeting_id, false)?;
    let thread_id = thread_id(database, meeting_id, thread_kind)?;
    let mut statement = database.connection().prepare(
        "SELECT id, thread_id, sequence, role, content, status, reply_to_segment_id, \
                artifact_ids_json, profile_source_ids_json, context_generation, confidence, created_at_ms \
         FROM chat_messages WHERE thread_id = ?1 ORDER BY sequence",
    )?;
    Ok(statement
        .query_map([thread_id], message_from_row)?
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn save_transcript(
    database: &mut Database,
    owner_user_id: &str,
    meeting_id: &str,
    input: TranscriptInput,
) -> Result<TranscriptInput, MessageStoreError> {
    validate_transcript(&input)?;
    authorize(database, owner_user_id, meeting_id, true)?;
    let existing = transcript_at(database, meeting_id, input.sequence)?;
    if let Some(existing) = &existing {
        if existing.is_final {
            return (existing == &input)
                .then_some(existing.clone())
                .ok_or(MessageStoreError::FinalTranscriptImmutable);
        }
        if existing.id != input.id {
            return Err(MessageStoreError::Conflict);
        }
    } else {
        let next = database.connection().query_row(
            "SELECT COALESCE(MAX(sequence) + 1, 0) FROM transcript_segments WHERE meeting_id = ?1",
            [meeting_id],
            |row| row.get::<_, i64>(0),
        )?;
        if input.sequence != next {
            return Err(MessageStoreError::Conflict);
        }
    }
    database.connection().execute(
        "INSERT INTO transcript_segments(id, meeting_id, sequence, speaker, text, confidence, \
            is_final, is_question, started_at_ms, ended_at_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
         ON CONFLICT(meeting_id, sequence) DO UPDATE SET \
            speaker = excluded.speaker, text = excluded.text, confidence = excluded.confidence, \
            is_final = excluded.is_final, is_question = excluded.is_question, \
            started_at_ms = excluded.started_at_ms, ended_at_ms = excluded.ended_at_ms \
         WHERE transcript_segments.is_final = 0",
        params![
            input.id,
            meeting_id,
            input.sequence,
            input.speaker,
            input.text,
            input.confidence,
            input.is_final,
            input.is_question,
            input.started_at_ms,
            input.ended_at_ms
        ],
    )?;
    transcript_at(database, meeting_id, input.sequence)?.ok_or(MessageStoreError::NotFound)
}

pub fn transcript(
    database: &Database,
    owner_user_id: &str,
    meeting_id: &str,
) -> Result<Vec<TranscriptInput>, MessageStoreError> {
    authorize(database, owner_user_id, meeting_id, false)?;
    let mut statement = database.connection().prepare(
        "SELECT id, sequence, speaker, text, confidence, is_final, is_question, \
                started_at_ms, ended_at_ms FROM transcript_segments \
         WHERE meeting_id = ?1 ORDER BY sequence",
    )?;
    Ok(statement
        .query_map([meeting_id], transcript_from_row)?
        .collect::<Result<Vec<_>, _>>()?)
}

fn authorize(
    database: &Database,
    owner_user_id: &str,
    meeting_id: &str,
    require_running: bool,
) -> Result<i64, MessageStoreError> {
    database
        .connection()
        .query_row(
            "SELECT m.context_generation, m.status FROM meetings m \
             JOIN launch_policies lp ON lp.id = m.launch_policy_id \
             JOIN ai_profiles p ON p.id = m.profile_id \
             WHERE m.id = ?1 AND lp.owner_user_id = ?2 AND p.owner_user_id = ?2",
            params![meeting_id, owner_user_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or(MessageStoreError::NotFound)
        .and_then(|(generation, status)| {
            (!require_running || status == "running")
                .then_some(generation)
                .ok_or(MessageStoreError::Invalid("meeting is not running"))
        })
}

fn thread_id(
    database: &Database,
    meeting_id: &str,
    kind: &str,
) -> Result<String, MessageStoreError> {
    if !matches!(kind, "live" | "side") {
        return Err(MessageStoreError::Invalid("invalid thread kind"));
    }
    database
        .connection()
        .query_row(
            "SELECT id FROM chat_threads WHERE meeting_id = ?1 AND kind = ?2",
            params![meeting_id, kind],
            |row| row.get(0),
        )
        .optional()?
        .ok_or(MessageStoreError::NotFound)
}

fn validate_message(
    input: &AppendMessageInput,
    thread_kind: &str,
) -> Result<(), MessageStoreError> {
    if input.id.is_empty()
        || !matches!(thread_kind, "live" | "side")
        || !matches!(input.role.as_str(), "user" | "assistant" | "system")
        || !matches!(
            input.status.as_str(),
            "pending" | "streaming" | "complete" | "error" | "cancelled"
        )
        || input.context_generation < 0
        || input
            .confidence
            .is_some_and(|value| !(0.0..=1.0).contains(&value))
    {
        return Err(MessageStoreError::Invalid("invalid message"));
    }
    Ok(())
}

fn validate_transcript(input: &TranscriptInput) -> Result<(), MessageStoreError> {
    if input.id.is_empty()
        || input.sequence < 0
        || !matches!(input.speaker.as_str(), "interviewer" | "user" | "unknown")
        || !(0.0..=1.0).contains(&input.confidence)
        || input.ended_at_ms < input.started_at_ms
        || (!input.is_final && input.is_question)
    {
        return Err(MessageStoreError::Invalid("invalid transcript segment"));
    }
    Ok(())
}

fn validate_attachments(
    database: &Database,
    meeting_id: &str,
    artifact_ids: &[String],
    profile_source_ids: &[String],
) -> Result<(), MessageStoreError> {
    for id in artifact_ids {
        let allowed: bool = database.connection().query_row(
            "SELECT EXISTS(SELECT 1 FROM artifacts WHERE id = ?1 AND meeting_id = ?2 \
             AND content_status IN ('allowed', 'redacted'))",
            params![id, meeting_id],
            |row| row.get(0),
        )?;
        if !allowed {
            return Err(MessageStoreError::Invalid("artifact is not allowed"));
        }
    }
    for id in profile_source_ids {
        let allowed: bool = database.connection().query_row(
            "SELECT EXISTS(SELECT 1 FROM profile_sources ps JOIN meetings m ON m.profile_id = ps.profile_id \
             WHERE ps.id = ?1 AND m.id = ?2 AND ps.content_status IN ('allowed', 'redacted'))",
            params![id, meeting_id],
            |row| row.get(0),
        )?;
        if !allowed {
            return Err(MessageStoreError::Invalid("profile source is not allowed"));
        }
    }
    Ok(())
}

fn message_by_id(
    database: &Database,
    thread_id: &str,
    message_id: &str,
) -> Result<Option<ChatMessage>, MessageStoreError> {
    Ok(database
        .connection()
        .query_row(
            "SELECT id, thread_id, sequence, role, content, status, reply_to_segment_id, \
                    artifact_ids_json, profile_source_ids_json, context_generation, confidence, created_at_ms \
             FROM chat_messages WHERE id = ?1 AND thread_id = ?2",
            params![message_id, thread_id],
            message_from_row,
        )
        .optional()?)
}

fn message_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChatMessage> {
    let artifacts: String = row.get(7)?;
    let sources: String = row.get(8)?;
    Ok(ChatMessage {
        id: row.get(0)?,
        thread_id: row.get(1)?,
        sequence: row.get(2)?,
        role: row.get(3)?,
        content: row.get(4)?,
        status: row.get(5)?,
        reply_to_segment_id: row.get(6)?,
        artifact_ids: parse_ids(artifacts, 7)?,
        profile_source_ids: parse_ids(sources, 8)?,
        context_generation: row.get(9)?,
        confidence: row.get(10)?,
        created_at_ms: row.get(11)?,
    })
}

fn message_matches(message: &ChatMessage, input: &AppendMessageInput) -> bool {
    message.role == input.role
        && message.content == input.content
        && message.status == input.status
        && message.reply_to_segment_id == input.reply_to_segment_id
        && message.artifact_ids == input.artifact_ids
        && message.profile_source_ids == input.profile_source_ids
        && message.context_generation == input.context_generation
        && message.confidence == input.confidence
}

fn transcript_at(
    database: &Database,
    meeting_id: &str,
    sequence: i64,
) -> Result<Option<TranscriptInput>, MessageStoreError> {
    Ok(database
        .connection()
        .query_row(
            "SELECT id, sequence, speaker, text, confidence, is_final, is_question, \
                    started_at_ms, ended_at_ms FROM transcript_segments \
             WHERE meeting_id = ?1 AND sequence = ?2",
            params![meeting_id, sequence],
            transcript_from_row,
        )
        .optional()?)
}

fn transcript_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TranscriptInput> {
    Ok(TranscriptInput {
        id: row.get(0)?,
        sequence: row.get(1)?,
        speaker: row.get(2)?,
        text: row.get(3)?,
        confidence: row.get(4)?,
        is_final: row.get(5)?,
        is_question: row.get(6)?,
        started_at_ms: row.get(7)?,
        ended_at_ms: row.get(8)?,
    })
}

fn parse_ids(json: String, column: usize) -> rusqlite::Result<Vec<String>> {
    serde_json::from_str(&json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}
