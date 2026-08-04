#[allow(dead_code)]
#[path = "../src/storage/database.rs"]
mod database;
#[path = "../src/storage/history.rs"]
mod history;
#[path = "../src/storage/retention.rs"]
mod retention;

mod storage {
    pub use crate::database::{Cursor, Database};

    pub mod history {
        pub use crate::history::*;
    }
}

use std::time::{Duration, Instant};

use database::Database;
use history::{HistoryError, HistorySearch, SearchField};

const DAY: i64 = 86_400_000;

fn fixture() -> Database {
    let database = Database::in_memory().unwrap();
    database
        .connection()
        .execute_batch(
            "INSERT INTO users(id, email, display_name, roles_json, device_ids_json, status) VALUES
               ('user-1', 'one@example.test', 'One', '[\"exporter\"]', '[]', 'active'),
               ('user-2', 'two@example.test', 'Two', '[]', '[]', 'active');
             INSERT INTO launch_policies(id, title, purpose, owner_user_id, status, environment_id, approved_device_ids_json, retention_days) VALUES
               ('policy-1', 'Policy 1', 'Synthetic policy for retained history tests', 'user-1', 'active', 'local', '[]', 30),
               ('policy-2', 'Policy 2', 'Synthetic policy for authorization tests', 'user-2', 'active', 'local', '[]', 30);
             INSERT INTO model_configurations(id, response_model_id, transcription_model_id, translation_language, answer_depth, question_confidence_threshold, processing_boundary_id) VALUES
               ('models', 'response', 'transcription', 'en', 'brief', 0.8, 'local');
             INSERT INTO ai_profiles(id, owner_user_id, name, status, model_configuration_id, created_at_ms, updated_at_ms) VALUES
               ('profile-rust', 'user-1', 'Backend profile', 'ready', 'models', 1, 1),
               ('profile-web', 'user-1', 'Frontend profile', 'ready', 'models', 1, 1),
               ('profile-other', 'user-2', 'Other profile', 'ready', 'models', 1, 1);
             INSERT INTO vacancy_sources(id, profile_id, source_kind, source_value, role_title, company_context, responsibilities_json, requirements_json, review_status, provenance_json) VALUES
               ('vacancy-rust', 'profile-rust', 'pasted_text', 'fixture', 'Rust platform engineer', 'Search company', '[\"Build APIs\"]', '[\"Rust\",\"SQLite\"]', 'confirmed', '{}'),
               ('vacancy-web', 'profile-web', 'pasted_text', 'fixture', 'Vue engineer', 'Web company', '[]', '[\"Vue\"]', 'confirmed', '{}');",
        )
        .unwrap();
    seed_meeting(
        &database,
        "meeting-rust",
        "policy-1",
        "profile-rust",
        "Rust systems interview",
        300,
        30 * DAY,
        "How does ownership work?",
    );
    seed_meeting(
        &database,
        "meeting-web",
        "policy-1",
        "profile-web",
        "UI architecture",
        200,
        30 * DAY,
        "Explain reactive rendering",
    );
    seed_meeting(
        &database,
        "meeting-other",
        "policy-2",
        "profile-other",
        "Private interview",
        400,
        30 * DAY,
        "Secret transcript",
    );
    database
}

#[allow(clippy::too_many_arguments)]
fn seed_meeting(
    database: &Database,
    id: &str,
    policy: &str,
    profile: &str,
    title: &str,
    created_at: i64,
    expires_at: i64,
    transcript: &str,
) {
    database.connection().execute(
        "INSERT INTO meetings(id, launch_policy_id, profile_id, profile_revision, model_snapshot_json, title, status, mode, capture_configuration_id, created_at_ms, ended_at_ms, retention_expires_at_ms)
         VALUES (?1, ?2, ?3, 1, '{}', ?4, 'completed', 'standard_lab', ?5, ?6, ?6, ?7)",
        rusqlite::params![id, policy, profile, title, format!("capture-{id}"), created_at, expires_at],
    ).unwrap();
    database.connection().execute(
        "INSERT INTO transcript_segments(id, meeting_id, sequence, speaker, text, confidence, is_final, is_question, started_at_ms, ended_at_ms)
         VALUES (?1, ?2, 0, 'interviewer', ?3, 0.99, 1, 1, ?4, ?4)",
        rusqlite::params![format!("transcript-{id}"), id, transcript, created_at],
    ).unwrap();
    database.connection().execute(
        "INSERT INTO chat_threads(id, meeting_id, kind, created_at_ms) VALUES (?1, ?2, 'live', ?3)",
        rusqlite::params![format!("thread-{id}"), id, created_at],
    ).unwrap();
    database.connection().execute(
        "INSERT INTO chat_messages(id, thread_id, sequence, role, content, status, context_generation, created_at_ms)
         VALUES (?1, ?2, 0, 'assistant', 'Structured answer', 'complete', 0, ?3)",
        rusqlite::params![format!("message-{id}"), format!("thread-{id}"), created_at],
    ).unwrap();
}

