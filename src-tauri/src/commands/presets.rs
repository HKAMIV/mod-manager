use super::mods::resolve_mod_path;
use crate::presets::{self, ApplyPresetResult, Preset};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn list_presets(game_id: String) -> Result<Vec<Preset>, String> {
    presets::list_presets(&game_id)
}

#[tauri::command]
pub fn save_preset(state: State<AppState>, game_id: String, name: String) -> Result<Preset, String> {
    let mod_path = resolve_mod_path(&state, &game_id)?;
    presets::save_preset(&game_id, &mod_path, &name)
}

#[tauri::command]
pub fn delete_preset(game_id: String, preset_id: String) -> Result<(), String> {
    presets::delete_preset(&game_id, &preset_id)
}

#[tauri::command]
pub fn rename_preset(game_id: String, preset_id: String, new_name: String) -> Result<Preset, String> {
    presets::rename_preset(&game_id, &preset_id, &new_name)
}

#[tauri::command]
pub fn apply_preset(
    state: State<AppState>,
    game_id: String,
    preset_id: String,
) -> Result<ApplyPresetResult, String> {
    let mod_path = resolve_mod_path(&state, &game_id)?;
    presets::apply_preset(&game_id, &mod_path, &preset_id)
}
