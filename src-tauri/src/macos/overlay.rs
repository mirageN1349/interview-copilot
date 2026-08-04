use tauri::{PhysicalPosition, Position};

use crate::error::CommandError;

use crate::security::capture_matrix::GuaranteeLevel;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureProtectionState {
    pub enabled: bool,
    pub guarantee: GuaranteeLevel,
    pub own_capture_excluded: bool,
    pub passive_pointer: bool,
    pub keyboard_area_selection: bool,
    pub third_party_cursor_controlled: bool,
}

pub fn set_capture_protection(
    window: &tauri::WebviewWindow,
    enabled: bool,
    exact_matrix_approved: bool,
) -> Result<CaptureProtectionState, CommandError> {
    if enabled && !exact_matrix_approved {
        return Err(CommandError::new(
            "ADVERSARIAL_MATRIX_UNSUPPORTED",
            "This presentation mode is unavailable",
        ));
    }
    window.set_content_protected(enabled)?;
    Ok(CaptureProtectionState {
        enabled,
        guarantee: GuaranteeLevel::BestEffort,
        own_capture_excluded: true,
        passive_pointer: true,
        keyboard_area_selection: true,
        third_party_cursor_controlled: false,
    })
}

pub fn open_for_meeting(
    window: &tauri::WebviewWindow,
    meeting_id: &str,
) -> Result<(), CommandError> {
    window.hide()?;
    let mut url = window.url()?;
    url.set_fragment(Some(&format!("/overlay/{meeting_id}")));
    window.navigate(url)?;
    window.set_ignore_cursor_events(false)?;
    Ok(())
}

pub fn show_ready(window: &tauri::WebviewWindow) -> Result<(), CommandError> {
    window.show()?;
    window.set_focus()?;
    Ok(())
}

pub fn set_interactive(
    window: &tauri::WebviewWindow,
    interactive: bool,
) -> Result<(), CommandError> {
    window.set_ignore_cursor_events(!interactive)?;
    if interactive {
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

pub fn move_by(
    window: &tauri::WebviewWindow,
    dx: i32,
    dy: i32,
) -> Result<OverlayPosition, CommandError> {
    let position = window.outer_position()?;
    let size = window.outer_size()?;
    let monitors = window.available_monitors()?;
    let monitor = monitors
        .iter()
        .find(|monitor| {
            let origin = monitor.position();
            let size = monitor.size();
            position.x >= origin.x
                && position.x < origin.x + size.width as i32
                && position.y >= origin.y
                && position.y < origin.y + size.height as i32
        })
        .or_else(|| monitors.first())
        .ok_or_else(|| CommandError::new("DISPLAY_LIST_UNAVAILABLE", "No display is available"))?;
    let origin = monitor.position();
    let monitor_size = monitor.size();
    let recovery = 48_i32;
    let x = (position.x + dx).clamp(
        origin.x - size.width as i32 + recovery,
        origin.x + monitor_size.width as i32 - recovery,
    );
    let y = (position.y + dy).clamp(origin.y, origin.y + monitor_size.height as i32 - recovery);
    window.set_position(Position::Physical(PhysicalPosition::new(x, y)))?;
    Ok(OverlayPosition { x, y })
}
