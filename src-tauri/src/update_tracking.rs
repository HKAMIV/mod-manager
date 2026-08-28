//! Phase 8 — Update Tracking.
//!
//! Persists the link between a locally-installed mod folder and its
//! GameBanana origin (mod ID + file ID + version string), so the app can
//! later batch-query GB for newer versions and surface "update available"
//! indicators in the mod list without the user needing to manually check
//! each mod's page.
//!
//! Storage: a per-game JSON manifest at
//! `~/.config/mod-manager/origins/<game_id>.json`, mapping the mod's stable
//! `key` (same key used by presets/restore points — survives enable/disable
//! toggling) to its GB origin metadata. This avoids touching the mod folder
//! itself (no hidden dotfiles that might confuse other tools) and keeps the
//! data model consistent with how presets/restore_points already work.

use crate::gamebanana;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// The persisted link between an installed mod and its GameBanana source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModOrigin {
    /// The mod's stable key (e.g. "Characters/Furina") — same as
    /// `ModInfo.key` / what presets store.
    pub mod_key: String,
    /// GameBanana mod ID (the numeric page ID, e.g. 703813).
    pub gb_mod_id: u32,
    /// The specific file ID that was installed (e.g. 1783188 — corresponds
    /// to one upload/version on the mod's GB page).
    pub installed_file_id: u64,
    /// The version string from the installed file at install time, if the
    /// uploader set one. Free-text, not semver — used for display only, not
    /// comparison.
    pub installed_version: Option<String>,
    /// Unix timestamp when the install (link) happened.
    pub installed_at: u64,
}

/// Returned by `check_for_updates` for each mod that has a newer file
/// available on GameBanana than what's currently installed.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub mod_key: String,
    pub gb_mod_id: u32,
    pub mod_name: String,
    /// The installed file's ID (what we currently have).
    pub installed_file_id: u64,
    pub installed_version: Option<String>,
    /// The newest available file on GB.
    pub latest_file_id: u64,
    pub latest_version: Option<String>,
    pub latest_file_name: String,
    pub latest_download_url: String,
    pub latest_filesize: u64,
}

fn origins_dir() -> PathBuf {
    AppState::config_dir().join("origins")
}

fn manifest_path(game_id: &str) -> PathBuf {
    origins_dir().join(format!("{}.json", game_id))
}

