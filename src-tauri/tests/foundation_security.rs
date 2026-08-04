use interview_copilot_lib::storage::{AppDataFiles, Cursor, Database, StorageKey};

fn temporary_directory(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "interview-copilot-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn migration_creates_schema_and_fts() {
    let database = Database::in_memory().unwrap();
    let version: i64 = database
        .connection()
        .query_row("SELECT version FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(version, 1);
    database
        .connection()
        .execute(
            "INSERT INTO meeting_search(meeting_id, owner_user_id, created_at_ms, title) VALUES (?1, ?2, ?3, ?4)",
            ("meeting-1", "user-1", 1_i64, "Rust interview"),
        )
        .unwrap();
    let found: i64 = database
        .connection()
        .query_row(
            "SELECT count(*) FROM meeting_search WHERE meeting_search MATCH 'Rust'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(found, 1);
}

#[test]
fn failed_transaction_rolls_back_all_rows() {
    let mut database = Database::in_memory().unwrap();
    let result: rusqlite::Result<()> = database.transaction(|transaction| {
        transaction.execute(
            "INSERT INTO users(id, email, display_name, roles_json, device_ids_json, status) VALUES (?1, ?2, ?3, '[]', '[]', 'active')",
            ("user-1", "test@example.invalid", "Test User"),
        )?;
        Err(rusqlite::Error::InvalidQuery)
    });
    assert!(result.is_err());
    let rows: i64 = database
        .connection()
        .query_row("SELECT count(*) FROM users", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 0);
}

#[test]
fn cursor_is_stable_and_rejects_malformed_input() {
    let cursor = Cursor {
        created_at_ms: 1_725_000_000_000,
        id: "018f-id".into(),
    };
    assert_eq!(Cursor::decode(&cursor.encode()), Some(cursor));
    assert_eq!(Cursor::decode("bad"), None);
    assert_eq!(Cursor::decode("1:"), None);
}

#[test]
fn app_data_files_reject_traversal_and_round_trip_opaque_keys() {
    let root = temporary_directory("files");
    let files = AppDataFiles::new(&root).unwrap();
    assert!(StorageKey::parse("../secret").is_err());
    assert!(StorageKey::parse("/tmp/secret").is_err());

    let key = StorageKey::parse("artifacts/meeting-1.bin").unwrap();
    files.write(&key, b"approved fixture").unwrap();
    assert_eq!(files.read(&key).unwrap(), b"approved fixture");
    files.delete(&key).unwrap();
    assert!(!root.join(key.as_str()).exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn app_data_files_reject_symlink_escape() {
    use std::os::unix::fs::symlink;

    let root = temporary_directory("symlink-root");
    let outside = temporary_directory("symlink-outside");
    symlink(&outside, root.join("escape")).unwrap();

    let files = AppDataFiles::new(&root).unwrap();
    let key = StorageKey::parse("escape/leak.bin").unwrap();
    let error = files.write(&key, b"must not escape").unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
    assert!(!outside.join("leak.bin").exists());

    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(outside).unwrap();
}

// The remaining T011 contracts (run gate, audit mutation and per-window capability)
// are exercised in this file once their T013/T014 modules land. Keeping those tests
// beside storage avoids duplicating their security implementation here.
