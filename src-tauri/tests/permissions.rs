use interview_copilot_lib::macos::permissions::{
    PermissionKind, PermissionState, microphone_state_from_raw, promptable_permission_state,
    settings_url,
};

#[test]
fn microphone_status_mapping_is_stable_and_fails_closed() {
    assert_eq!(microphone_state_from_raw(0), PermissionState::NotDetermined);
    assert_eq!(microphone_state_from_raw(1), PermissionState::Restricted);
    assert_eq!(microphone_state_from_raw(2), PermissionState::Denied);
    assert_eq!(microphone_state_from_raw(3), PermissionState::Granted);
    assert_eq!(microphone_state_from_raw(99), PermissionState::Restricted);
}

#[test]
fn screen_permission_can_be_requested_before_opening_settings() {
    assert_eq!(
        promptable_permission_state(false, false),
        PermissionState::NotDetermined
    );
    assert_eq!(
        promptable_permission_state(false, true),
        PermissionState::Denied
    );
    assert_eq!(
        promptable_permission_state(true, false),
        PermissionState::Granted
    );
}

#[test]
fn each_permission_has_a_fixed_system_settings_deep_link() {
    assert!(settings_url(PermissionKind::Screen).ends_with("Privacy_ScreenCapture"));
    assert!(settings_url(PermissionKind::Microphone).ends_with("Privacy_Microphone"));
    assert!(settings_url(PermissionKind::Accessibility).ends_with("Privacy_Accessibility"));
}

#[test]
fn permission_contract_serializes_with_public_wire_names() {
    assert_eq!(
        serde_json::to_string(&PermissionKind::Accessibility).unwrap(),
        "\"accessibility\""
    );
    assert_eq!(
        serde_json::to_string(&PermissionState::NotDetermined).unwrap(),
        "\"not_determined\""
    );
}
