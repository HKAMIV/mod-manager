use crate::install::{
    install_mod_from_archive as install_archive,
    install_mod_from_folder as install_folder,
    InstallResult,
};
use crate::migration::{get_layout_status, migrate_to_symlink_layout, LayoutStatus, MigrationResult};
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

// ---------------------------------------------------------------------------
// Phase 12 — symlink layout commands
// ---------------------------------------------------------------------------

/// Query the layout status of a game's mod directory (cheap, read-only).
/// Returns whether the symlink layout is already active, and if not, how
/// many legacy mods would be migrated.
#[tauri::command]
pub fn get_symlink_layout_status(
    state: State<AppState>,
    game_id: String,
) -> Result<LayoutStatus, String> {
    let mod_path = resolve_mod_path(&state, &game_id)?;
    get_layout_status(&mod_path)
}

/// Convert a game's mod directory from the legacy DISABLED_-prefix layout to
/// the symlink layout. This is a one-time, user-initiated operation.
///
/// - `with_restore_point`: if true, a state-only restore point is created
///   before any files are moved so the user can recover if something goes wrong.
/// - Returns `MigrationResult` with per-mod outcomes and the restore point id.
///
/// Already-migrated games are a silent no-op (returns 0 migrated).
#[tauri::command]
pub fn migrate_to_symlink_layout_cmd(
    state: State<AppState>,
    game_id: String,
    with_restore_point: bool,
) -> Result<MigrationResult, String> {
    let mod_path = resolve_mod_path(&state, &game_id)?;
    migrate_to_symlink_layout(&game_id, &mod_path, with_restore_point)
}

// ---------------------------------------------------------------------------
// Manual mod installation
// ---------------------------------------------------------------------------

/// Install a mod by copying a folder the user chose from disk.
#[tauri::command]
pub fn install_mod_from_folder(
    state: State<AppState>,
    game_id: String,
    source_path: String,
    category: String,
    mod_name: String,
) -> Result<InstallResult, String> {
    let mod_path = resolve_mod_path(&state, &game_id)?;
    install_folder(&source_path, &mod_path, &category, &mod_name)
}

/// Install a mod by extracting an archive (.zip / .7z / .rar) the user chose.
#[tauri::command]
pub fn install_mod_from_archive(
    state: State<AppState>,
    game_id: String,
    archive_path: String,
    category: String,
    mod_name: String,
) -> Result<InstallResult, String> {
    let mod_path = resolve_mod_path(&state, &game_id)?;
    install_archive(&archive_path, &mod_path, &category, &mod_name)
}
