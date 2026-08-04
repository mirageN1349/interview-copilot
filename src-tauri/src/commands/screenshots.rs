use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::Manager;

use crate::{
    commands::{authorize_window, profiles::ProfileCommandState},
    error::CommandError,
    macos::screenshot::{
        LogicalRect, NativeScreenshotError, ScreenshotScanDecision, capture_display_png,
        scan_screenshot,
    },
    state::AppState,
    storage::StorageKey,
};

const RETENTION_MS: i64 = 30 * 24 * 60 * 60 * 1_000;
static SCREENSHOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureScreenshotInput {
    pub meeting_id: String,
    pub display_id: u32,
    pub area: Option<LogicalRect>,
    pub chat_thread: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotArtifactSummary {
    pub id: String,
    pub meeting_id: String,
    pub content_status: &'static str,
    pub mime_type: &'static str,
    pub byte_length: usize,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub shows_cursor: bool,
    pub excludes_current_application: bool,
}

#[tauri::command]
pub async fn capture_screenshot(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    input: CaptureScreenshotInput,
) -> Result<ScreenshotArtifactSummary, CommandError> {
    authorize_window(window.label(), &["main", "overlay"])?;
    if !matches!(input.chat_thread.as_str(), "live" | "side") {
        return Err(CommandError::new(
            "INVALID_CHAT_THREAD",
            "Screenshot chat thread is invalid",
        ));
    }
    let owner = {
        let runtime = app_state.0.lock().map_err(|_| state_error())?;
        if runtime.active_meeting_id.as_deref() != Some(&input.meeting_id) {
            return Err(CommandError::new(
                "MEETING_NOT_RUNNING",
                "The meeting capture is not active",
            ));
        }
        runtime
            .active_user_id
            .clone()
            .ok_or_else(|| CommandError::new("AUTH_REQUIRED", "Sign in is required"))?
    };
    let bundle_identifier = window.app_handle().config().identifier.clone();
    let display_id = input.display_id;
    let area = input.area;
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        capture_display_png(display_id, area, &bundle_identifier)
    })
    .await
    .map_err(|_| CommandError::new("SCREENSHOT_FAILED", "Screenshot capture failed"))?
    .map_err(native_error)?;
    let (pixel_width, pixel_height) = png_dimensions(&bytes).ok_or_else(|| {
        CommandError::new("SCREENSHOT_REJECTED", "Screenshot encoding is invalid")
    })?;

    let now = now_ms();
    let sequence = SCREENSHOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let artifact_id = format!("screenshot-{now}-{sequence}");
    let storage_key = StorageKey::parse(format!("screenshots/{artifact_id}.png"))
        .map_err(|_| CommandError::new("SCREENSHOT_STORE_FAILED", "Screenshot storage failed"))?;
    profile_state
        .files
        .write(&storage_key, &bytes)
        .map_err(|_| CommandError::new("SCREENSHOT_STORE_FAILED", "Screenshot storage failed"))?;
    let checksum = format!("{:x}", Sha256::digest(&bytes));
    let insert_result = profile_state
        .database
        .lock()
        .map_err(|_| state_error())?
        .connection()
        .execute(
            "INSERT INTO artifacts(id, meeting_id, kind, storage_key, mime_type, byte_length, \
             checksum, content_status, created_at_ms, expires_at_ms) \
             SELECT ?1, m.id, 'screenshot', ?2, 'image/png', ?3, ?4, 'pending', ?5, ?6 \
             FROM meetings m \
             JOIN launch_policies lp ON lp.id = m.launch_policy_id \
             JOIN ai_profiles p ON p.id = m.profile_id \
             WHERE m.id = ?7 AND lp.owner_user_id = ?8 AND p.owner_user_id = ?8 \
             AND m.status = 'running'",
            rusqlite::params![
                artifact_id,
                storage_key.as_str(),
                bytes.len() as i64,
                checksum,
                now,
                now + RETENTION_MS,
                input.meeting_id,
                owner,
            ],
        );
    if !matches!(insert_result, Ok(1)) {
        let _ = profile_state.files.delete(&storage_key);
        return Err(CommandError::new(
            "SCREENSHOT_STORE_FAILED",
            "Screenshot metadata could not be stored",
        ));
    }

    let (content_status, final_bytes) = match scan_screenshot(&bytes) {
        ScreenshotScanDecision::Allow => {
            let updated = profile_state
                .database
                .lock()
                .map_err(|_| state_error())?
                .connection()
                .execute(
                    "UPDATE artifacts SET content_status = 'allowed' \
                     WHERE id = ?1 AND content_status = 'pending'",
                    [&artifact_id],
                )
                .map_err(|_| state_error())?;
            if updated != 1 {
                let _ = profile_state.files.delete(&storage_key);
                return Err(state_error());
            }
            ("allowed", bytes)
        }
        ScreenshotScanDecision::Redact {
            bytes: redacted,
            reason: _,
        } => {
            profile_state
                .files
                .write(&storage_key, &redacted)
                .map_err(|_| {
                    CommandError::new("SCREENSHOT_STORE_FAILED", "Screenshot storage failed")
                })?;
            let checksum = format!("{:x}", Sha256::digest(&redacted));
            let updated = profile_state
                .database
                .lock()
                .map_err(|_| state_error())?
                .connection()
                .execute(
                    "UPDATE artifacts SET content_status = 'redacted', byte_length = ?2, checksum = ?3 \
                     WHERE id = ?1 AND content_status = 'pending'",
                    rusqlite::params![artifact_id, redacted.len() as i64, checksum],
                )
                .map_err(|_| state_error())?;
            if updated != 1 {
                let _ = profile_state.files.delete(&storage_key);
                return Err(state_error());
            }
            ("redacted", redacted)
        }
        ScreenshotScanDecision::Reject { reason } => {
            let _ = profile_state
                .database
                .lock()
                .map_err(|_| state_error())?
                .connection()
                .execute(
                    "UPDATE artifacts SET content_status = 'rejected' \
                     WHERE id = ?1 AND content_status = 'pending'",
                    [&artifact_id],
                );
            let _ = profile_state.files.delete(&storage_key);
            return Err(CommandError::new("SCREENSHOT_REJECTED", reason));
        }
    };

    Ok(ScreenshotArtifactSummary {
        id: artifact_id,
        meeting_id: input.meeting_id,
        content_status,
        mime_type: "image/png",
        byte_length: final_bytes.len(),
        pixel_width,
        pixel_height,
        shows_cursor: false,
        excludes_current_application: true,
    })
}

fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    (width > 0 && height > 0).then_some((width, height))
}

fn native_error(error: NativeScreenshotError) -> CommandError {
    match error {
        NativeScreenshotError::DisplayNotFound => CommandError::new(
            "DISPLAY_NOT_FOUND",
            "The selected display is no longer available",
        ),
        NativeScreenshotError::AreaOutOfBounds => CommandError::new(
            "AREA_OUT_OF_BOUNDS",
            "The screenshot area is outside the selected display",
        ),
        NativeScreenshotError::PermissionDenied => CommandError::new(
            "PERMISSION_REQUIRED",
            "Screen recording permission is required",
        ),
        NativeScreenshotError::TimedOut => {
            CommandError::new("SCREENSHOT_TIMEOUT", "Screenshot capture timed out").retryable(None)
        }
        NativeScreenshotError::CaptureFailed => {
            CommandError::new("SCREENSHOT_FAILED", "Screenshot capture failed").retryable(None)
        }
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn state_error() -> CommandError {
    CommandError::new("STATE_UNAVAILABLE", "Screenshot state is unavailable").retryable(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_png_ihdr_dimensions() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend_from_slice(&640_u32.to_be_bytes());
        png.extend_from_slice(&360_u32.to_be_bytes());
        assert_eq!(png_dimensions(&png), Some((640, 360)));
    }

    #[test]
    fn summary_uses_frontend_artifact_shape() {
        let value = serde_json::to_value(ScreenshotArtifactSummary {
            id: "shot-1".into(),
            meeting_id: "meeting-1".into(),
            content_status: "allowed",
            mime_type: "image/png",
            byte_length: 10,
            pixel_width: 2,
            pixel_height: 3,
            shows_cursor: false,
            excludes_current_application: true,
        })
        .unwrap();
        assert_eq!(value["id"], "shot-1");
        assert_eq!(value["contentStatus"], "allowed");
        assert!(value.get("artifactId").is_none());
    }
}
