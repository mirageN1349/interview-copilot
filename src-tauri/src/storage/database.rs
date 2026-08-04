use std::path::Path;

use rusqlite::{Connection, OpenFlags, TransactionBehavior};

const INITIAL_MIGRATION: &str = include_str!("../../migrations/001_initial.sql");

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                rusqlite::Error::InvalidPath(format!("{parent:?}: {error}").into())
            })?;
        }
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;
        Self::initialize(connection)
    }

    pub fn in_memory() -> rusqlite::Result<Self> {
        Self::initialize(Connection::open_in_memory()?)
    }

    fn initialize(connection: Connection) -> rusqlite::Result<Self> {
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.execute_batch(INITIAL_MIGRATION)?;
        Ok(Self { connection })
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    pub fn transaction<T>(
        &mut self,
        operation: impl FnOnce(&rusqlite::Transaction<'_>) -> rusqlite::Result<T>,
    ) -> rusqlite::Result<T> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let result = operation(&transaction)?;
        transaction.commit()?;
        Ok(result)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cursor {
    pub created_at_ms: i64,
    pub id: String,
}

impl Cursor {
    pub fn encode(&self) -> String {
        format!("{}:{}", self.created_at_ms, self.id)
    }

    pub fn decode(value: &str) -> Option<Self> {
        let (created_at_ms, id) = value.split_once(':')?;
        let created_at_ms = created_at_ms.parse().ok()?;
        (!id.is_empty()).then(|| Self {
            created_at_ms,
            id: id.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_existing_database_is_idempotent() {
        let path = std::env::temp_dir().join(format!(
            "interview-copilot-reopen-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        drop(Database::open(&path).unwrap());
        drop(Database::open(&path).unwrap());
        std::fs::remove_file(path).unwrap();
    }
}
