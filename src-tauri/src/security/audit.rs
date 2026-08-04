use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub sequence: u64,
    pub occurred_at_ms: i64,
    pub action: String,
    pub outcome: String,
    pub reason_code: String,
    pub previous_hash: String,
    pub event_hash: String,
}

#[derive(Debug, Default)]
pub struct AuditChain {
    events: Vec<AuditEvent>,
}

impl AuditChain {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(
        &mut self,
        occurred_at_ms: i64,
        action: impl Into<String>,
        outcome: impl Into<String>,
        reason_code: impl Into<String>,
    ) -> &AuditEvent {
        let sequence = self.events.len() as u64 + 1;
        let previous_hash = self
            .events
            .last()
            .map(|event| event.event_hash.clone())
            .unwrap_or_else(|| "genesis".into());
        let action = action.into();
        let outcome = outcome.into();
        let reason_code = reason_code.into();
        let event_hash = hash_event(
            sequence,
            occurred_at_ms,
            &action,
            &outcome,
            &reason_code,
            &previous_hash,
        );
        self.events.push(AuditEvent {
            sequence,
            occurred_at_ms,
            action,
            outcome,
            reason_code,
            previous_hash,
            event_hash,
        });
        self.events.last().expect("event was just appended")
    }

    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    pub fn verify(&self) -> bool {
        let mut previous_hash = "genesis".to_owned();
        self.events.iter().all(|event| {
            let valid = event.previous_hash == previous_hash
                && event.event_hash
                    == hash_event(
                        event.sequence,
                        event.occurred_at_ms,
                        &event.action,
                        &event.outcome,
                        &event.reason_code,
                        &event.previous_hash,
                    );
            previous_hash = event.event_hash.clone();
            valid
        })
    }
}

fn hash_event(
    sequence: u64,
    occurred_at_ms: i64,
    action: &str,
    outcome: &str,
    reason_code: &str,
    previous_hash: &str,
) -> String {
    let mut hasher = Sha256::new();
    for part in [
        sequence.to_string(),
        occurred_at_ms.to_string(),
        action.to_owned(),
        outcome.to_owned(),
        reason_code.to_owned(),
        previous_hash.to_owned(),
    ] {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_and_verify_chain() {
        let mut chain = AuditChain::new();
        chain.append(1, "login", "succeeded", "OK");
        chain.append(2, "meeting_start", "denied", "POLICY_STALE");
        assert!(chain.verify());
        assert_eq!(
            chain.events()[1].previous_hash,
            chain.events()[0].event_hash
        );
    }

    #[test]
    fn mutation_is_detected() {
        let mut chain = AuditChain::new();
        chain.append(1, "login", "succeeded", "OK");
        chain.events[0].outcome = "failed".into();
        assert!(!chain.verify());
    }
}
