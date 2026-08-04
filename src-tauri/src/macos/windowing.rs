use crate::error::CommandError;

pub fn configure_overlay(window: &tauri::WebviewWindow) -> Result<(), CommandError> {
    window.set_ignore_cursor_events(false)?;
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSColor, NSWindow};

        window.set_visible_on_all_workspaces(true)?;
        let native = window.ns_window()?;
        let native = unsafe { &*native.cast::<NSWindow>() };
        native.setOpaque(false);
        native.setBackgroundColor(Some(&NSColor::clearColor()));
        if let Some(content_view) = native.contentView() {
            content_view.setWantsLayer(true);
            if let Some(layer) = content_view.layer() {
                layer.setCornerRadius(22.0);
                layer.setMasksToBounds(true);
            }
        }
    }
    Ok(())
}
