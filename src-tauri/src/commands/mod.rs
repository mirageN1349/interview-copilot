use serde::Serialize;

use crate::error::CommandError;

pub mod auth;
pub mod capture;
pub mod emergency;
pub mod history;
pub mod meetings;
pub mod overlay;
pub mod permissions;
pub mod preferences;
pub mod presentation;
pub mod profiles;
pub mod screenshots;

pub fn authorize_window(label: &str, allowed: &[&str]) -> Result<(), CommandError> {
    if allowed.contains(&label) {
        Ok(())
    } else {
        Err(CommandError::new(
            "WINDOW_CAPABILITY_DENIED",
            "This window cannot perform that action",
        ))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthStatus {
    status: &'static str,
}

#[tauri::command]
pub fn system_health(window: tauri::Window) -> Result<HealthStatus, CommandError> {
    authorize_window(window.label(), &["main"])?;
    Ok(HealthStatus { status: "ready" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_authorization_fails_closed() {
        assert!(authorize_window("main", &["main"]).is_ok());
        assert_eq!(
            authorize_window("overlay", &["main"]).unwrap_err().code,
            "WINDOW_CAPABILITY_DENIED"
        );
    }
}
