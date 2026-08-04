use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MeetingMode {
    StandardLab,
    AdversarialLab,
}

#[derive(Debug, Clone)]
pub struct GateContext {
    pub session_bound: bool,
    pub user_active: bool,
    pub device_allowed: bool,
    pub environment_allowed: bool,
    pub launch_policy_valid: bool,
    pub policy_fresh: bool,
    pub kill_switch_clear: bool,
    pub consent_complete: bool,
    pub processing_boundary_approved: bool,
    pub mode: MeetingMode,
    pub adversarial_role: bool,
    pub adversarial_approved: bool,
    pub matrix_row_approved: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GateDecision {
    pub allowed: bool,
    pub reason_codes: Vec<&'static str>,
}

pub fn evaluate(context: &GateContext) -> GateDecision {
    let mut reasons = Vec::new();
    let checks = [
        (context.session_bound, "SESSION_MISSING"),
        (context.user_active, "USER_INACTIVE"),
        (context.device_allowed, "DEVICE_NOT_ALLOWED"),
        (context.environment_allowed, "ENVIRONMENT_NOT_ALLOWED"),
        (context.launch_policy_valid, "LAUNCH_POLICY_INVALID"),
        (context.policy_fresh, "POLICY_STALE"),
        (context.kill_switch_clear, "KILL_SWITCH_ACTIVE"),
        (context.consent_complete, "CONSENT_INCOMPLETE"),
        (
            context.processing_boundary_approved,
            "PROCESSING_BOUNDARY_UNAPPROVED",
        ),
    ];

    for (passed, code) in checks {
        if !passed {
            reasons.push(code);
        }
    }

    if context.mode == MeetingMode::AdversarialLab {
        if !context.adversarial_role {
            reasons.push("ADVERSARIAL_ROLE_REQUIRED");
        }
        if !context.adversarial_approved {
            reasons.push("ADVERSARIAL_APPROVAL_REQUIRED");
        }
        if !context.matrix_row_approved {
            reasons.push("ADVERSARIAL_MATRIX_UNSUPPORTED");
        }
    }

    GateDecision {
        allowed: reasons.is_empty(),
        reason_codes: reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> GateContext {
        GateContext {
            session_bound: true,
            user_active: true,
            device_allowed: true,
            environment_allowed: true,
            launch_policy_valid: true,
            policy_fresh: true,
            kill_switch_clear: true,
            consent_complete: true,
            processing_boundary_approved: true,
            mode: MeetingMode::StandardLab,
            adversarial_role: false,
            adversarial_approved: false,
            matrix_row_approved: false,
        }
    }

    #[test]
    fn every_required_check_fails_closed() {
        let mut context = valid();
        context.policy_fresh = false;
        context.consent_complete = false;
        let decision = evaluate(&context);
        assert!(!decision.allowed);
        assert_eq!(
            decision.reason_codes,
            ["POLICY_STALE", "CONSENT_INCOMPLETE"]
        );
    }

    #[test]
    fn adversarial_mode_requires_all_three_extra_checks() {
        let mut context = valid();
        context.mode = MeetingMode::AdversarialLab;
        assert_eq!(evaluate(&context).reason_codes.len(), 3);
        context.adversarial_role = true;
        context.adversarial_approved = true;
        context.matrix_row_approved = true;
        assert!(evaluate(&context).allowed);
    }
}
