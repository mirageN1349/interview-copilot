use serde::Serialize;

use crate::error::CommandError;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialKind {
    LiquidGlass,
    Vibrancy,
    Opaque,
}

#[cfg(target_os = "macos")]
pub fn apply_material(
    window: &tauri::WebviewWindow,
    reduce_transparency: bool,
) -> Result<MaterialKind, CommandError> {
    use window_vibrancy::{
        LiquidGlassOptions, NSGlassEffectViewStyle, NSVisualEffectMaterial, NSVisualEffectState,
        apply_liquid_glass, apply_vibrancy, clear_liquid_glass, clear_vibrancy,
    };

    if reduce_transparency {
        let _ = clear_liquid_glass(window);
        let _ = clear_vibrancy(window);
        if window.label() == "overlay" {
            super::windowing::clip_overlay_corners(window)?;
        }
        return Ok(MaterialKind::Opaque);
    }

    let is_overlay = window.label() == "overlay";
    let liquid_glass = if is_overlay {
        LiquidGlassOptions::new(NSGlassEffectViewStyle::Regular)
            .tint_color((58, 64, 74, 72))
            .radius(22.0)
    } else {
        LiquidGlassOptions::new(NSGlassEffectViewStyle::Clear).radius(22.0)
    };
    if apply_liquid_glass(window, liquid_glass).is_ok() {
        if is_overlay {
            super::windowing::clip_overlay_corners(window)?;
        }
        return Ok(MaterialKind::LiquidGlass);
    }

    apply_vibrancy(
        window,
        if is_overlay {
            NSVisualEffectMaterial::HudWindow
        } else {
            NSVisualEffectMaterial::Popover
        },
        Some(NSVisualEffectState::Active),
        Some(22.0),
    )
    .map_err(|_| CommandError::new("MATERIAL_UNAVAILABLE", "Window material is unavailable"))?;
    if window.label() == "overlay" {
        super::windowing::clip_overlay_corners(window)?;
    }
    Ok(MaterialKind::Vibrancy)
}

#[cfg(not(target_os = "macos"))]
pub fn apply_material(
    _window: &tauri::WebviewWindow,
    _reduce_transparency: bool,
) -> Result<MaterialKind, CommandError> {
    Ok(MaterialKind::Opaque)
}
