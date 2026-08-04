use rusqlite::{OptionalExtension, params};
use serde::Serialize;

use crate::storage::{
    Database,
    history::{self, AuditWrite, HistoryError},
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionPlan {
    pub meeting_id: String,
    pub storage_keys: Vec<String>,
}

pub fn prepare_meeting_deletion(
    database: &mut Database,
    owner_user_id: &str,
    meeting_id: &str,
    now_ms: i64,
) -> Result<DeletionPlan, HistoryError> {
    let (launch_policy_id, status) = database
        .connection()
        .query_row(
            "SELECT m.launch_policy_id, m.status FROM meetings m
             JOIN launch_policies lp ON lp.id = m.launch_policy_id
             JOIN ai_profiles p ON p.id = m.profile_id
             WHERE m.id = ?1 AND lp.owner_user_id = ?2 AND p.owner_user_id = ?2",
            params![meeting_id, owner_user_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or(HistoryError::NotFound)?;
    if !matches!(status.as_str(), "completed" | "failed" | "expired") {
        return Err(HistoryError::Invalid(
            "active meeting content cannot be deleted",
        ));
    }
    prepare(
        database,
        meeting_id,
        Some(owner_user_id),
        &launch_policy_id,
        now_ms,
    )
}

pub fn prepare_due(
    database: &mut Database,
    now_ms: i64,
) -> Result<Vec<DeletionPlan>, HistoryError> {
    let due = {
        let mut statement = database.connection().prepare(
            "SELECT m.id, lp.owner_user_id, m.launch_policy_id FROM meetings m
             JOIN launch_policies lp ON lp.id = m.launch_policy_id
             WHERE m.retention_expires_at_ms <= ?1 AND m.status != 'running'
             ORDER BY m.retention_expires_at_ms, m.id",
        )?;
        statement
            .query_map([now_ms], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?
    };
    due.into_iter()
        .map(|(meeting_id, owner, policy)| {
            prepare(database, &meeting_id, Some(&owner), &policy, now_ms)
        })
        .collect()
}

fn prepare(
    database: &mut Database,
    meeting_id: &str,
    owner_user_id: Option<&str>,
    launch_policy_id: &str,
    now_ms: i64,
) -> Result<DeletionPlan, HistoryError> {
    database.transaction(|transaction| {
        transaction.execute(
            "UPDATE meetings SET status = 'expired', title = 'Deleted meeting', model_snapshot_json = '{}',
                    failure_code = NULL, ended_at_ms = COALESCE(ended_at_ms, ?1)
             WHERE id = ?2 AND status != 'expired'",
            params![now_ms, meeting_id],
        )?;
        transaction.execute("DELETE FROM meeting_search WHERE meeting_id = ?1", [meeting_id])?;
        transaction.execute("DELETE FROM transcript_segments WHERE meeting_id = ?1", [meeting_id])?;
        transaction.execute("DELETE FROM chat_threads WHERE meeting_id = ?1", [meeting_id])?;
        transaction.execute("DELETE FROM diagram_documents WHERE meeting_id = ?1", [meeting_id])?;
        transaction.execute("DELETE FROM capture_configurations WHERE meeting_id = ?1", [meeting_id])?;
        transaction.execute(
            "UPDATE artifacts SET content_status = 'expired' WHERE meeting_id = ?1 AND content_status != 'expired'",
            [meeting_id],
        )?;
        let audit_exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM audit_events WHERE meeting_id = ?1 AND action = 'content_retention_deleted')",
            [meeting_id],
            |row| row.get(0),
        )?;
        if !audit_exists {
            history::append_audit_event(
                transaction,
                AuditWrite {
                    occurred_at_ms: now_ms,
                    user_id: owner_user_id,
                    launch_policy_id: Some(launch_policy_id),
                    meeting_id: Some(meeting_id),
                    action: "content_retention_deleted",
                    outcome: "succeeded",
                    reason_code: "OK",
                },
            )
            .map_err(history_database_error)?;
        }
        let mut statement = transaction.prepare(
            "SELECT storage_key FROM artifacts WHERE meeting_id = ?1 AND content_status = 'expired' ORDER BY storage_key",
        )?;
        let storage_keys = statement
            .query_map([meeting_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(DeletionPlan { meeting_id: meeting_id.to_owned(), storage_keys })
    })
    .map_err(HistoryError::Database)
}

pub fn complete_file_cleanup(
    database: &mut Database,
    meeting_id: &str,
    deleted_storage_keys: &[String],
) -> Result<(), HistoryError> {
    database.transaction(|transaction| {
        for storage_key in deleted_storage_keys {
            transaction.execute(
                "DELETE FROM artifacts WHERE meeting_id = ?1 AND storage_key = ?2 AND content_status = 'expired'",
                params![meeting_id, storage_key],
            )?;
        }
        Ok(())
    })?;
    Ok(())
}

pub fn verify_orphans(database: &Database) -> Result<Vec<String>, HistoryError> {
    let mut issues = Vec::new();
    let mut foreign_keys = database.connection().prepare("PRAGMA foreign_key_check")?;
    issues.extend(
        foreign_keys
            .query_map([], |row| {
                Ok(format!(
                    "foreign-key:{}:{}",
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?,
    );
    let mut search_orphans = database.connection().prepare(
        "SELECT meeting_id FROM meeting_search
         WHERE NOT EXISTS(SELECT 1 FROM meetings WHERE meetings.id = meeting_search.meeting_id)
            OR EXISTS(SELECT 1 FROM meetings WHERE meetings.id = meeting_search.meeting_id AND meetings.status = 'expired')
         ORDER BY meeting_id",
    )?;
    issues.extend(
        search_orphans
            .query_map([], |row| Ok(format!("search:{}", row.get::<_, String>(0)?)))?
            .collect::<Result<Vec<_>, _>>()?,
    );
    let mut expired_artifacts = database.connection().prepare(
        "SELECT a.id FROM artifacts a JOIN meetings m ON m.id = a.meeting_id
         WHERE m.status = 'expired' ORDER BY a.id",
    )?;
    issues.extend(
        expired_artifacts
            .query_map([], |row| {
                Ok(format!("pending-artifact:{}", row.get::<_, String>(0)?))
            })?
            .collect::<Result<Vec<_>, _>>()?,
    );
    Ok(issues)
}

fn history_database_error(error: HistoryError) -> rusqlite::Error {
    match error {
        HistoryError::Database(error) => error,
        _ => rusqlite::Error::InvalidQuery,
    }
}
