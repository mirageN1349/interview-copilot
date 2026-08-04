use std::{path::Path, process::Command};

use sha2::{Digest, Sha256};

use crate::security::capture_matrix::{CaptureEnvironment, CaptureMatrixRow, EvidenceResult};

#[derive(Clone, Debug)]
pub struct ProbeRequest {
    pub row_id: String,
    pub capture_client: String,
    pub capture_client_version: String,
    pub share_mode: String,
    pub monitor_topology: String,
    pub reference_capture: Option<std::path::PathBuf>,
}

pub fn collect_blocked_evidence(request: ProbeRequest, now_ms: i64) -> CaptureMatrixRow {
    let executable = std::env::current_exe().ok();
    CaptureMatrixRow {
        id: request.row_id,
        environment: CaptureEnvironment {
            macos_version: macos_version().unwrap_or_else(|| "unknown".into()),
            capture_client: request.capture_client,
            capture_client_version: request.capture_client_version,
            share_mode: request.share_mode,
            monitor_topology: request.monitor_topology,
            app_build_checksum: executable
                .as_deref()
                .and_then(file_checksum)
                .unwrap_or_else(|| "unavailable".into()),
        },
        evidence_checksum: request
            .reference_capture
            .as_deref()
            .and_then(file_checksum)
            .unwrap_or_else(|| "unavailable".into()),
        recorded_at_ms: now_ms,
        signature_verified: executable.as_deref().is_some_and(code_signature_valid),
        result: EvidenceResult::Blocked,
    }
}

fn macos_version() -> Option<String> {
    Command::new("/usr/bin/sw_vers")
        .args(["-productVersion"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|version| version.trim().to_owned())
        .filter(|version| !version.is_empty())
}

pub fn runtime_environment_from(recorded: &CaptureEnvironment) -> Option<CaptureEnvironment> {
    let executable = std::env::current_exe().ok()?;
    if !code_signature_valid(&executable) {
        return None;
    }
    Some(CaptureEnvironment {
        macos_version: macos_version()?,
        app_build_checksum: file_checksum(&executable)?,
        ..recorded.clone()
    })
}

fn code_signature_valid(executable: &Path) -> bool {
    Command::new("/usr/bin/codesign")
        .args(["--verify", "--strict", "--verbose=2"])
        .arg(executable)
        .status()
        .is_ok_and(|status| status.success())
}

fn file_checksum(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let digest = Sha256::digest(bytes);
    Some(format!("sha256:{digest:x}"))
}
