mod app;
mod backup;
mod files;
mod logging;
mod models;
mod save_monitor;
mod session;
mod settings;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            app::get_exe_dir,
            app::get_noita_save_path,
            app::get_entangled_worlds_config_path,
            app::get_entangled_worlds_save_path,
            app::get_app_settings_dir,
            app::open_workshop_item,
            settings::save_settings,
            settings::load_settings,
            settings::save_presets,
            settings::load_presets,
            files::read_mod_config,
            files::write_mod_config,
            app::is_dev_build,
            app::get_version,
            app::open_directory,
            files::check_file_modified,
            files::get_file_modified_time,
            logging::add_log_entry,
            logging::get_log_entries,
            logging::clear_log_buffer,
            logging::flush_log_buffer,
            files::write_file,
            files::read_file,
            files::check_file_exists,
            app::get_dev_save_dir,
            session::create_session_lock,
            session::remove_session_lock,
            session::check_session_lock,
            session::cache_and_overwrite_mod_config,
            session::revert_mod_config,
            session::check_mod_config_cache_exists,
            backup::create_backup,
            backup::list_backups,
            backup::delete_backup,
            backup::cleanup_old_backups,
            backup::get_backup_contents,
            backup::restore_backup,
            app::get_system_info,
            save_monitor::create_monitor_snapshot,
            save_monitor::list_monitor_snapshots,
            save_monitor::cleanup_monitor_snapshots,
            save_monitor::clear_monitor_data
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            match event {
                tauri::RunEvent::WindowEvent {
                    label,
                    event: tauri::WindowEvent::CloseRequested { .. },
                    ..
                } => {
                    if label == "main" {
                        // Close all other windows (e.g. detached log window)
                        // so the application can exit cleanly
                        for (name, window) in app.webview_windows() {
                            if name != "main" {
                                let _ = window.close();
                            }
                        }
                    }
                }
                tauri::RunEvent::Exit => {
                    session::revert_mod_config_internal();
                    let _ = logging::add_log_entry(
                        "INFO".to_string(),
                        "Application shutting down".to_string(),
                        "App".to_string(),
                    );
                    let _ = logging::flush_log_buffer_sync();
                    logging::write_session_end_marker();
                }
                _ => {}
            }
        });
}
