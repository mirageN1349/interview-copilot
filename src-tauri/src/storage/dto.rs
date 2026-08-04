use rusqlite::Row;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    pub status: String,
    pub updated_at_ms: i64,
    pub revision: i64,
}

impl ProfileSummary {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            status: row.get("status")?,
            updated_at_ms: row.get("updated_at_ms")?,
            revision: row.get("revision")?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    pub created_at_ms: i64,
    pub retention_expires_at_ms: i64,
}

impl MeetingSummary {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            title: row.get("title")?,
            status: row.get("status")?,
            created_at_ms: row.get("created_at_ms")?,
            retention_expires_at_ms: row.get("retention_expires_at_ms")?,
        })
    }
}
