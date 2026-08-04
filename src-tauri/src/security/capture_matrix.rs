use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureEnvironment {
    pub macos_version: String,
    pub capture_client: String,
    pub capture_client_version: String,
    pub share_mode: String,
    pub monitor_topology: String,
    pub app_build_checksum: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceResult {
    Approved,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureMatrixRow {
    pub id: String,
    pub environment: CaptureEnvironment,
    pub evidence_checksum: String,
    pub recorded_at_ms: i64,
    pub signature_verified: bool,
    pub result: EvidenceResult,
}

impl CaptureMatrixRow {
    pub fn is_approval_eligible(&self) -> bool {
        !self.id.trim().is_empty()
            && self.recorded_at_ms > 0
            && self.signature_verified
            && self.result == EvidenceResult::Approved
            && valid_checksum(&self.evidence_checksum)
            && valid_checksum(&self.environment.app_build_checksum)
            && [
                &self.environment.macos_version,
                &self.environment.capture_client,
                &self.environment.capture_client_version,
                &self.environment.share_mode,
                &self.environment.monitor_topology,
            ]
            .iter()
            .all(|value| !value.trim().is_empty())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuaranteeLevel {
    BestEffort,
}

impl GuaranteeLevel {
    pub const fn as_str(self) -> &'static str {
        "best_effort"
    }
}

#[derive(Clone, Debug, Default)]
pub struct CaptureMatrix {
    rows: HashMap<String, CaptureMatrixRow>,
}

impl CaptureMatrix {
    pub fn from_rows(
        rows: impl IntoIterator<Item = CaptureMatrixRow>,
    ) -> Result<Self, &'static str> {
        let mut matrix = Self::default();
        for row in rows {
            if matrix.rows.insert(row.id.clone(), row).is_some() {
                return Err("CAPTURE_MATRIX_ROW_IMMUTABLE");
            }
        }
        Ok(matrix)
    }

    pub fn exact_approved(
        &self,
        row_id: &str,
        environment: &CaptureEnvironment,
    ) -> Option<&CaptureMatrixRow> {
        self.rows
            .get(row_id)
            .filter(|row| row.environment == *environment && row.is_approval_eligible())
    }
}

fn valid_checksum(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}
