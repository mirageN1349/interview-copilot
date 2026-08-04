pub mod commands;
pub mod error;
pub mod events;
pub mod macos;
pub mod security;
pub mod state;
pub mod storage;

use std::sync::Mutex;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(state::AppState::default())
        .manage(commands::presentation::PresentationCommandState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::system_health,
            commands::permissions::permissions_status,
            commands::permissions::permissions_request,
            commands::permissions::permissions_open_settings,
            commands::auth::auth_session_sync,
            commands::auth::auth_session_clear,
            commands::capture::displays_list,
            commands::capture::capture_start,
            commands::capture::emergency_stop_all,
            commands::meetings::run_gate_evaluate,
            commands::meetings::meeting_start,
            commands::meetings::meeting_get,
            commands::meetings::meeting_stop,
            commands::meetings::meeting_context_reset,
            commands::overlay::overlay_open,
            commands::overlay::overlay_ready,
            commands::overlay::overlay_show,
            commands::overlay::overlay_hide,
            commands::overlay::overlay_move,
            commands::overlay::overlay_set_interactive,
            commands::overlay::hotkeys_register,
            commands::overlay::hotkeys_unregister_all,
            commands::profiles::profile_list,
            commands::profiles::profile_get,
            commands::profiles::profile_save,
            commands::profiles::profile_archive,
            commands::profiles::profile_restore,
            commands::profiles::profile_source_import,
            commands::screenshots::capture_screenshot,
            commands::history::meeting_search,
            commands::history::meeting_history_get,
            commands::history::meeting_delete_content,
            commands::history::meeting_export,
            commands::history::retention_run,
            commands::history::audit_verify_chain,
            commands::presentation::presentation_profiles_list,
            commands::presentation::presentation_profile_apply,
            commands::emergency::emergency_stop_all_fail_closed,
            commands::overlay::overlay_set_capture_protection,
            commands::preferences::appearance_get,
            commands::preferences::appearance_save,
            commands::preferences::overlay_apply_material
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let app_data_root = app.path().app_data_dir()?;
            app.manage(commands::profiles::ProfileCommandState {
                database: Mutex::new(storage::Database::open(
                    app_data_root.join("interview-copilot.sqlite3"),
                )?),
                files: storage::AppDataFiles::new(app_data_root.join("files"))?,
                fixture_root: app.path().resource_dir()?.join("fixtures/profile-sources"),
            });
            if let Some(overlay) = app.get_webview_window("overlay") {
                macos::windowing::configure_overlay(&overlay)?;
                macos::glass::apply_material(&overlay, false)?;
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build Interview Copilot");
    app.run(|app, event| {
        if matches!(
            event,
            tauri::RunEvent::Ready | tauri::RunEvent::Reopen { .. }
        ) {
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.show();
                let _ = main.set_focus();
            }
        }
    });
}
