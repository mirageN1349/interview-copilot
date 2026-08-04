use interview_copilot_lib::storage::{
    Database,
    meetings::{self, CreateMeetingInput, MeetingStoreError},
    messages::{self, AppendMessageInput, TranscriptInput},
};

fn database() -> Database {
    let database = Database::in_memory().unwrap();
    database
        .connection()
        .execute_batch(
            "INSERT INTO users(id, email, display_name, roles_json, device_ids_json, status) VALUES
               ('user-1', 'one@example.invalid', 'One', '[]', '[]', 'active'),
               ('user-2', 'two@example.invalid', 'Two', '[]', '[]', 'active');
             INSERT INTO launch_policies(id, title, purpose, owner_user_id, status, environment_id, approved_device_ids_json, retention_days) VALUES
               ('policy-1', 'Policy', 'Synthetic policy purpose for meeting tests', 'user-1', 'active', 'local', '[]', 7);
             INSERT INTO model_configurations(id, response_model_id, transcription_model_id, translation_language, answer_depth, question_confidence_threshold, processing_boundary_id) VALUES
               ('model-1', 'response-v1', 'transcription-v1', 'ru', 'balanced', 0.7, 'local-mock');
             INSERT INTO ai_profiles(id, owner_user_id, name, status, manual_context, model_configuration_id, created_at_ms, updated_at_ms, revision) VALUES
               ('profile-1', 'user-1', 'Backend', 'ready', 'Fixture context', 'model-1', 1, 1, 3);",
        )
        .unwrap();
    database
}

fn create(database: &mut Database) -> meetings::Meeting {
    meetings::create(
        database,
        "user-1",
        CreateMeetingInput {
            id: "meeting-1".into(),
            launch_policy_id: "policy-1".into(),
            profile_id: "profile-1".into(),
            title: "Synthetic interview".into(),
            mode: "standard_lab".into(),
            capture_configuration_id: "capture-1".into(),
            display_id: 1,
            sound_threshold: 0.2,
            retention_expires_at_ms: 10_000,
        },
        100,
    )
    .unwrap()
}

#[test]
fn creation_is_owner_scoped_and_snapshots_profile_with_exactly_two_threads() {
    let mut database = database();
    let meeting = create(&mut database);
    assert_eq!(
        (meeting.profile_revision, meeting.status.as_str()),
        (3, "prepared")
    );
    assert_eq!(meeting.model_snapshot["responseModelId"], "response-v1");
    assert_eq!(
        messages::threads(&database, "user-1", &meeting.id)
            .unwrap()
            .len(),
        2
    );

    database
        .connection()
        .execute(
            "UPDATE model_configurations SET response_model_id = 'response-v2' WHERE id = 'model-1'",
            [],
        )
        .unwrap();
    assert_eq!(
        meetings::get(&database, "user-1", &meeting.id)
            .unwrap()
            .model_snapshot["responseModelId"],
        "response-v1"
    );
    assert!(matches!(
        meetings::get(&database, "user-2", &meeting.id),
        Err(MeetingStoreError::NotFound)
    ));
}

#[test]
fn meetings_can_reuse_the_same_capture_settings() {
    let mut database = database();
    let first = create(&mut database);
    let second = meetings::create(
        &mut database,
        "user-1",
        CreateMeetingInput {
            id: "meeting-2".into(),
            launch_policy_id: "policy-1".into(),
            profile_id: "profile-1".into(),
            title: "Second interview".into(),
            mode: "standard_lab".into(),
            capture_configuration_id: "capture-1".into(),
            display_id: 1,
            sound_threshold: 0.2,
            retention_expires_at_ms: 10_000,
        },
        101,
    )
    .unwrap();

    assert_eq!(
        first.capture_configuration_id,
        second.capture_configuration_id
    );
    let count: i64 = database
        .connection()
        .query_row("SELECT COUNT(*) FROM capture_configurations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn transitions_are_strict_and_finalization_is_idempotent() {
    let mut database = database();
    create(&mut database);
    assert!(matches!(
        meetings::transition(&mut database, "user-1", "meeting-1", "running", 110, None),
        Err(MeetingStoreError::InvalidTransition { .. })
    ));
    meetings::transition(&mut database, "user-1", "meeting-1", "gating", 110, None).unwrap();
    let running =
        meetings::transition(&mut database, "user-1", "meeting-1", "running", 120, None).unwrap();
    assert_eq!(running.started_at_ms, Some(120));

    let first = meetings::finalize(&mut database, "user-1", "meeting-1", 200).unwrap();
    let repeated = meetings::finalize(&mut database, "user-1", "meeting-1", 999).unwrap();
    assert_eq!(
        (first.status.as_str(), first.ended_at_ms),
        ("completed", Some(200))
    );
    assert_eq!(repeated.ended_at_ms, Some(200));
}

#[test]
fn messages_and_transcripts_are_ordered_idempotent_and_owner_scoped() {
    let mut database = database();
    create(&mut database);
    meetings::transition(&mut database, "user-1", "meeting-1", "gating", 110, None).unwrap();
    meetings::transition(&mut database, "user-1", "meeting-1", "running", 120, None).unwrap();

    let live = messages::append(
        &mut database,
        "user-1",
        "meeting-1",
        "live",
        AppendMessageInput {
            id: "message-1".into(),
            role: "assistant".into(),
            content: "Fixture answer".into(),
            status: "complete".into(),
            reply_to_segment_id: None,
            artifact_ids: vec![],
            profile_source_ids: vec![],
            context_generation: 0,
            confidence: Some(0.9),
        },
        130,
    )
    .unwrap();
    let repeated = messages::append(
        &mut database,
        "user-1",
        "meeting-1",
        "live",
        AppendMessageInput {
            id: "message-1".into(),
            role: "assistant".into(),
            content: "Fixture answer".into(),
            status: "complete".into(),
            reply_to_segment_id: None,
            artifact_ids: vec![],
            profile_source_ids: vec![],
            context_generation: 0,
            confidence: Some(0.9),
        },
        999,
    )
    .unwrap();
    assert_eq!((live.sequence, repeated.sequence), (0, 0));
    assert!(
        messages::list(&database, "user-1", "meeting-1", "side")
            .unwrap()
            .is_empty()
    );

    let partial = TranscriptInput {
        id: "segment-1".into(),
        sequence: 0,
        speaker: "interviewer".into(),
        text: "What is".into(),
        confidence: 0.7,
        is_final: false,
        is_question: false,
        started_at_ms: 0,
        ended_at_ms: 100,
    };
    messages::save_transcript(&mut database, "user-1", "meeting-1", partial).unwrap();
    let final_segment = TranscriptInput {
        id: "segment-1".into(),
        sequence: 0,
        speaker: "interviewer".into(),
        text: "What is ownership?".into(),
        confidence: 0.95,
        is_final: true,
        is_question: true,
        started_at_ms: 0,
        ended_at_ms: 250,
    };
    messages::save_transcript(&mut database, "user-1", "meeting-1", final_segment.clone()).unwrap();
    messages::save_transcript(&mut database, "user-1", "meeting-1", final_segment).unwrap();
    assert_eq!(
        messages::transcript(&database, "user-1", "meeting-1").unwrap()[0].text,
        "What is ownership?"
    );
    assert!(messages::list(&database, "user-2", "meeting-1", "live").is_err());
}
