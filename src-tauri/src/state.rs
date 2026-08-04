use std::sync::Mutex;

use crate::security::audit::AuditChain;
use crate::{macos::capture::CaptureRuntime, security::policy::SafetyPolicySnapshot};

#[derive(Debug, Default)]
pub struct RuntimeState {
    pub active_user_id: Option<String>,
    pub active_meeting_id: Option<String>,
    pub capture_runtime: Option<CaptureRuntime>,
    pub policy: Option<SafetyPolicySnapshot>,
    pub audit: AuditChain,
}

#[derive(Debug, Default)]
pub struct AppState(pub Mutex<RuntimeState>);
