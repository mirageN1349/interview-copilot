use interview_copilot_lib::{
    macos::{
        presentation::{PresentationMode, PresentationState},
        screenshot::own_capture_privacy,
    },
    security::{
        capture_matrix::{
            CaptureEnvironment, CaptureMatrix, CaptureMatrixRow, EvidenceResult, GuaranteeLevel,
        },
        policy::{KillSwitch, PolicyAction, PolicyEnforcer, SafetyPolicySnapshot},
    },
    storage::{Database, capture_matrix as capture_matrix_storage},
};

fn environment() -> CaptureEnvironment {
    CaptureEnvironment {
        macos_version: "15.6.1".into(),
        capture_client: "reference-recorder".into(),
        capture_client_version: "1.0.0".into(),
        share_mode: "display".into(),
        monitor_topology: "single-retina".into(),
        app_build_checksum: format!("sha256:{}", "a".repeat(64)),
    }
}

fn row(result: EvidenceResult, signed: bool) -> CaptureMatrixRow {
    CaptureMatrixRow {
        id: "row-1".into(),
        environment: environment(),
        evidence_checksum: format!("sha256:{}", "b".repeat(64)),
        recorded_at_ms: 1_700_000_000_000,
        signature_verified: signed,
        result,
    }
}

#[test]
fn matrix_requires_an_exact_immutable_approved_signed_row() {
    let matrix = CaptureMatrix::from_rows([row(EvidenceResult::Approved, true)]).unwrap();
    assert!(matrix.exact_approved("row-1", &environment()).is_some());

    let mut changed = environment();
    changed.capture_client_version = "1.0.1".into();
    assert!(matrix.exact_approved("row-1", &changed).is_none());
    assert!(
        CaptureMatrix::from_rows([row(EvidenceResult::Blocked, true)])
            .unwrap()
            .exact_approved("row-1", &environment())
            .is_none()
    );
    assert!(
        CaptureMatrix::from_rows([row(EvidenceResult::Approved, false)])
            .unwrap()
            .exact_approved("row-1", &environment())
            .is_none()
    );
    assert!(
        CaptureMatrix::from_rows([
            row(EvidenceResult::Approved, true),
            row(EvidenceResult::Approved, true),
        ])
        .is_err()
    );
    assert_eq!(GuaranteeLevel::BestEffort.as_str(), "best_effort");
}

#[test]
fn persisted_matrix_rows_cannot_be_replaced_or_deleted() {
    let database = Database::in_memory().unwrap();
    let approved = row(EvidenceResult::Approved, true);
    assert!(capture_matrix_storage::persist_immutable(database.connection(), &approved).unwrap());
    assert!(!capture_matrix_storage::persist_immutable(database.connection(), &approved).unwrap());

    let mut changed = approved.clone();
    changed.environment.capture_client_version = "2.0.0".into();
    assert!(capture_matrix_storage::persist_immutable(database.connection(), &changed).is_err());
    assert!(
        database
            .connection()
            .execute("DELETE FROM capture_matrix_evidence WHERE id = 'row-1'", [])
            .is_err()
    );
}

#[test]
fn own_capture_privacy_never_claims_third_party_cursor_control() {
    let privacy = own_capture_privacy();
    assert!(privacy.current_application_excluded);
    assert!(!privacy.cursor_included);
    assert!(!privacy.third_party_cursor_controlled);
}

#[test]
fn presentation_restoration_is_unconditional_and_idempotent() {
    let mut state = PresentationState::default();
    state.apply(PresentationMode::Generic).unwrap();
    assert_eq!(state.mode(), PresentationMode::Generic);
    assert!(state.restore_standard());
    assert!(!state.restore_standard());
    assert_eq!(state.mode(), PresentationMode::Standard);
}

#[test]
fn generic_presentation_asset_fails_closed_without_an_approved_checksum() {
    assert!(
        !interview_copilot_lib::macos::presentation::generic_asset_approved(std::path::Path::new(
            "/missing/generic.icns"
        ))
    );
}

fn policy(expires_at_ms: i64, kill_switch: KillSwitch) -> SafetyPolicySnapshot {
    SafetyPolicySnapshot {
        policy_version: "v1".into(),
        user_id: "user-1".into(),
        device_id: "device-1".into(),
        environment_id: "env-1".into(),
        allow_adversarial: false,
        allow_export: false,
        kill_switch,
        expires_at_ms,
        verified: true,
    }
}

#[test]
fn stale_or_lost_policy_stops_once_without_network_acknowledgement() {
    let mut enforcer = PolicyEnforcer::default();
    assert_eq!(
        enforcer.apply(policy(200, KillSwitch::Clear), 100),
        PolicyAction::Continue
    );
    assert_eq!(enforcer.tick(200), PolicyAction::StopAll);
    assert_eq!(enforcer.tick(201), PolicyAction::AlreadyStopped);
    assert_eq!(
        enforcer.apply(policy(500, KillSwitch::Clear), 202),
        PolicyAction::AlreadyStopped
    );

    let mut enforcer = PolicyEnforcer::default();
    assert_eq!(enforcer.transport_lost(), PolicyAction::StopAll);
    assert_eq!(enforcer.transport_lost(), PolicyAction::AlreadyStopped);

    let mut enforcer = PolicyEnforcer::default();
    assert_eq!(
        enforcer.apply(policy(500, KillSwitch::StopAll), 100),
        PolicyAction::StopAll
    );
    assert_eq!(
        enforcer.apply(policy(500, KillSwitch::StopAll), 101),
        PolicyAction::AlreadyStopped
    );
}
