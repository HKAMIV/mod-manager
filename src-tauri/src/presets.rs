use crate::mods::scan_mods;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// A named snapshot of which mods should be enabled for a given game. Presets
/// store the mod's stable `key` (category/folder, DISABLED_ prefix stripped),
/// not its path or id, since those change when a mod is toggled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    /// Keys (see ModInfo::key) of every mod that should be enabled when this
    /// preset is applied. Any mod not in this set is disabled on apply.
    pub enabled_keys: Vec<String>,
    pub created_at: u64,
    /// Optional global keyboard shortcut (e.g. "CommandOrControl+Alt+1") bound
    /// to this preset. When set, pressing it applies the preset even while the
    /// app is unfocused. See `crate::hotkeys` for registration.
    #[serde(default)]
    pub hotkey: Option<String>,
}

/// Result of applying a preset: how many mods were toggled on/off, and any
/// per-mod errors encountered (e.g. a naming collision), so the caller can
/// report a partial-success outcome instead of aborting entirely.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPresetResult {
    pub enabled_count: u32,
    pub disabled_count: u32,
    pub errors: Vec<String>,
}

fn presets_dir() -> PathBuf {
    AppState::config_dir().join("presets")
}

fn presets_path(game_id: &str) -> PathBuf {
    presets_dir().join(format!("{}.json", game_id))
}

