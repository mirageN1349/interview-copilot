use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KillSwitch {
    Clear,
    StopNew,
    StopAll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyPolicySnapshot {
    pub policy_version: String,
    pub user_id: String,
    pub device_id: String,
    pub environment_id: String,
    pub allow_adversarial: bool,
    pub allow_export: bool,
    pub kill_switch: KillSwitch,
    pub expires_at_ms: i64,
    pub verified: bool,
}

impl SafetyPolicySnapshot {
    pub fn is_fresh(&self, now_ms: i64) -> bool {
        self.verified && self.expires_at_ms > now_ms
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyAction {
    Continue,
    StopAll,
    AlreadyStopped,
}

#[derive(Debug, Default)]
pub struct PolicyEnforcer {
    snapshot: Option<SafetyPolicySnapshot>,
    stopped: bool,
}

impl PolicyEnforcer {
    pub fn apply(&mut self, snapshot: SafetyPolicySnapshot, now_ms: i64) -> PolicyAction {
        if self.stopped {
            return PolicyAction::AlreadyStopped;
        }
        let permitted = snapshot.is_fresh(now_ms) && snapshot.kill_switch == KillSwitch::Clear;
        self.snapshot = Some(snapshot);
        if permitted {
            PolicyAction::Continue
        } else {
            self.stop_once()
        }
    }

    pub fn tick(&mut self, now_ms: i64) -> PolicyAction {
        if self.snapshot.as_ref().is_some_and(|snapshot| {
            snapshot.is_fresh(now_ms) && snapshot.kill_switch == KillSwitch::Clear
        }) {
            PolicyAction::Continue
        } else {
            self.stop_once()
        }
    }

    pub fn transport_lost(&mut self) -> PolicyAction {
        self.snapshot = None;
        self.stop_once()
    }

    fn stop_once(&mut self) -> PolicyAction {
        if std::mem::replace(&mut self.stopped, true) {
            PolicyAction::AlreadyStopped
        } else {
            PolicyAction::StopAll
        }
    }
}
