use crate::error::CommandError;

pub fn configure_overlay(window: &tauri::WebviewWindow) -> Result<(), CommandError> {
    window.set_ignore_cursor_events(false)?;
    #[cfg(target_os = "macos")]
    {
        window.set_visible_on_all_workspaces(true)?;
        clip_overlay_corners(window)?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn clip_overlay_corners(window: &tauri::WebviewWindow) -> Result<(), CommandError> {
    use objc2_app_kit::{NSColor, NSView, NSWindow};
    use objc2_quartz_core::kCACornerCurveContinuous;

    fn clip(view: &NSView) {
        view.setWantsLayer(true);
        if let Some(layer) = view.layer() {
            layer.setCornerRadius(22.0);
            layer.setCornerCurve(unsafe { kCACornerCurveContinuous });
            layer.setMasksToBounds(true);
        }
    }

    let native = window.ns_window()?;
    let native = unsafe { &*native.cast::<NSWindow>() };
    native.setOpaque(false);
    native.setHasShadow(false);
    native.setBackgroundColor(Some(&NSColor::clearColor()));
    if let Some(content_view) = native.contentView() {
        clip(&content_view);
    }
    window.with_webview(|webview| unsafe {
        clip(&*webview.inner().cast::<NSView>());
    })?;
    Ok(())
}
