use crate::mods::{batch_delete_mods, batch_set_enabled, delete_mod, scan_mods, set_mod_enabled, BatchDeleteResult, BatchToggleResult, ModInfo, ToggleTarget};
use crate::state::AppState;
use tauri::State;

/// Scan the configured mod directory for the given game and return the mods found.
/// Returns an error if the game is unknown or has no mod path configured yet.
#[tauri::command]
pub fn scan_game_mods(state: State<AppState>, game_id: String) -> Result<Vec<ModInfo>, String> {
    let mod_path = resolve_mod_path(&state, &game_id)?;
    scan_mods(&mod_path)
}

/// Look up the configured mod directory for `game_id`. Shared by every command
/// in this module that needs to resolve "which folder am I operating on".
pub fn resolve_mod_path(state: &State<AppState>, game_id: &str) -> Result<String, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    let game = settings
        .games
        .iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| format!("Game '{}' not found", game_id))?;
    game.mod_path
        .clone()
        .ok_or_else(|| "No mod directory configured for this game".to_string())
}

/// Enable or disable a single mod by its folder path.
#[tauri::command]
pub fn toggle_mod(path: String, category: String, enabled: bool) -> Result<ModInfo, String> {
    set_mod_enabled(&path, &category, enabled)
}

/// Enable or disable a batch of mods at once.
#[tauri::command]
pub fn batch_toggle_mods(
    targets: Vec<ToggleTarget>,
    enabled: bool,
) -> Result<BatchToggleResult, String> {
    Ok(batch_set_enabled(&targets, enabled))
}

/// Permanently delete a single mod folder.
#[tauri::command]
pub fn delete_game_mod(state: State<AppState>, game_id: String, path: String) -> Result<String, String> {
    let mod_path = resolve_mod_path(&state, &game_id)?;
    delete_mod(&path, &mod_path)
}

/// Permanently delete multiple mod folders at once.
#[tauri::command]
pub fn batch_delete_game_mods(state: State<AppState>, game_id: String, paths: Vec<String>) -> Result<BatchDeleteResult, String> {
    let mod_path = resolve_mod_path(&state, &game_id)?;
    Ok(batch_delete_mods(&paths, &mod_path))
}
