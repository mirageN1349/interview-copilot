use interview_copilot_lib::storage::{
    Database,
    profiles::{self, ModelConfigurationInput, NewProfileSource, SaveProfileInput},
};

fn database() -> Database {
    let database = Database::in_memory().unwrap();
    for (id, email) in [
        ("user-1", "one@example.invalid"),
        ("user-2", "two@example.invalid"),
    ] {
        database
            .connection()
            .execute(
                "INSERT INTO users(id, email, display_name, roles_json, device_ids_json, status) \
             VALUES (?1, ?2, 'Fixture User', '[]', '[]', 'active')",
                (id, email),
            )
            .unwrap();
    }
    database
}

fn draft(name: &str) -> SaveProfileInput {
    SaveProfileInput {
        id: None,
        expected_revision: None,
        name: name.into(),
        manual_context: "Synthetic candidate context".into(),
        vacancy: None,
        model_configuration: None,
    }
}

#[test]
fn crud_is_owner_scoped_and_rejects_stale_updates() {
    let mut database = database();
    let created = profiles::save(&mut database, "user-1", draft("Backend"), 10).unwrap();
    assert_eq!(created.revision, 1);
    assert!(matches!(
        profiles::get(&database, "user-2", &created.id),
        Err(profiles::ProfileStoreError::NotFound)
    ));

    let mut update = draft("Backend updated");
    update.id = Some(created.id.clone());
    update.expected_revision = Some(1);
    let updated = profiles::save(&mut database, "user-1", update.clone(), 20).unwrap();
    assert_eq!(
        (updated.name.as_str(), updated.revision),
        ("Backend updated", 2)
    );
    assert!(matches!(
        profiles::save(&mut database, "user-1", update, 30),
        Err(profiles::ProfileStoreError::RevisionConflict { current: 2 })
    ));
}

#[test]
fn readiness_allows_no_vacancy_and_requires_explicit_model() {
    let mut database = database();
    let created = profiles::save(&mut database, "user-1", draft("Frontend"), 10).unwrap();
    let input = SaveProfileInput {
        id: Some(created.id.clone()),
        expected_revision: Some(1),
        name: created.name,
        manual_context: created.manual_context,
        vacancy: None,
        model_configuration: Some(ModelConfigurationInput {
            response_model_id: "mock-response".into(),
            transcription_model_id: "openai/whisper-large-v3-turbo".into(),
            translation_language: "ru".into(),
            answer_depth: "balanced".into(),
            question_confidence_threshold: 0.7,
            processing_boundary_id: "local-mock".into(),
        }),
    };
    let ready = profiles::save(&mut database, "user-1", input, 20).unwrap();
    assert_eq!(ready.status, "ready");
    assert!(ready.vacancy.is_none());
}

#[test]
fn source_import_metadata_is_stable_and_archive_checks_active_meetings() {
    let mut database = database();
    let created = profiles::save(&mut database, "user-1", draft("Systems"), 10).unwrap();
    let imported = profiles::insert_source(
        &mut database,
        "user-1",
        &created.id,
        1,
        NewProfileSource {
            id: "source-stable",
            kind: "project",
            display_name: "synthetic-project.md",
            mime_type: "text/markdown",
            storage_key: "profiles/source-stable.md",
            content_status: "redacted",
            redaction_summary: Some("Synthetic secret removed"),
            checksum: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            extracted_facts: &[],
        },
        20,
    )
    .unwrap();
    assert_eq!(imported.revision, 2);
    assert_eq!(imported.sources[0].id, "source-stable");
    assert_eq!(imported.sources[0].content_status, "redacted");

    database.connection().execute_batch(&format!(
        "INSERT INTO launch_policies(id, title, purpose, owner_user_id, status, environment_id, approved_device_ids_json, retention_days) \
         VALUES ('policy-1', 'Fixture policy', 'Synthetic policy purpose for profile test', 'user-1', 'active', 'local', '[]', 7); \
         INSERT INTO meetings(id, launch_policy_id, profile_id, profile_revision, model_snapshot_json, title, status, mode, capture_configuration_id, created_at_ms, retention_expires_at_ms) \
         VALUES ('meeting-1', 'policy-1', '{}', 2, '{{}}', 'Fixture meeting', 'running', 'standard_lab', 'capture-1', 20, 1000);",
        created.id
    )).unwrap();
    assert!(matches!(
        profiles::archive(&mut database, "user-1", &created.id, 2, 30),
        Err(profiles::ProfileStoreError::InUse)
    ));
    database
        .connection()
        .execute("UPDATE meetings SET status = 'completed'", [])
        .unwrap();
    let archived = profiles::archive(&mut database, "user-1", &created.id, 2, 40).unwrap();
    assert_eq!(
        (archived.status.as_str(), archived.revision),
        ("archived", 3)
    );
}

#[test]
fn restore_is_owner_scoped_revision_safe_and_recalculates_readiness() {
    let mut database = database();
    let draft = profiles::save(&mut database, "user-1", draft("Draft"), 10).unwrap();
    let archived = profiles::archive(&mut database, "user-1", &draft.id, 1, 20).unwrap();

    assert!(matches!(
        profiles::restore(&mut database, "user-2", &draft.id, archived.revision, 30),
        Err(profiles::ProfileStoreError::NotFound)
    ));
    assert!(matches!(
        profiles::restore(&mut database, "user-1", &draft.id, 1, 30),
        Err(profiles::ProfileStoreError::RevisionConflict { current: 2 })
    ));

    let restored =
        profiles::restore(&mut database, "user-1", &draft.id, archived.revision, 30).unwrap();
    assert_eq!(
        (
            restored.status.as_str(),
            restored.revision,
            restored.updated_at_ms
        ),
        ("draft", 3, 30)
    );

    let ready_input = SaveProfileInput {
        id: None,
        expected_revision: None,
        name: "Ready".into(),
        manual_context: "Synthetic candidate context".into(),
        vacancy: None,
        model_configuration: Some(ModelConfigurationInput {
            response_model_id: "mock-response".into(),
            transcription_model_id: "openai/whisper-large-v3-turbo".into(),
            translation_language: "ru".into(),
            answer_depth: "balanced".into(),
            question_confidence_threshold: 0.7,
            processing_boundary_id: "local-mock".into(),
        }),
    };
    let ready = profiles::save(&mut database, "user-1", ready_input, 40).unwrap();
    let archived = profiles::archive(&mut database, "user-1", &ready.id, 1, 50).unwrap();
    let restored =
        profiles::restore(&mut database, "user-1", &ready.id, archived.revision, 60).unwrap();
    assert_eq!(
        (
            restored.status.as_str(),
            restored.revision,
            restored.updated_at_ms
        ),
        ("ready", 3, 60)
    );
}