fn load_origins(game_id: &str) -> HashMap<String, ModOrigin> {
    let path = manifest_path(game_id);
    let Ok(data) = fs::read_to_string(&path) else {
        return HashMap::new();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

fn save_origins(game_id: &str, origins: &HashMap<String, ModOrigin>) -> Result<(), String> {
    fs::create_dir_all(origins_dir()).map_err(|e| e.to_string())?;
    let data = serde_json::to_string_pretty(origins).map_err(|e| e.to_string())?;
    fs::write(manifest_path(game_id), data).map_err(|e| e.to_string())
}

/// Record that an installed mod (identified by its stable key) came from a
/// specific GameBanana mod + file. Called automatically by the download
/// pipeline on successful install, and can also be called manually to link
/// a mod that was installed outside the app.
pub fn link_mod(
    game_id: &str,
    mod_key: &str,
    gb_mod_id: u32,
    installed_file_id: u64,
    installed_version: Option<String>,
) -> Result<(), String> {
    let mut origins = load_origins(game_id);
    origins.insert(
        mod_key.to_string(),
        ModOrigin {
            mod_key: mod_key.to_string(),
            gb_mod_id,
            installed_file_id,
            installed_version,
            installed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        },
    );
    save_origins(game_id, &origins)
}

/// Remove the GB origin link for a mod (e.g. when the user deletes it, or
/// manually wants to stop tracking a particular mod).
pub fn unlink_mod(game_id: &str, mod_key: &str) -> Result<(), String> {
    let mut origins = load_origins(game_id);
    origins.remove(mod_key);
    save_origins(game_id, &origins)
}

/// Get all tracked mod origins for a game.
pub fn get_origins(game_id: &str) -> Vec<ModOrigin> {
    load_origins(game_id).into_values().collect()
}

/// Query GameBanana for the current file list of every tracked mod, and
/// return update info for any mod whose installed file_id is no longer the
/// newest non-archived upload. This is intentionally sequential and
/// rate-limited (one request per mod with a small delay between) rather
/// than concurrent, since GameBanana has no documented batch endpoint and
/// aggressive parallel requests would be rude.
pub async fn check_for_updates(game_id: &str) -> Result<Vec<UpdateInfo>, String> {
    let origins = load_origins(game_id);
    if origins.is_empty() {
        return Ok(Vec::new());
    }

    let mut updates = Vec::new();

    for origin in origins.values() {
        // Small delay between requests to avoid hammering GB.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        let detail = match gamebanana::get_mod_detail(origin.gb_mod_id).await {
            Ok(d) => d,
            Err(_) => continue, // mod might have been removed from GB — skip, don't fail
        };

        // The newest non-archived file is the "current" version. GB returns
        // files newest-first, so the first non-archived entry is it.
        let latest = detail
            .files
            .iter()
            .find(|f| !f.is_archived);

        let Some(latest) = latest else {
            continue; // all files archived (mod withdrawn?) — nothing to update to
        };

        if latest.id != origin.installed_file_id {
            updates.push(UpdateInfo {
                mod_key: origin.mod_key.clone(),
                gb_mod_id: origin.gb_mod_id,
                mod_name: detail.name.clone(),
                installed_file_id: origin.installed_file_id,
                installed_version: origin.installed_version.clone(),
                latest_file_id: latest.id,
                latest_version: latest.version.clone(),
                latest_file_name: latest.file_name.clone(),
                latest_download_url: latest.download_url.clone(),
                latest_filesize: latest.filesize_bytes,
            });
        }
    }

    Ok(updates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn unique_game_id() -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("__test_origins_{}_{}", std::process::id(), n)
    }

    fn cleanup(game_id: &str) {
        let _ = fs::remove_file(manifest_path(game_id));
    }

    #[test]
    fn link_and_get_origin() {
        let game_id = unique_game_id();
        link_mod(&game_id, "Characters/Furina", 123456, 789, Some("v1.0".to_string()))
            .expect("link should succeed");

        let origins = get_origins(&game_id);
        assert_eq!(origins.len(), 1);
        assert_eq!(origins[0].mod_key, "Characters/Furina");
        assert_eq!(origins[0].gb_mod_id, 123456);
        assert_eq!(origins[0].installed_file_id, 789);
        assert_eq!(origins[0].installed_version.as_deref(), Some("v1.0"));
        assert!(origins[0].installed_at > 0);

        cleanup(&game_id);
    }

    #[test]
    fn link_overwrites_existing_origin() {
        let game_id = unique_game_id();
        link_mod(&game_id, "Characters/Furina", 100, 1, None).unwrap();
        link_mod(&game_id, "Characters/Furina", 100, 2, Some("v2".to_string())).unwrap();

        let origins = get_origins(&game_id);
        assert_eq!(origins.len(), 1);
        assert_eq!(origins[0].installed_file_id, 2);
        assert_eq!(origins[0].installed_version.as_deref(), Some("v2"));

        cleanup(&game_id);
    }

    #[test]
    fn unlink_removes_origin() {
        let game_id = unique_game_id();
        link_mod(&game_id, "Characters/Furina", 100, 1, None).unwrap();
        unlink_mod(&game_id, "Characters/Furina").unwrap();

        let origins = get_origins(&game_id);
        assert!(origins.is_empty());

        cleanup(&game_id);
    }

    #[test]
    fn get_origins_returns_empty_for_unknown_game() {
        let origins = get_origins("__nonexistent_game_xyz_999");
        assert!(origins.is_empty());
    }
}
