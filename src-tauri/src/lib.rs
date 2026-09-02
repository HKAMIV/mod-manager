mod commands;
mod downloads;
mod gamebanana;
mod hotkeys;
mod migration;
mod mods;
mod power_management;
mod presets;
mod restore_points;
mod state;
mod symlink;
mod update_tracking;
mod watcher;

use hotkeys::HotkeyRegistry;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(app_state)
        .manage(HotkeyRegistry::default())
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::get_game_config,
            commands::settings::set_game_path,
            commands::settings::export_config,
            commands::settings::import_config,
            commands::mods::scan_game_mods,
            commands::mods::toggle_mod,
            commands::mods::batch_toggle_mods,
            commands::mods::delete_game_mod,
            commands::mods::batch_delete_game_mods,
            commands::mods::get_symlink_layout_status,
            commands::mods::migrate_to_symlink_layout_cmd,
            commands::watcher::watch_mod_directory,
            commands::watcher::unwatch_mod_directory,
            commands::presets::list_presets,
            commands::presets::save_preset,
            commands::presets::delete_preset,
            commands::presets::rename_preset,
            commands::presets::apply_preset,
            commands::hotkeys::set_preset_hotkey,
            commands::hotkeys::clear_preset_hotkey,
            commands::restore_points::list_restore_points,
            commands::restore_points::create_restore_point,
            commands::restore_points::delete_restore_point,
            commands::restore_points::restore_from_point,
            commands::gamebanana::browse_gamebanana_mods,
            commands::gamebanana::get_gamebanana_mod_detail,
            commands::gamebanana::get_gamebanana_categories,
            commands::downloads::list_downloads,
            commands::downloads::queue_download,
            commands::downloads::cancel_download,
            commands::downloads::retry_download,
            commands::downloads::remove_download,
            commands::downloads::resolve_download_conflict,
            commands::updates::list_mod_origins,
            commands::updates::link_mod_origin,
            commands::updates::unlink_mod_origin,
            commands::updates::check_for_updates,
        ])
        .setup(|app| {
            power_management::disable_app_nap();
            watcher::init(app.handle().clone());
            hotkeys::init(app.handle());
            downloads::init(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
