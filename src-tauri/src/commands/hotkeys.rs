use crate::hotkeys;
use crate::presets::{self, Preset};
use tauri::AppHandle;

/// Bind a global keyboard shortcut to a preset, so pressing it applies the
/// preset even while the app is unfocused. Registers the shortcut with the OS
/// and persists it on the preset record.
#[tauri::command]
pub fn set_preset_hotkey(
    app: AppHandle,
    game_id: String,
    preset_id: String,
    hotkey: String,
) -> Result<Preset, String> {
    hotkeys::register(&app, &game_id, &preset_id, &hotkey)?;
    presets::set_preset_hotkey(&game_id, &preset_id, Some(hotkey))
}

/// Remove the global keyboard shortcut bound to a preset, if any.
#[tauri::command]
pub fn clear_preset_hotkey(app: AppHandle, game_id: String, preset_id: String) -> Result<Preset, String> {
    hotkeys::unregister_for_preset(&app, &preset_id)?;
    presets::set_preset_hotkey(&game_id, &preset_id, None)
}
