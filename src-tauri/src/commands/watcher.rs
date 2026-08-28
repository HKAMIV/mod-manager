use crate::watcher;
use tauri::AppHandle;

/// Start watching the given mod directory for filesystem changes. The frontend
/// listens for the `mods-changed` event and re-scans when it fires. Call this
/// whenever the active game changes or its mod path is set, and only when the
/// user's `auto_reload` setting is enabled.
#[tauri::command]
pub fn watch_mod_directory(app: AppHandle, game_id: String, path: String) -> Result<(), String> {
    watcher::watch(app, game_id, path)
}

/// Stop watching the mod directory for the given game.
#[tauri::command]
pub fn unwatch_mod_directory(game_id: String) -> Result<(), String> {
    watcher::unwatch(&game_id)
}