#[test]
fn search_is_authorized_fielded_and_cursor_stable() {
    let mut database = fixture();
    history::index_owner(&mut database, "user-1").unwrap();
    history::index_owner(&mut database, "user-2").unwrap();

    let title = history::search(
        &database,
        "user-1",
        &HistorySearch {
            query: Some("Rust systems".into()),
            field: SearchField::Title,
            limit: 20,
            ..Default::default()
        },
        1_000,
    )
    .unwrap();
    assert_eq!(title.items[0].id, "meeting-rust");
    assert!(title.items.iter().all(|item| item.id != "meeting-other"));

    for search in [
        HistorySearch {
            query: Some("SQLite".into()),
            field: SearchField::Vacancy,
            ..Default::default()
        },
        HistorySearch {
            query: Some("ownership".into()),
            field: SearchField::Transcript,
            ..Default::default()
        },
        HistorySearch {
            profile_query: Some("Backend".into()),
            ..Default::default()
        },
        HistorySearch {
            from_ms: Some(250),
            to_ms: Some(350),
            ..Default::default()
        },
    ] {
        assert_eq!(
            history::search(&database, "user-1", &search, 1_000)
                .unwrap()
                .items[0]
                .id,
            "meeting-rust"
        );
    }

    let first = history::search(
        &database,
        "user-1",
        &HistorySearch {
            limit: 1,
            ..Default::default()
        },
        1_000,
    )
    .unwrap();
    assert_eq!(first.items[0].id, "meeting-rust");
    let second = history::search(
        &database,
        "user-1",
        &HistorySearch {
            limit: 1,
            cursor: first.next_cursor,
            ..Default::default()
        },
        1_000,
    )
    .unwrap();
    assert_eq!(second.items[0].id, "meeting-web");
    assert!(matches!(
        history::search(
            &database,
            "user-1",
            &HistorySearch {
                cursor: Some("not-a-cursor".into()),
                ..Default::default()
            },
            1_000,
        ),
        Err(HistoryError::Invalid("invalid cursor"))
    ));
}

#[test]
fn detail_hides_storage_keys_and_expired_or_unauthorized_content() {
    let database = fixture();
    database.connection().execute(
        "INSERT INTO artifacts(id, meeting_id, kind, storage_key, mime_type, byte_length, checksum, content_status, created_at_ms, expires_at_ms)
         VALUES ('artifact-1', 'meeting-rust', 'recording', 'recordings/private.caf', 'audio/x-caf', 20, 'checksum', 'allowed', 300, ?1)",
        [30 * DAY],
    ).unwrap();
    let detail = history::detail(&database, "user-1", "meeting-rust", 1_000).unwrap();
    assert_eq!(detail.artifacts[0].id, "artifact-1");
    assert_eq!(detail.transcript[0].text, "How does ownership work?");
    assert_eq!(detail.chats[0].messages[0].content, "Structured answer");
    assert!(matches!(
        history::detail(&database, "user-2", "meeting-rust", 1_000),
        Err(HistoryError::NotFound)
    ));
    database
        .connection()
        .execute(
            "UPDATE meetings SET retention_expires_at_ms = 1 WHERE id = 'meeting-rust'",
            [],
        )
        .unwrap();
    assert!(matches!(
        history::detail(&database, "user-1", "meeting-rust", 1_000),
        Err(HistoryError::NotFound)
    ));
}

