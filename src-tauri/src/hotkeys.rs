use crate::presets;
use crate::state::AppState;
use crate::watcher::MODS_CHANGED_EVENT;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// Tracks which shortcut string is currently registered for which preset, so
/// we can unregister cleanly when a hotkey is changed or cleared instead of
/// leaking stale OS-level registrations. Keyed by preset_id.
#[derive(Default)]
pub struct HotkeyRegistry {
    bound_shortcuts: Mutex<HashMap<String, String>>, // preset_id -> shortcut string
}

/// Re-register every preset hotkey found across all known games. Called once
/// at startup.
pub fn init(app: &AppHandle) {
    let game_ids = {
        let state = app.state::<AppState>();
        let settings = state.settings.lock().unwrap();
        settings.games.iter().map(|g| g.id.clone()).collect::<Vec<_>>()
    };

    let bound = presets::list_all_presets_with_hotkeys(&game_ids);
    for (game_id, preset) in bound {
        if let Some(hotkey) = preset.hotkey.clone() {
            if let Err(e) = register(app, &game_id, &preset.id, &hotkey) {
                eprintln!(
                    "Failed to register hotkey '{}' for preset '{}': {}",
                    hotkey, preset.name, e
                );
            }
        }
    }
}

/// Register a global shortcut that applies the given preset when pressed.
/// If this preset already has a different shortcut registered, that one is
/// unregistered first.
pub fn register(app: &AppHandle, game_id: &str, preset_id: &str, shortcut: &str) -> Result<(), String> {
    unregister_for_preset(app, preset_id)?;

    let parsed: Shortcut =
        Shortcut::from_str(shortcut).map_err(|e| format!("Invalid shortcut '{}': {}", shortcut, e))?;

    let game_id = game_id.to_string();
    let preset_id_owned = preset_id.to_string();
    let app_handle = app.clone();

    app.global_shortcut()
        .on_shortcut(parsed, move |_app, _shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }

            let state = app_handle.state::<AppState>();
            let mod_path = {
                let settings = state.settings.lock().unwrap();
                settings
                    .games
                    .iter()
                    .find(|g| g.id == game_id)
                    .and_then(|g| g.mod_path.clone())
            };
            let Some(mod_path) = mod_path else {
                eprintln!("Hotkey fired for '{}' but no mod path is configured", game_id);
                return;
            };

            match presets::apply_preset(&game_id, &mod_path, &preset_id_owned) {
                Ok(_) => {
                    let _ = app_handle.emit(MODS_CHANGED_EVENT, &game_id);
                }
                Err(e) => eprintln!("Failed to apply preset via hotkey: {}", e),
            }
        })
        .map_err(|e| format!("Failed to register shortcut '{}': {}", shortcut, e))?;

    let registry = app.state::<HotkeyRegistry>();
    let mut bound = registry.bound_shortcuts.lock().map_err(|e| e.to_string())?;
    bound.insert(preset_id.to_string(), shortcut.to_string());

    Ok(())
}

/// Unregister the shortcut currently bound (if any) for a given preset.
pub fn unregister_for_preset(app: &AppHandle, preset_id: &str) -> Result<(), String> {
    let registry = app.state::<HotkeyRegistry>();
    let previous = {
        let mut bound = registry.bound_shortcuts.lock().map_err(|e| e.to_string())?;
        bound.remove(preset_id)
    };

    if let Some(shortcut_str) = previous {
        if let Ok(parsed) = Shortcut::from_str(&shortcut_str) {
            let _ = app.global_shortcut().unregister(parsed);
        }
    }

    Ok(())
}
