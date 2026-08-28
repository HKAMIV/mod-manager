use crate::state::{AppState, GameConfig, Settings};
use tauri::State;

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn save_settings(state: State<AppState>, settings: Settings) -> Result<(), String> {
    let mut current = state.settings.lock().map_err(|e| e.to_string())?;
    *current = settings.clone();
    AppState::save_settings(&settings)?;
    Ok(())
}

#[tauri::command]
pub fn get_game_config(state: State<AppState>, game_id: String) -> Result<GameConfig, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings
        .games
        .iter()
        .find(|g| g.id == game_id)
        .cloned()
        .ok_or_else(|| format!("Game '{}' not found", game_id))
}

#[tauri::command]
pub fn set_game_path(
    state: State<AppState>,
    game_id: String,
    path: String,
) -> Result<(), String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    let game = settings
        .games
        .iter_mut()
        .find(|g| g.id == game_id)
        .ok_or_else(|| format!("Game '{}' not found", game_id))?;
    game.mod_path = Some(path);
    AppState::save_settings(&settings)?;
    Ok(())
}

/// Export the current settings to a JSON file at the given path.
#[tauri::command]
pub fn export_config(state: State<AppState>, path: String) -> Result<(), String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    let data = serde_json::to_string_pretty(&*settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, data).map_err(|e| format!("Failed to write config: {}", e))
}

/// Import settings from a JSON file at the given path, replacing the current
/// config entirely.
#[tauri::command]
pub fn import_config(state: State<AppState>, path: String) -> Result<Settings, String> {
    let data = std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;
    let imported: Settings =
        serde_json::from_str(&data).map_err(|e| format!("Invalid config file: {}", e))?;
    let mut current = state.settings.lock().map_err(|e| e.to_string())?;
    *current = imported.clone();
    AppState::save_settings(&imported)?;
    Ok(imported)
}