#[test]
fn deletion_and_retention_are_idempotent_and_keep_minimal_audit() {
    let mut database = fixture();
    history::index_owner(&mut database, "user-1").unwrap();
    database
        .connection()
        .execute(
            "UPDATE meetings SET status = 'running' WHERE id = 'meeting-web'",
            [],
        )
        .unwrap();
    assert!(matches!(
        retention::prepare_meeting_deletion(&mut database, "user-1", "meeting-web", 1_000),
        Err(HistoryError::Invalid(
            "active meeting content cannot be deleted"
        ))
    ));
    database
        .connection()
        .execute(
            "UPDATE meetings SET status = 'completed' WHERE id = 'meeting-web'",
            [],
        )
        .unwrap();
    database.connection().execute(
        "INSERT INTO artifacts(id, meeting_id, kind, storage_key, mime_type, byte_length, checksum, content_status, created_at_ms, expires_at_ms)
         VALUES ('artifact-1', 'meeting-rust', 'recording', 'recordings/one.caf', 'audio/x-caf', 20, 'checksum', 'allowed', 300, 400)",
        [],
    ).unwrap();

    let first = retention::prepare_meeting_deletion(&mut database, "user-1", "meeting-rust", 1_000)
        .unwrap();
    let repeated =
        retention::prepare_meeting_deletion(&mut database, "user-1", "meeting-rust", 1_001)
            .unwrap();
    assert_eq!(first.storage_keys, vec!["recordings/one.caf"]);
    assert_eq!(repeated.storage_keys, first.storage_keys);
    assert!(matches!(
        history::detail(&database, "user-1", "meeting-rust", 1_001),
        Err(HistoryError::NotFound)
    ));
    assert_eq!(
        database
            .connection()
            .query_row(
                "SELECT count(*) FROM meeting_search WHERE meeting_id = 'meeting-rust'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(database.connection().query_row("SELECT count(*) FROM audit_events WHERE meeting_id = 'meeting-rust' AND action = 'content_retention_deleted'", [], |row| row.get::<_, i64>(0)).unwrap(), 1);

    retention::complete_file_cleanup(&mut database, "meeting-rust", &first.storage_keys).unwrap();
    retention::complete_file_cleanup(&mut database, "meeting-rust", &first.storage_keys).unwrap();
    assert!(retention::verify_orphans(&database).unwrap().is_empty());

    database
        .connection()
        .execute(
            "UPDATE meetings SET retention_expires_at_ms = 5 WHERE id = 'meeting-web'",
            [],
        )
        .unwrap();
    let due = retention::prepare_due(&mut database, 10).unwrap();
    assert_eq!(
        due.iter()
            .map(|plan| plan.meeting_id.as_str())
            .collect::<Vec<_>>(),
        vec!["meeting-web"]
    );
    assert!(history::detail(&database, "user-1", "meeting-web", 10).is_err());
}

#[test]
fn export_is_role_and_policy_gated_and_refuses_a_broken_audit_chain() {
    let mut database = fixture();
    let denied = history::export(&mut database, "user-1", "meeting-rust", false, 1_000);
    assert!(matches!(denied, Err(HistoryError::Forbidden)));
    let bundle = history::export(&mut database, "user-1", "meeting-rust", true, 1_001).unwrap();
    assert_eq!(bundle.meeting.id, "meeting-rust");
    assert!(history::verify_audit_chain(&database).unwrap());
    database
        .connection()
        .execute(
            "UPDATE audit_events SET reason_code = 'TAMPERED' WHERE sequence = 1",
            [],
        )
        .unwrap();
    assert!(!history::verify_audit_chain(&database).unwrap());
    assert!(matches!(
        history::export(&mut database, "user-1", "meeting-rust", true, 1_002),
        Err(HistoryError::AuditIntegrity)
    ));
}

#[test]
fn benchmark_10k_field_searches_and_query_plan() {
    let database = fixture();
    let transaction = database.connection().unchecked_transaction().unwrap();
    for index in 0..10_000_i64 {
        let id = format!("bulk-{index:05}");
        transaction.execute(
            "INSERT INTO meetings(id, launch_policy_id, profile_id, profile_revision, model_snapshot_json, title, status, mode, capture_configuration_id, created_at_ms, ended_at_ms, retention_expires_at_ms)
             VALUES (?1, 'policy-1', 'profile-rust', 1, '{}', ?2, 'completed', 'standard_lab', ?3, ?4, ?4, ?5)",
            rusqlite::params![id, format!("Synthetic {index}"), format!("capture-{index}"), 10_000 + index, 90 * DAY],
        ).unwrap();
        transaction.execute(
            "INSERT INTO meeting_search(meeting_id, owner_user_id, created_at_ms, title, vacancy, transcript, chat)
             VALUES (?1, 'user-1', ?2, ?3, 'Rust platform role', ?4, 'complete answer')",
            rusqlite::params![id, 10_000 + index, format!("Synthetic {index}"), format!("needle-{index}")],
        ).unwrap();
    }
    transaction.commit().unwrap();

    let queries = [
        HistorySearch {
            query: Some("Synthetic 7777".into()),
            field: SearchField::Title,
            ..Default::default()
        },
        HistorySearch {
            query: Some("platform".into()),
            field: SearchField::Vacancy,
            ..Default::default()
        },
        HistorySearch {
            query: Some("needle-7777".into()),
            field: SearchField::Transcript,
            ..Default::default()
        },
        HistorySearch {
            profile_query: Some("Backend".into()),
            from_ms: Some(17_000),
            to_ms: Some(18_000),
            ..Default::default()
        },
    ];
    let mut durations = Vec::new();
    for iteration in 0..20 {
        let query = &queries[iteration % queries.len()];
        let started = Instant::now();
        let page = history::search(&database, "user-1", query, 20_000).unwrap();
        assert!(!page.items.is_empty());
        durations.push(started.elapsed());
    }
    durations.sort();
    let p95 = durations[durations.len() * 95 / 100];
    let plan = history::search_query_plan(&database, &queries[0], "user-1", 20_000).unwrap();
    eprintln!(
        "history_search_p95_ms={} plan={}",
        p95.as_millis(),
        plan.join(" | ")
    );
    assert!(p95 < Duration::from_secs(2));
    assert!(plan.iter().any(|line| line.contains("VIRTUAL TABLE INDEX")));
}
