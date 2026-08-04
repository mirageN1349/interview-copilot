use rusqlite::{Connection, OptionalExtension, params};

use crate::security::capture_matrix::{CaptureEnvironment, CaptureMatrixRow, EvidenceResult};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS capture_matrix_evidence (
  id TEXT PRIMARY KEY,
  environment_json TEXT NOT NULL CHECK(json_valid(environment_json)),
  evidence_checksum TEXT NOT NULL,
  recorded_at_ms INTEGER NOT NULL CHECK(recorded_at_ms > 0),
  signature_verified INTEGER NOT NULL CHECK(signature_verified IN (0, 1)),
  result TEXT NOT NULL CHECK(result IN ('approved', 'blocked'))
);
CREATE TRIGGER IF NOT EXISTS capture_matrix_no_update
BEFORE UPDATE ON capture_matrix_evidence BEGIN SELECT RAISE(ABORT, 'capture matrix rows are immutable'); END;
CREATE TRIGGER IF NOT EXISTS capture_matrix_no_delete
BEFORE DELETE ON capture_matrix_evidence BEGIN SELECT RAISE(ABORT, 'capture matrix rows are immutable'); END;
";

pub fn ensure_schema(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(SCHEMA)
}

pub fn persist_immutable(
    connection: &Connection,
    row: &CaptureMatrixRow,
) -> rusqlite::Result<bool> {
    ensure_schema(connection)?;
    let environment =
        serde_json::to_string(&row.environment).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let result = match row.result {
        EvidenceResult::Approved => "approved",
        EvidenceResult::Blocked => "blocked",
    };
    let inserted = connection.execute(
        "INSERT OR IGNORE INTO capture_matrix_evidence
         (id, environment_json, evidence_checksum, recorded_at_ms, signature_verified, result)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            row.id,
            environment,
            row.evidence_checksum,
            row.recorded_at_ms,
            row.signature_verified,
            result
        ],
    )?;
    if inserted == 1 {
        return Ok(true);
    }
    (find(connection, &row.id)? == Some(row.clone()))
        .then_some(false)
        .ok_or(rusqlite::Error::InvalidQuery)
}

pub fn find_exact_approved(
    connection: &Connection,
    row_id: &str,
    environment: &CaptureEnvironment,
) -> rusqlite::Result<Option<CaptureMatrixRow>> {
    Ok(find(connection, row_id)?
        .filter(|row| row.environment == *environment && row.is_approval_eligible()))
}

pub fn find(connection: &Connection, row_id: &str) -> rusqlite::Result<Option<CaptureMatrixRow>> {
    ensure_schema(connection)?;
    connection
        .query_row(
            "SELECT id, environment_json, evidence_checksum, recorded_at_ms, signature_verified, result
             FROM capture_matrix_evidence WHERE id = ?1",
            [row_id],
            |row| {
                let environment_json: String = row.get(1)?;
                let environment = serde_json::from_str(&environment_json)
                    .map_err(|error| rusqlite::Error::FromSqlConversionFailure(
                        environment_json.len(), rusqlite::types::Type::Text, Box::new(error)
                    ))?;
                Ok(CaptureMatrixRow {
                    id: row.get(0)?,
                    environment,
                    evidence_checksum: row.get(2)?,
                    recorded_at_ms: row.get(3)?,
                    signature_verified: row.get(4)?,
                    result: match row.get::<_, String>(5)?.as_str() {
                        "approved" => EvidenceResult::Approved,
                        _ => EvidenceResult::Blocked,
                    },
                })
            },
        )
        .optional()
}
