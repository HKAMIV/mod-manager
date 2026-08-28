use super::mods::resolve_mod_path;
use crate::restore_points::{self, RestorePoint, RestoreResult};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn list_restore_points(game_id: String) -> Result<Vec<RestorePoint>, String> {
    restore_points::list_restore_points(&game_id)
}

#[tauri::command]
pub fn create_restore_point(
    state: State<AppState>,
    game_id: String,
    name: String,
    with_file_backup: bool,
) -> Result<RestorePoint, String> {
    let mod_path = resolve_mod_path(&state, &game_id)?;
    restore_points::create_restore_point(&game_id, &mod_path, &name, with_file_backup)
}

#[tauri::command]
pub fn delete_restore_point(game_id: String, restore_point_id: String) -> Result<(), String> {
    restore_points::delete_restore_point(&game_id, &restore_point_id)
}

#[tauri::command]
pub fn restore_from_point(
    state: State<AppState>,
    game_id: String,
    restore_point_id: String,
) -> Result<RestoreResult, String> {
    let mod_path = resolve_mod_path(&state, &game_id)?;
    restore_points::restore(&game_id, &mod_path, &restore_point_id)
}
