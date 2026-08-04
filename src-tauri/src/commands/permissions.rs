use crate::{
    commands::authorize_window,
    error::CommandError,
    macos::permissions::{self, PermissionKind, PermissionSnapshot},
};

#[tauri::command]
pub fn permissions_status(window: tauri::Window) -> Result<PermissionSnapshot, CommandError> {
    authorize_window(window.label(), &["main"])?;
    permissions::status()
}

#[tauri::command]
pub fn permissions_request(
    window: tauri::Window,
    kind: PermissionKind,
) -> Result<PermissionSnapshot, CommandError> {
    authorize_window(window.label(), &["main"])?;
    permissions::request(kind)
}

#[tauri::command]
pub fn permissions_open_settings(
    window: tauri::Window,
    kind: PermissionKind,
) -> Result<(), CommandError> {
    authorize_window(window.label(), &["main"])?;
    permissions::open_settings(kind)
}
