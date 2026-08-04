use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::storage::Database;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferences {
    pub user_id: String,
    pub theme: String,
    pub reduce_visual_effects_override: String,
    pub updated_at_ms: i64,
    pub revision: i64,
}

pub fn get(database: &Database, user_id: &str) -> rusqlite::Result<Option<AppPreferences>> {
    database
        .connection()
        .query_row(
            "SELECT user_id, theme, reduce_visual_effects_override, updated_at_ms, revision FROM app_preferences WHERE user_id = ?1",
            [user_id],
            |row| Ok(AppPreferences {
                user_id: row.get(0)?,
                theme: row.get(1)?,
                reduce_visual_effects_override: row.get(2)?,
                updated_at_ms: row.get(3)?,
                revision: row.get(4)?,
            }),
        )
        .optional()
}

pub fn save_theme(
    database: &Database,
    user_id: &str,
    theme: &str,
    reduce_visual_effects_override: &str,
    now_ms: i64,
) -> rusqlite::Result<AppPreferences> {
    if !matches!(theme, "light" | "dark" | "auto") {
        return Err(rusqlite::Error::InvalidParameterName("theme".into()));
    }
    if !matches!(reduce_visual_effects_override, "system" | "reduce") {
        return Err(rusqlite::Error::InvalidParameterName(
            "reduceVisualEffectsOverride".into(),
        ));
    }
    database.connection().execute(
        "INSERT INTO app_preferences (user_id, theme, overlay_position_by_display_json, hotkeys_json, reduce_visual_effects_override, updated_at_ms, revision) \
         VALUES (?1, ?2, '{}', '{}', ?3, ?4, 1) \
         ON CONFLICT(user_id) DO UPDATE SET theme = excluded.theme, reduce_visual_effects_override = excluded.reduce_visual_effects_override, updated_at_ms = excluded.updated_at_ms, revision = app_preferences.revision + 1",
        params![user_id, theme, reduce_visual_effects_override, now_ms],
    )?;
    get(database, user_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_only_supported_themes_and_increments_revision() {
        let database = Database::in_memory().unwrap();
        database.connection().execute(
            "INSERT INTO users(id, email, display_name, roles_json, device_ids_json, status, last_authenticated_at_ms) VALUES ('user-1', 'user@example.test', 'User', '[]', '[]', 'active', 1)",
            [],
        ).unwrap();
        let first = save_theme(&database, "user-1", "dark", "system", 10).unwrap();
        let second = save_theme(&database, "user-1", "auto", "reduce", 20).unwrap();
        assert_eq!((first.theme.as_str(), first.revision), ("dark", 1));
        assert_eq!((second.theme.as_str(), second.revision), ("auto", 2));
        assert!(save_theme(&database, "user-1", "neon", "system", 30).is_err());
    }
}
