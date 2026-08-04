use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::storage::{Cursor, Database};

const MAX_PAGE_SIZE: usize = 100;
const MAX_EXPORT_BYTES: usize = 2 * 1024 * 1024;
const AUDIT_RETENTION_MS: i64 = 365 * 24 * 60 * 60_000;

#[derive(Debug)]
pub enum HistoryError {
    Database(rusqlite::Error),
    Invalid(&'static str),
    NotFound,
    Forbidden,
    AuditIntegrity,
    ExportTooLarge,
}

impl From<rusqlite::Error> for HistoryError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SearchField {
    #[default]
    Any,
    Title,
    Vacancy,
    Transcript,
    Chat,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorySearch {
    pub query: Option<String>,
    #[serde(default)]
    pub field: SearchField,
    pub profile_query: Option<String>,
    pub from_ms: Option<i64>,
    pub to_ms: Option<i64>,
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

impl Default for HistorySearch {
    fn default() -> Self {
        Self {
            query: None,
            field: SearchField::Any,
            profile_query: None,
            from_ms: None,
            to_ms: None,
            cursor: None,
            limit: default_limit(),
        }
    }
}

fn default_limit() -> usize {
    30
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryListItem {
    pub id: String,
    pub title: String,
    pub status: String,
    pub mode: String,
    pub profile_id: String,
    pub profile_name: String,
    pub vacancy_role: String,
    pub created_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub retention_expires_at_ms: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub items: Vec<HistoryListItem>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryArtifact {
    pub id: String,
    pub kind: String,
    pub mime_type: String,
    pub byte_length: i64,
    pub content_status: String,
    pub created_at_ms: i64,
    pub expires_at_ms: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryTranscriptSegment {
    pub id: String,
    pub sequence: i64,
    pub speaker: String,
    pub text: String,
    pub confidence: f64,
    pub is_question: bool,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryChatMessage {
    pub id: String,
    pub sequence: i64,
    pub role: String,
    pub content: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryChat {
    pub kind: String,
    pub messages: Vec<HistoryChatMessage>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingHistoryDetail {
    pub id: String,
    pub title: String,
    pub status: String,
    pub mode: String,
    pub profile_id: String,
    pub profile_name: String,
    pub vacancy_role: String,
    pub created_at_ms: i64,
    pub started_at_ms: Option<i64>,
    pub ended_at_ms: Option<i64>,
    pub retention_expires_at_ms: i64,
    pub artifacts: Vec<HistoryArtifact>,
    pub transcript: Vec<HistoryTranscriptSegment>,
    pub chats: Vec<HistoryChat>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryExport {
    pub exported_at_ms: i64,
    pub meeting: MeetingHistoryDetail,
}

pub struct AuditWrite<'a> {
    pub occurred_at_ms: i64,
    pub user_id: Option<&'a str>,
    pub launch_policy_id: Option<&'a str>,
    pub meeting_id: Option<&'a str>,
    pub action: &'a str,
    pub outcome: &'a str,
    pub reason_code: &'a str,
}

pub fn index_owner(database: &mut Database, owner_user_id: &str) -> Result<(), HistoryError> {
    database.transaction(|transaction| {
        transaction.execute("DELETE FROM meeting_search WHERE owner_user_id = ?1", [owner_user_id])?;
        transaction.execute(
            "INSERT INTO meeting_search(meeting_id, owner_user_id, created_at_ms, title, vacancy, transcript, chat)
             SELECT m.id, lp.owner_user_id, m.created_at_ms, m.title,
                    trim(COALESCE(v.role_title, '') || ' ' || COALESCE(v.company_context, '') || ' ' || COALESCE(v.responsibilities_json, '') || ' ' || COALESCE(v.requirements_json, '')),
                    COALESCE((SELECT group_concat(ts.text, ' ') FROM transcript_segments ts WHERE ts.meeting_id = m.id AND ts.is_final = 1), ''),
                    COALESCE((SELECT group_concat(cm.content, ' ') FROM chat_messages cm JOIN chat_threads ct ON ct.id = cm.thread_id WHERE ct.meeting_id = m.id AND cm.status = 'complete'), '')
             FROM meetings m
             JOIN launch_policies lp ON lp.id = m.launch_policy_id
             JOIN ai_profiles p ON p.id = m.profile_id AND p.owner_user_id = lp.owner_user_id
             LEFT JOIN vacancy_sources v ON v.profile_id = m.profile_id AND v.review_status = 'confirmed'
             WHERE lp.owner_user_id = ?1 AND m.status != 'expired'",
            [owner_user_id],
        )?;
        Ok(())
    })?;
    Ok(())
}

pub fn search(
    database: &Database,
    owner_user_id: &str,
    input: &HistorySearch,
    now_ms: i64,
) -> Result<HistoryPage, HistoryError> {
    let prepared = PreparedSearch::new(input)?;
    let cursor_at = prepared.cursor.as_ref().map(|cursor| cursor.created_at_ms);
    let cursor_id = prepared.cursor.as_ref().map(|cursor| cursor.id.as_str());
    let items = if let Some(fts_query) = &prepared.fts_query {
        let mut statement = database.connection().prepare(SEARCH_WITH_MATCH_SQL)?;
        statement
            .query_map(
                params![
                    owner_user_id,
                    now_ms,
                    fts_query,
                    prepared.profile_query,
                    prepared.from_ms,
                    prepared.to_ms,
                    cursor_at,
                    cursor_id,
                    (prepared.limit + 1) as i64,
                ],
                history_item_from_row,
            )?
            .collect::<Result<Vec<_>, _>>()?
    } else {
        let mut statement = database.connection().prepare(SEARCH_WITHOUT_MATCH_SQL)?;
        statement
            .query_map(
                params![
                    owner_user_id,
                    now_ms,
                    prepared.profile_query,
                    prepared.from_ms,
                    prepared.to_ms,
                    cursor_at,
                    cursor_id,
                    (prepared.limit + 1) as i64,
                ],
                history_item_from_row,
            )?
            .collect::<Result<Vec<_>, _>>()?
    };
    let has_more = items.len() > prepared.limit;
    let mut items = items;
    items.truncate(prepared.limit);
    let next_cursor = has_more.then(|| {
        let last = items.last().expect("a page with more rows is not empty");
        Cursor {
            created_at_ms: last.created_at_ms,
            id: last.id.clone(),
        }
        .encode()
    });
    Ok(HistoryPage { items, next_cursor })
}

pub fn search_query_plan(
    database: &Database,
    input: &HistorySearch,
    owner_user_id: &str,
    now_ms: i64,
) -> Result<Vec<String>, HistoryError> {
    let prepared = PreparedSearch::new(input)?;
    let cursor_at = prepared.cursor.as_ref().map(|cursor| cursor.created_at_ms);
    let cursor_id = prepared.cursor.as_ref().map(|cursor| cursor.id.as_str());
    let plan = if let Some(fts_query) = &prepared.fts_query {
        let mut statement = database
            .connection()
            .prepare(&format!("EXPLAIN QUERY PLAN {SEARCH_WITH_MATCH_SQL}"))?;
        statement
            .query_map(
                params![
                    owner_user_id,
                    now_ms,
                    fts_query,
                    prepared.profile_query,
                    prepared.from_ms,
                    prepared.to_ms,
                    cursor_at,
                    cursor_id,
                    (prepared.limit + 1) as i64,
                ],
                |row| row.get(3),
            )?
            .collect::<Result<Vec<_>, _>>()?
    } else {
        let mut statement = database
            .connection()
            .prepare(&format!("EXPLAIN QUERY PLAN {SEARCH_WITHOUT_MATCH_SQL}"))?;
        statement
            .query_map(
                params![
                    owner_user_id,
                    now_ms,
                    prepared.profile_query,
                    prepared.from_ms,
                    prepared.to_ms,
                    cursor_at,
                    cursor_id,
                    (prepared.limit + 1) as i64,
                ],
                |row| row.get(3),
            )?
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok(plan)
}

pub fn detail(
    database: &Database,
    owner_user_id: &str,
    meeting_id: &str,
    now_ms: i64,
) -> Result<MeetingHistoryDetail, HistoryError> {
    let base = database
        .connection()
        .query_row(
            "SELECT m.id, m.title, m.status, m.mode, m.profile_id, p.name,
                    COALESCE(v.role_title, ''), m.created_at_ms, m.started_at_ms, m.ended_at_ms,
                    m.retention_expires_at_ms
             FROM meetings m
             JOIN launch_policies lp ON lp.id = m.launch_policy_id
             JOIN ai_profiles p ON p.id = m.profile_id AND p.owner_user_id = lp.owner_user_id
             LEFT JOIN vacancy_sources v ON v.profile_id = m.profile_id AND v.review_status = 'confirmed'
             WHERE m.id = ?1 AND lp.owner_user_id = ?2 AND m.status != 'expired'
                   AND m.retention_expires_at_ms > ?3",
            params![meeting_id, owner_user_id, now_ms],
            |row| {
                Ok((
                    row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?, row.get::<_, String>(4)?, row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?, row.get::<_, i64>(7)?, row.get::<_, Option<i64>>(8)?,
                    row.get::<_, Option<i64>>(9)?, row.get::<_, i64>(10)?,
                ))
            },
        )
        .optional()?
        .ok_or(HistoryError::NotFound)?;
    let artifacts = artifacts(database.connection(), meeting_id, now_ms)?;
    let transcript = transcript(database.connection(), meeting_id)?;
    let chats = chats(database.connection(), meeting_id)?;
    Ok(MeetingHistoryDetail {
        id: base.0,
        title: base.1,
        status: base.2,
        mode: base.3,
        profile_id: base.4,
        profile_name: base.5,
        vacancy_role: base.6,
        created_at_ms: base.7,
        started_at_ms: base.8,
        ended_at_ms: base.9,
        retention_expires_at_ms: base.10,
        artifacts,
        transcript,
        chats,
    })
}

pub fn export(
    database: &mut Database,
    owner_user_id: &str,
    meeting_id: &str,
    policy_allows_export: bool,
    now_ms: i64,
) -> Result<HistoryExport, HistoryError> {
    if !verify_audit_chain(database)? {
        return Err(HistoryError::AuditIntegrity);
    }
    let (roles, launch_policy_id): (String, String) = database
        .connection()
        .query_row(
            "SELECT u.roles_json, m.launch_policy_id FROM users u
             JOIN launch_policies lp ON lp.owner_user_id = u.id
             JOIN meetings m ON m.launch_policy_id = lp.id
             WHERE u.id = ?1 AND m.id = ?2",
            params![owner_user_id, meeting_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or(HistoryError::NotFound)?;
    let roles: Vec<String> =
        serde_json::from_str(&roles).map_err(|_| HistoryError::Invalid("invalid user roles"))?;
    let authorized = policy_allows_export
        && roles
            .iter()
            .any(|role| matches!(role.as_str(), "exporter" | "security_admin"));
    if !authorized {
        append_audit_event(
            database.connection(),
            AuditWrite {
                occurred_at_ms: now_ms,
                user_id: Some(owner_user_id),
                launch_policy_id: Some(&launch_policy_id),
                meeting_id: Some(meeting_id),
                action: "meeting_export",
                outcome: "denied",
                reason_code: "EXPORT_NOT_ALLOWED",
            },
        )?;
        return Err(HistoryError::Forbidden);
    }
    let meeting = detail(database, owner_user_id, meeting_id, now_ms)?;
    let bundle = HistoryExport {
        exported_at_ms: now_ms,
        meeting,
    };
    if serde_json::to_vec(&bundle)
        .map_err(|_| HistoryError::Invalid("export serialization failed"))?
        .len()
        > MAX_EXPORT_BYTES
    {
        return Err(HistoryError::ExportTooLarge);
    }
    append_audit_event(
        database.connection(),
        AuditWrite {
            occurred_at_ms: now_ms,
            user_id: Some(owner_user_id),
            launch_policy_id: Some(&launch_policy_id),
            meeting_id: Some(meeting_id),
            action: "meeting_export",
            outcome: "succeeded",
            reason_code: "OK",
        },
    )?;
    Ok(bundle)
}

pub fn verify_audit_chain(database: &Database) -> Result<bool, HistoryError> {
    let mut statement = database.connection().prepare(
        "SELECT sequence, occurred_at_ms, user_id, launch_policy_id, meeting_id, action,
                outcome, reason_code, metadata_json, previous_hash, event_hash
         FROM audit_events ORDER BY sequence",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
        ))
    })?;
    let mut previous = "genesis".to_owned();
    for row in rows {
        let row = row?;
        if row.9 != previous
            || row.10
                != audit_hash(
                    row.0,
                    row.1,
                    row.2.as_deref(),
                    row.3.as_deref(),
                    row.4.as_deref(),
                    &row.5,
                    &row.6,
                    &row.7,
                    &row.8,
                    &row.9,
                )
        {
            return Ok(false);
        }
        previous = row.10;
    }
    Ok(true)
}

pub fn append_audit_event(
    connection: &Connection,
    event: AuditWrite<'_>,
) -> Result<i64, HistoryError> {
    let previous_hash = connection
        .query_row(
            "SELECT event_hash FROM audit_events ORDER BY sequence DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .unwrap_or_else(|| "genesis".into());
    connection.execute(
        "INSERT INTO audit_events(id, occurred_at_ms, user_id, launch_policy_id, meeting_id,
                action, outcome, reason_code, metadata_json, previous_hash, event_hash, retention_expires_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '{}', ?9, '', ?10)",
        params![
            format!("audit-{}-{}", event.occurred_at_ms, connection.last_insert_rowid() + 1),
            event.occurred_at_ms,
            event.user_id,
            event.launch_policy_id,
            event.meeting_id,
            event.action,
            event.outcome,
            event.reason_code,
            previous_hash,
            event.occurred_at_ms + AUDIT_RETENTION_MS,
        ],
    )?;
    let sequence = connection.last_insert_rowid();
    let event_hash = audit_hash(
        sequence,
        event.occurred_at_ms,
        event.user_id,
        event.launch_policy_id,
        event.meeting_id,
        event.action,
        event.outcome,
        event.reason_code,
        "{}",
        &previous_hash,
    );
    connection.execute(
        "UPDATE audit_events SET event_hash = ?1 WHERE sequence = ?2 AND event_hash = ''",
        params![event_hash, sequence],
    )?;
    Ok(sequence)
}

const SEARCH_WITH_MATCH_SQL: &str =
    "SELECT m.id, m.title, m.status, m.mode, m.profile_id, p.name, COALESCE(v.role_title, ''),
            m.created_at_ms, m.ended_at_ms, m.retention_expires_at_ms
     FROM meeting_search
     JOIN meetings m ON m.id = meeting_search.meeting_id
     JOIN launch_policies lp ON lp.id = m.launch_policy_id
     JOIN ai_profiles p ON p.id = m.profile_id AND p.owner_user_id = lp.owner_user_id
     LEFT JOIN vacancy_sources v ON v.profile_id = m.profile_id AND v.review_status = 'confirmed'
     WHERE lp.owner_user_id = ?1 AND meeting_search.owner_user_id = ?1
       AND m.status != 'expired' AND m.retention_expires_at_ms > ?2
       AND meeting_search MATCH ?3
       AND (?4 IS NULL OR p.name LIKE ?4 ESCAPE '\\')
       AND (?5 IS NULL OR m.created_at_ms >= ?5)
       AND (?6 IS NULL OR m.created_at_ms <= ?6)
       AND (?7 IS NULL OR m.created_at_ms < ?7 OR (m.created_at_ms = ?7 AND m.id < ?8))
     ORDER BY m.created_at_ms DESC, m.id DESC LIMIT ?9";

const SEARCH_WITHOUT_MATCH_SQL: &str =
    "SELECT m.id, m.title, m.status, m.mode, m.profile_id, p.name, COALESCE(v.role_title, ''),
            m.created_at_ms, m.ended_at_ms, m.retention_expires_at_ms
     FROM meeting_search
     JOIN meetings m ON m.id = meeting_search.meeting_id
     JOIN launch_policies lp ON lp.id = m.launch_policy_id
     JOIN ai_profiles p ON p.id = m.profile_id AND p.owner_user_id = lp.owner_user_id
     LEFT JOIN vacancy_sources v ON v.profile_id = m.profile_id AND v.review_status = 'confirmed'
     WHERE lp.owner_user_id = ?1 AND meeting_search.owner_user_id = ?1
       AND m.status != 'expired' AND m.retention_expires_at_ms > ?2
       AND (?3 IS NULL OR p.name LIKE ?3 ESCAPE '\\')
       AND (?4 IS NULL OR m.created_at_ms >= ?4)
       AND (?5 IS NULL OR m.created_at_ms <= ?5)
       AND (?6 IS NULL OR m.created_at_ms < ?6 OR (m.created_at_ms = ?6 AND m.id < ?7))
     ORDER BY m.created_at_ms DESC, m.id DESC LIMIT ?8";

struct PreparedSearch {
    fts_query: Option<String>,
    profile_query: Option<String>,
    from_ms: Option<i64>,
    to_ms: Option<i64>,
    cursor: Option<Cursor>,
    limit: usize,
}

impl PreparedSearch {
    fn new(input: &HistorySearch) -> Result<Self, HistoryError> {
        if input.limit == 0 || input.limit > MAX_PAGE_SIZE {
            return Err(HistoryError::Invalid("invalid page size"));
        }
        if input.query.as_ref().is_some_and(|value| value.len() > 256)
            || input
                .profile_query
                .as_ref()
                .is_some_and(|value| value.len() > 100)
        {
            return Err(HistoryError::Invalid("search query is too long"));
        }
        if input
            .from_ms
            .zip(input.to_ms)
            .is_some_and(|(from, to)| from > to)
        {
            return Err(HistoryError::Invalid("invalid date range"));
        }
        let cursor = input
            .cursor
            .as_deref()
            .map(Cursor::decode)
            .transpose_option()
            .ok_or(HistoryError::Invalid("invalid cursor"))?;
        let fts_query = input
            .query
            .as_deref()
            .map(|query| fts_expression(query, input.field))
            .transpose()?;
        let profile_query = input
            .profile_query
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(like_pattern);
        Ok(Self {
            fts_query,
            profile_query,
            from_ms: input.from_ms,
            to_ms: input.to_ms,
            cursor,
            limit: input.limit,
        })
    }
}

trait TransposeOption<T> {
    fn transpose_option(self) -> Option<Option<T>>;
}

impl<T> TransposeOption<T> for Option<Option<T>> {
    fn transpose_option(self) -> Option<Option<T>> {
        match self {
            None => Some(None),
            Some(Some(value)) => Some(Some(value)),
            Some(None) => None,
        }
    }
}

fn fts_expression(query: &str, field: SearchField) -> Result<String, HistoryError> {
    let tokens = query
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .take(20)
        .map(|token| format!("\"{token}\""))
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        return Err(HistoryError::Invalid("empty search query"));
    }
    let expression = tokens.join(" AND ");
    Ok(match field {
        SearchField::Any => expression,
        SearchField::Title => format!("title : ({expression})"),
        SearchField::Vacancy => format!("vacancy : ({expression})"),
        SearchField::Transcript => format!("transcript : ({expression})"),
        SearchField::Chat => format!("chat : ({expression})"),
    })
}

fn like_pattern(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

fn history_item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryListItem> {
    Ok(HistoryListItem {
        id: row.get(0)?,
        title: row.get(1)?,
        status: row.get(2)?,
        mode: row.get(3)?,
        profile_id: row.get(4)?,
        profile_name: row.get(5)?,
        vacancy_role: row.get(6)?,
        created_at_ms: row.get(7)?,
        ended_at_ms: row.get(8)?,
        retention_expires_at_ms: row.get(9)?,
    })
}

fn artifacts(
    connection: &Connection,
    meeting_id: &str,
    now_ms: i64,
) -> Result<Vec<HistoryArtifact>, HistoryError> {
    let mut statement = connection.prepare(
        "SELECT id, kind, mime_type, byte_length, content_status, created_at_ms, expires_at_ms
         FROM artifacts WHERE meeting_id = ?1 AND content_status IN ('allowed', 'redacted')
              AND expires_at_ms > ?2 ORDER BY created_at_ms, id",
    )?;
    Ok(statement
        .query_map(params![meeting_id, now_ms], |row| {
            Ok(HistoryArtifact {
                id: row.get(0)?,
                kind: row.get(1)?,
                mime_type: row.get(2)?,
                byte_length: row.get(3)?,
                content_status: row.get(4)?,
                created_at_ms: row.get(5)?,
                expires_at_ms: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?)
}

fn transcript(
    connection: &Connection,
    meeting_id: &str,
) -> Result<Vec<HistoryTranscriptSegment>, HistoryError> {
    let mut statement = connection.prepare(
        "SELECT id, sequence, speaker, text, confidence, is_question, started_at_ms, ended_at_ms
         FROM transcript_segments WHERE meeting_id = ?1 AND is_final = 1 ORDER BY sequence LIMIT 10000",
    )?;
    Ok(statement
        .query_map([meeting_id], |row| {
            Ok(HistoryTranscriptSegment {
                id: row.get(0)?,
                sequence: row.get(1)?,
                speaker: row.get(2)?,
                text: row.get(3)?,
                confidence: row.get(4)?,
                is_question: row.get(5)?,
                started_at_ms: row.get(6)?,
                ended_at_ms: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?)
}

fn chats(connection: &Connection, meeting_id: &str) -> Result<Vec<HistoryChat>, HistoryError> {
    let mut threads = connection.prepare(
        "SELECT id, kind FROM chat_threads WHERE meeting_id = ?1 ORDER BY CASE kind WHEN 'live' THEN 0 ELSE 1 END",
    )?;
    let threads = threads
        .query_map([meeting_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    threads
        .into_iter()
        .map(|(thread_id, kind)| {
            let mut messages = connection.prepare(
                "SELECT id, sequence, role, content, created_at_ms FROM chat_messages
                 WHERE thread_id = ?1 AND status = 'complete' ORDER BY sequence LIMIT 10000",
            )?;
            let messages = messages
                .query_map([thread_id], |row| {
                    Ok(HistoryChatMessage {
                        id: row.get(0)?,
                        sequence: row.get(1)?,
                        role: row.get(2)?,
                        content: row.get(3)?,
                        created_at_ms: row.get(4)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(HistoryChat { kind, messages })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn audit_hash(
    sequence: i64,
    occurred_at_ms: i64,
    user_id: Option<&str>,
    launch_policy_id: Option<&str>,
    meeting_id: Option<&str>,
    action: &str,
    outcome: &str,
    reason_code: &str,
    metadata_json: &str,
    previous_hash: &str,
) -> String {
    let mut hasher = Sha256::new();
    for part in [
        sequence.to_string(),
        occurred_at_ms.to_string(),
        user_id.unwrap_or("").to_owned(),
        launch_policy_id.unwrap_or("").to_owned(),
        meeting_id.unwrap_or("").to_owned(),
        action.to_owned(),
        outcome.to_owned(),
        reason_code.to_owned(),
        metadata_json.to_owned(),
        previous_hash.to_owned(),
    ] {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}
