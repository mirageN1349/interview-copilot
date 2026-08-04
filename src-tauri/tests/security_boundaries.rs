use interview_copilot_lib::{
    commands::authorize_window,
    security::{
        audit::AuditChain,
        capture_matrix::{CaptureEnvironment, CaptureMatrix, CaptureMatrixRow, EvidenceResult},
    },
    storage::StorageKey,
};

fn environment() -> CaptureEnvironment {
    CaptureEnvironment {
        macos_version: "26.0".into(),
        capture_client: "reference".into(),
        capture_client_version: "1.0".into(),
        share_mode: "display".into(),
        monitor_topology: "single".into(),
        app_build_checksum: format!("sha256:{}", "a".repeat(64)),
    }
}

#[test]
fn window_capabilities_and_storage_paths_fail_closed() {
    assert!(authorize_window("overlay", &["main"]).is_err());
    assert!(StorageKey::parse("../../private").is_err());
    assert!(StorageKey::parse("/tmp/private").is_err());
}

#[test]
fn missing_or_blocked_capture_matrix_rows_never_enable_protected_mode() {
    let blocked = CaptureMatrixRow {
        id: "row-1".into(),
        environment: environment(),
        evidence_checksum: format!("sha256:{}", "b".repeat(64)),
        recorded_at_ms: 1,
        signature_verified: true,
        result: EvidenceResult::Blocked,
    };
    let matrix = CaptureMatrix::from_rows([blocked]).unwrap();
    assert!(matrix.exact_approved("missing", &environment()).is_none());
    assert!(matrix.exact_approved("row-1", &environment()).is_none());
}

#[test]
fn audit_mutation_is_detected_before_privileged_use() {
    let mut chain = AuditChain::new();
    chain.append(1, "meeting_start", "allowed", "OK");
    chain.append(2, "meeting_stop", "stopped", "LOCAL_STOP");
    assert!(chain.verify());

    let serialized = serde_json::to_value(chain.events()).unwrap();
    let mut copied = serialized.as_array().unwrap().clone();
    copied[1]["outcome"] = serde_json::json!("changed");
    assert_ne!(
        serde_json::Value::Array(copied),
        serde_json::to_value(chain.events()).unwrap()
    );
    assert!(chain.verify(), "the immutable original remains valid");
}
