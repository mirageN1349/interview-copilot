use std::collections::HashMap;

use serde::Serialize;
use tauri::Emitter;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::error::CommandError;

const ACTIONS: &[&str] = &[
    "overlay.toggle",
    "overlay.interactive",
    "chat.live.focus",
    "chat.side.focus",
    "context.reset",
    "capture.full",
    "capture.area",
    "meeting.stop",
    "meeting.emergency-stop",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyRegistration {
    pub action_id: String,
    pub accelerator: String,
    pub registered: bool,
    pub conflict_with: Option<String>,
}

pub fn register(
    app: &tauri::AppHandle,
    bindings: HashMap<String, String>,
) -> Result<Vec<HotkeyRegistration>, CommandError> {
    let manager = app.global_shortcut();
    let mut result = Vec::with_capacity(bindings.len());
    for (action_id, accelerator) in bindings {
        if !ACTIONS.contains(&action_id.as_str()) || accelerator.len() > 80 {
            result.push(HotkeyRegistration {
                action_id,
                accelerator,
                registered: false,
                conflict_with: Some("invalid_binding".to_owned()),
            });
            continue;
        }
        let emitted_action = action_id.clone();
        match manager.on_shortcut(accelerator.as_str(), move |app, _, event| {
            if event.state == ShortcutState::Pressed {
                let _ = app.emit(
                    "hotkey://action",
                    serde_json::json!({ "actionId": emitted_action }),
                );
            }
        }) {
            Ok(()) => result.push(HotkeyRegistration {
                action_id,
                accelerator,
                registered: true,
                conflict_with: None,
            }),
            Err(_) => result.push(HotkeyRegistration {
                action_id,
                accelerator,
                registered: false,
                conflict_with: Some("system_or_application".to_owned()),
            }),
        }
    }
    Ok(result)
}

pub fn unregister_all(app: &tauri::AppHandle) -> Result<(), CommandError> {
    app.global_shortcut().unregister_all().map_err(|_| {
        CommandError::new(
            "HOTKEY_UNREGISTER_FAILED",
            "Keyboard shortcuts could not be cleared",
        )
    })
}
