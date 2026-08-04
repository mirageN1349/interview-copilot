use serde::{Deserialize, Serialize};
const MAX_AUDIO_FRAGMENT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactStatus {
    Pending,
    Allowed,
    Redacted,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactDecision {
    Allow,
    Redact,
    Reject,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactRecord {
    pub id: String,
    pub meeting_id: String,
    pub sequence: u64,
    pub storage_key: String,
    pub byte_length: u64,
    pub checksum: String,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub status: ArtifactStatus,
}

impl ArtifactRecord {
    pub fn vet(mut self, decision: ArtifactDecision) -> Result<Self, &'static str> {
        if self.status != ArtifactStatus::Pending {
            return Err("Only pending artifacts can be vetted");
        }
        self.status = match decision {
            ArtifactDecision::Allow => ArtifactStatus::Allowed,
            ArtifactDecision::Redact => ArtifactStatus::Redacted,
            ArtifactDecision::Reject => ArtifactStatus::Rejected,
        };
        Ok(self)
    }

    pub fn is_attachable(&self) -> bool {
        matches!(
            self.status,
            ArtifactStatus::Allowed | ArtifactStatus::Redacted
        )
    }

    pub fn vet_audio_bytes(self, byte_length: usize) -> Result<Self, &'static str> {
        self.vet(
            if byte_length == 0 || byte_length > MAX_AUDIO_FRAGMENT_BYTES {
                ArtifactDecision::Reject
            } else {
                ArtifactDecision::Allow
            },
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordingFragment {
    id: String,
    meeting_id: String,
    sequence: u64,
    started_at_ms: i64,
}

impl RecordingFragment {
    pub fn meeting_id(&self) -> &str {
        &self.meeting_id
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn open(
        id: impl Into<String>,
        meeting_id: impl Into<String>,
        sequence: u64,
        started_at_ms: i64,
    ) -> Result<Self, &'static str> {
        let id = id.into();
        let meeting_id = meeting_id.into();
        if id.is_empty() || meeting_id.is_empty() {
            return Err("Fragment and meeting identifiers are required");
        }
        Ok(Self {
            id,
            meeting_id,
            sequence,
            started_at_ms,
        })
    }

    pub fn complete(
        self,
        ended_at_ms: i64,
        storage_key: impl Into<String>,
        byte_length: u64,
        checksum: impl Into<String>,
    ) -> Result<ArtifactRecord, &'static str> {
        let storage_key = storage_key.into();
        let checksum = checksum.into();
        if ended_at_ms < self.started_at_ms {
            return Err("Fragment end precedes its start");
        }
        if storage_key.is_empty() || checksum.is_empty() {
            return Err("Stored fragments require a key and checksum");
        }
        Ok(ArtifactRecord {
            id: self.id,
            meeting_id: self.meeting_id,
            sequence: self.sequence,
            storage_key,
            byte_length,
            checksum,
            started_at_ms: self.started_at_ms,
            ended_at_ms,
            status: ArtifactStatus::Pending,
        })
    }
}