fn load_presets(game_id: &str) -> Result<Vec<Preset>, String> {
    let path = presets_path(game_id);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

fn save_presets(game_id: &str, presets: &[Preset]) -> Result<(), String> {
    fs::create_dir_all(presets_dir()).map_err(|e| e.to_string())?;
    let path = presets_path(game_id);
    let data = serde_json::to_string_pretty(presets).map_err(|e| e.to_string())?;
    fs::write(path, data).map_err(|e| e.to_string())
}

pub fn list_presets(game_id: &str) -> Result<Vec<Preset>, String> {
    let mut presets = load_presets(game_id)?;
    presets.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(presets)
}

/// Save a new preset capturing the currently-enabled mods in `mod_path`.
pub fn save_preset(game_id: &str, mod_path: &str, name: &str) -> Result<Preset, String> {
    let mods = scan_mods(mod_path)?;
    let enabled_keys: Vec<String> = mods.iter().filter(|m| m.enabled).map(|m| m.key.clone()).collect();

    let mut presets = load_presets(game_id)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let preset = Preset {
        id: format!("{}-{}", now, presets.len()),
        name: name.to_string(),
        enabled_keys,
        created_at: now,
        hotkey: None,
    };

    presets.push(preset.clone());
    save_presets(game_id, &presets)?;
    Ok(preset)
}

/// Set or clear (pass None) the hotkey string bound to a preset. Does not
/// register the shortcut with the OS itself — that's handled by
/// `crate::hotkeys`, which should be called with the updated preset list
/// afterwards to keep the live registrations in sync.
pub fn set_preset_hotkey(game_id: &str, preset_id: &str, hotkey: Option<String>) -> Result<Preset, String> {
    let mut presets = load_presets(game_id)?;
    let preset = presets
        .iter_mut()
        .find(|p| p.id == preset_id)
        .ok_or_else(|| format!("Preset '{}' not found", preset_id))?;
    preset.hotkey = hotkey;
    let updated = preset.clone();
    save_presets(game_id, &presets)?;
    Ok(updated)
}

/// Load every preset across every game that has a hotkey assigned. Used at
/// startup to re-register all bound shortcuts.
pub fn list_all_presets_with_hotkeys(game_ids: &[String]) -> Vec<(String, Preset)> {
    let mut result = Vec::new();
    for game_id in game_ids {
        if let Ok(presets) = load_presets(game_id) {
            for preset in presets {
                if preset.hotkey.is_some() {
                    result.push((game_id.clone(), preset));
                }
            }
        }
    }
    result
}

pub fn delete_preset(game_id: &str, preset_id: &str) -> Result<(), String> {
    let mut presets = load_presets(game_id)?;
    let original_len = presets.len();
    presets.retain(|p| p.id != preset_id);
    if presets.len() == original_len {
        return Err(format!("Preset '{}' not found", preset_id));
    }
    save_presets(game_id, &presets)
}

pub fn rename_preset(game_id: &str, preset_id: &str, new_name: &str) -> Result<Preset, String> {
    let mut presets = load_presets(game_id)?;
    let preset = presets
        .iter_mut()
        .find(|p| p.id == preset_id)
        .ok_or_else(|| format!("Preset '{}' not found", preset_id))?;
    preset.name = new_name.to_string();
    let updated = preset.clone();
    save_presets(game_id, &presets)?;
    Ok(updated)
}

/// Apply a preset: enable every mod whose key is in the preset's enabled set,
/// disable every other mod. Continues past individual toggle failures (e.g. a
/// naming collision) and reports them in `errors` rather than aborting, since
/// one bad mod shouldn't block the rest of the preset from applying.
pub fn apply_preset(game_id: &str, mod_path: &str, preset_id: &str) -> Result<ApplyPresetResult, String> {
    let presets = load_presets(game_id)?;
    let preset = presets
        .iter()
        .find(|p| p.id == preset_id)
        .ok_or_else(|| format!("Preset '{}' not found", preset_id))?;

    let enabled_keys: HashSet<&str> = preset.enabled_keys.iter().map(|s| s.as_str()).collect();
    let mods = scan_mods(mod_path)?;

    let mut enabled_count = 0;
    let mut disabled_count = 0;
    let mut errors = Vec::new();

    for m in &mods {
        let should_be_enabled = enabled_keys.contains(m.key.as_str());
        if m.enabled == should_be_enabled {
            continue;
        }
        match crate::mods::set_mod_enabled(&m.path, &m.category, should_be_enabled) {
            Ok(_) => {
                if should_be_enabled {
                    enabled_count += 1;
                } else {
                    disabled_count += 1;
                }
            }
            Err(e) => errors.push(format!("{}: {}", m.name, e)),
        }
    }

    Ok(ApplyPresetResult {
        enabled_count,
        disabled_count,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Presets are keyed by game_id and persisted under the real XDG config
    /// dir (there's no dependency injection for the storage location), so
    /// tests use a unique fake game_id per run to avoid colliding with real
    /// user data or with each other when run in parallel, and clean up the
    /// file they create afterward.
    fn unique_test_game_id() -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("__test_game_{}_{}", std::process::id(), n)
    }

    fn cleanup(game_id: &str) {
        let _ = fs::remove_file(presets_path(game_id));
    }

    fn temp_mod_dir_with(mods: &[(&str, bool)]) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "mod-manager-preset-test-{}-{}",
            std::process::id(),
            n
        ));
        for (name, enabled) in mods {
            let folder_name = if *enabled {
                name.to_string()
            } else {
                format!("DISABLED_{}", name)
            };
            let mod_dir = dir.join(&folder_name);
            fs::create_dir_all(&mod_dir).unwrap();
            fs::write(mod_dir.join("mod.ini"), "test").unwrap();
        }
        dir
    }

    #[test]
    fn save_and_list_presets() {
        let game_id = unique_test_game_id();
        let mod_path = temp_mod_dir_with(&[("Furina", true), ("Nahida", false)]);

        let preset = save_preset(&game_id, mod_path.to_str().unwrap(), "My Preset")
            .expect("save should succeed");
        assert_eq!(preset.name, "My Preset");
        assert_eq!(preset.enabled_keys, vec!["Uncategorized/Furina".to_string()]);

        let listed = list_presets(&game_id).expect("list should succeed");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, preset.id);

        cleanup(&game_id);
        fs::remove_dir_all(&mod_path).ok();
    }

    #[test]
    fn apply_preset_toggles_mods_to_match_saved_state() {
        let game_id = unique_test_game_id();
        // Start with Furina enabled, Nahida disabled — save that as a preset.
        let mod_path = temp_mod_dir_with(&[("Furina", true), ("Nahida", false)]);
        let preset = save_preset(&game_id, mod_path.to_str().unwrap(), "Preset A").unwrap();

        // Now flip both mods the opposite way...
        let mods = scan_mods(mod_path.to_str().unwrap()).unwrap();
        for m in &mods {
            crate::mods::set_mod_enabled(&m.path, &m.category, !m.enabled).unwrap();
        }
        let flipped = scan_mods(mod_path.to_str().unwrap()).unwrap();
        let furina = flipped.iter().find(|m| m.name == "Furina").unwrap();
        assert!(!furina.enabled, "Furina should now be disabled");

        // ...and applying the preset should restore the original state.
        let result = apply_preset(&game_id, mod_path.to_str().unwrap(), &preset.id)
            .expect("apply should succeed");
        assert_eq!(result.enabled_count, 1);
        assert_eq!(result.disabled_count, 1);
        assert!(result.errors.is_empty());

        let restored = scan_mods(mod_path.to_str().unwrap()).unwrap();
        let furina = restored.iter().find(|m| m.name == "Furina").unwrap();
        let nahida = restored.iter().find(|m| m.name == "Nahida").unwrap();
        assert!(furina.enabled);
        assert!(!nahida.enabled);

        cleanup(&game_id);
        fs::remove_dir_all(&mod_path).ok();
    }

    #[test]
    fn delete_and_rename_preset() {
        let game_id = unique_test_game_id();
        let mod_path = temp_mod_dir_with(&[("Furina", true)]);
        let preset = save_preset(&game_id, mod_path.to_str().unwrap(), "Original").unwrap();

        let renamed = rename_preset(&game_id, &preset.id, "Renamed").expect("rename should succeed");
        assert_eq!(renamed.name, "Renamed");

        delete_preset(&game_id, &preset.id).expect("delete should succeed");
        let listed = list_presets(&game_id).unwrap();
        assert!(listed.is_empty());

        cleanup(&game_id);
        fs::remove_dir_all(&mod_path).ok();
    }

    #[test]
    fn set_and_clear_hotkey() {
        let game_id = unique_test_game_id();
        let mod_path = temp_mod_dir_with(&[("Furina", true)]);
        let preset = save_preset(&game_id, mod_path.to_str().unwrap(), "Preset").unwrap();

        let with_hotkey =
            set_preset_hotkey(&game_id, &preset.id, Some("CommandOrControl+Alt+1".to_string()))
                .expect("set hotkey should succeed");
        assert_eq!(with_hotkey.hotkey, Some("CommandOrControl+Alt+1".to_string()));

        let cleared = set_preset_hotkey(&game_id, &preset.id, None).expect("clear should succeed");
        assert_eq!(cleared.hotkey, None);

        cleanup(&game_id);
        fs::remove_dir_all(&mod_path).ok();
    }

    #[test]
    fn operations_on_missing_preset_error() {
        let game_id = unique_test_game_id();
        assert!(delete_preset(&game_id, "does-not-exist").is_err());
        assert!(rename_preset(&game_id, "does-not-exist", "New Name").is_err());
        cleanup(&game_id);
    }
}
