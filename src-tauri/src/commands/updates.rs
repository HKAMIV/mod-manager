use crate::update_tracking::{self, ModOrigin, UpdateInfo};

/// Get all tracked mod origins (GB links) for a game.
#[tauri::command]
pub fn list_mod_origins(game_id: String) -> Vec<ModOrigin> {
    update_tracking::get_origins(&game_id)
}

/// Manually link a locally-installed mod to a GameBanana mod + file version.
/// Normally this happens automatically at install time via the download
/// pipeline, but this command allows linking mods that were installed
/// manually (dragged into the folder) to a GB page the user identified.
#[tauri::command]
pub fn link_mod_origin(
    game_id: String,
    mod_key: String,
    gb_mod_id: u32,
    installed_file_id: u64,
    installed_version: Option<String>,
) -> Result<(), String> {
    update_tracking::link_mod(&game_id, &mod_key, gb_mod_id, installed_file_id, installed_version)
}

/// Remove the GB origin link for a mod.
#[tauri::command]
pub fn unlink_mod_origin(game_id: String, mod_key: String) -> Result<(), String> {
    update_tracking::unlink_mod(&game_id, &mod_key)
}

/// Check GameBanana for newer versions of all tracked mods for a game.
/// Returns only mods that actually have an update available (newer
/// non-archived file with a different file_id than what's installed).
#[tauri::command]
pub async fn check_for_updates(game_id: String) -> Result<Vec<UpdateInfo>, String> {
    update_tracking::check_for_updates(&game_id).await
}
