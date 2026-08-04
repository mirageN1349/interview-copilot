use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::CommandError;

static SCREEN_PERMISSION_REQUESTED: AtomicBool = AtomicBool::new(false);
static ACCESSIBILITY_PERMISSION_REQUESTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionKind {
    Screen,
    Microphone,
    Accessibility,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    NotDetermined,
    Granted,
    Denied,
    Restricted,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionSnapshot {
    pub screen_recording: PermissionState,
    pub microphone: PermissionState,
    pub accessibility: PermissionState,
    pub observed_at: String,
    pub restart_may_be_required: bool,
}

pub fn microphone_state_from_raw(status: isize) -> PermissionState {
    match status {
        0 => PermissionState::NotDetermined,
        1 => PermissionState::Restricted,
        2 => PermissionState::Denied,
        3 => PermissionState::Granted,
        _ => PermissionState::Restricted,
    }
}

pub fn promptable_permission_state(granted: bool, requested: bool) -> PermissionState {
    if granted {
        PermissionState::Granted
    } else if requested {
        PermissionState::Denied
    } else {
        PermissionState::NotDetermined
    }
}

pub fn settings_url(kind: PermissionKind) -> &'static str {
    match kind {
        PermissionKind::Screen => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
        }
        PermissionKind::Microphone => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
        }
        PermissionKind::Accessibility => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        }
    }
}

fn observed_at() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

#[cfg(target_os = "macos")]
pub fn status() -> Result<PermissionSnapshot, CommandError> {
    use objc2_application_services::AXIsProcessTrusted;
    use objc2_av_foundation::{AVCaptureDevice, AVMediaTypeAudio};
    use objc2_core_graphics::CGPreflightScreenCaptureAccess;

    let microphone = unsafe {
        AVMediaTypeAudio
            .map(|media_type| AVCaptureDevice::authorizationStatusForMediaType(media_type))
            .map(|value| microphone_state_from_raw(value.0))
            .unwrap_or(PermissionState::Restricted)
    };
    let screen_recording = promptable_permission_state(
        CGPreflightScreenCaptureAccess(),
        SCREEN_PERMISSION_REQUESTED.load(Ordering::Relaxed),
    );
    let accessibility = promptable_permission_state(
        unsafe { AXIsProcessTrusted() },
        ACCESSIBILITY_PERMISSION_REQUESTED.load(Ordering::Relaxed),
    );

    Ok(PermissionSnapshot {
        screen_recording,
        microphone,
        accessibility,
        observed_at: observed_at(),
        restart_may_be_required: screen_recording != PermissionState::Granted
            || accessibility != PermissionState::Granted,
    })
}

#[cfg(not(target_os = "macos"))]
pub fn status() -> Result<PermissionSnapshot, CommandError> {
    Err(CommandError::new(
        "PERMISSION_API_UNAVAILABLE",
        "System permissions are available only on macOS",
    ))
}

#[cfg(target_os = "macos")]
pub fn request(kind: PermissionKind) -> Result<PermissionSnapshot, CommandError> {
    match kind {
        PermissionKind::Screen => {
            SCREEN_PERMISSION_REQUESTED.store(true, Ordering::Relaxed);
            objc2_core_graphics::CGRequestScreenCaptureAccess();
        }
        PermissionKind::Microphone => unsafe {
            use objc2_av_foundation::{AVCaptureDevice, AVMediaTypeAudio};

            let media_type = AVMediaTypeAudio.ok_or_else(|| {
                CommandError::new(
                    "PERMISSION_API_UNAVAILABLE",
                    "Microphone permission API is unavailable",
                )
            })?;
            let completion = block2::RcBlock::new(|_| {});
            AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type, &completion);
        },
        PermissionKind::Accessibility => unsafe {
            use objc2_application_services::{
                AXIsProcessTrustedWithOptions, kAXTrustedCheckOptionPrompt,
            };
            use objc2_core_foundation::{CFBoolean, CFDictionary};

            ACCESSIBILITY_PERMISSION_REQUESTED.store(true, Ordering::Relaxed);
            let options =
                CFDictionary::from_slices(&[kAXTrustedCheckOptionPrompt], &[CFBoolean::new(true)]);
            AXIsProcessTrustedWithOptions(Some(options.as_opaque()));
        },
    }

    status()
}

#[cfg(not(target_os = "macos"))]
pub fn request(_kind: PermissionKind) -> Result<PermissionSnapshot, CommandError> {
    status()
}

#[cfg(target_os = "macos")]
pub fn open_settings(kind: PermissionKind) -> Result<(), CommandError> {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSString, NSURL};

    let url = NSURL::URLWithString(&NSString::from_str(settings_url(kind))).ok_or_else(|| {
        CommandError::new(
            "PERMISSION_SETTINGS_OPEN_FAILED",
            "Permission settings could not be opened",
        )
    })?;
    if NSWorkspace::sharedWorkspace().openURL(&url) {
        Ok(())
    } else {
        Err(CommandError::new(
            "PERMISSION_SETTINGS_OPEN_FAILED",
            "Permission settings could not be opened",
        ))
    }
}

#[cfg(not(target_os = "macos"))]
pub fn open_settings(_kind: PermissionKind) -> Result<(), CommandError> {
    Err(CommandError::new(
        "PERMISSION_API_UNAVAILABLE",
        "System permissions are available only on macOS",
    ))
}
