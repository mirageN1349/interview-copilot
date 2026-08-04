use serde::Serialize;
use tauri::{Emitter, Runtime};

use crate::error::CommandError;

pub const PERMISSIONS_CHANGED: &str = "permissions://changed";
pub const MEETING_STATE: &str = "meeting://state";
pub const CAPTURE_STATE: &str = "capture://state";
pub const POLICY_STALE: &str = "policy://stale";

pub fn emit<R: Runtime, T: Serialize + Clone>(
    app: &tauri::AppHandle<R>,
    event: &str,
    payload: T,
) -> Result<(), CommandError> {
    app.emit(event, payload).map_err(CommandError::from)
}
