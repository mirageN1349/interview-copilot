use serde::Serialize;
use sha2::{Digest, Sha256};

// Set only after an original asset passes the internal approval workflow.
pub const APPROVED_GENERIC_ASSET_SHA256: Option<&str> = None;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationMode {
    #[default]
    Standard,
    Generic,
}

#[derive(Clone, Debug, Default)]
pub struct PresentationState {
    mode: PresentationMode,
}

impl PresentationState {
    pub fn mode(&self) -> PresentationMode {
        self.mode
    }

    pub fn apply(&mut self, mode: PresentationMode) -> Result<(), &'static str> {
        self.mode = mode;
        Ok(())
    }

    pub fn restore_standard(&mut self) -> bool {
        std::mem::replace(&mut self.mode, PresentationMode::Standard) != PresentationMode::Standard
    }
}

pub fn generic_asset_approved(path: &std::path::Path) -> bool {
    let Some(expected) = APPROVED_GENERIC_ASSET_SHA256 else {
        return false;
    };
    std::fs::read(path)
        .ok()
        .map(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)))
        .is_some_and(|actual| actual == expected)
}

#[cfg(target_os = "macos")]
pub fn apply_activation_policy(
    app: &tauri::AppHandle,
    mode: PresentationMode,
) -> tauri::Result<()> {
    app.set_activation_policy(match mode {
        PresentationMode::Standard => tauri::ActivationPolicy::Regular,
        PresentationMode::Generic => tauri::ActivationPolicy::Accessory,
    })
}

#[cfg(not(target_os = "macos"))]
pub fn apply_activation_policy(
    _app: &tauri::AppHandle,
    _mode: PresentationMode,
) -> tauri::Result<()> {
    Ok(())
}
