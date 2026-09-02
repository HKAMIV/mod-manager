use crate::mods::scan_mods;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// One mod's state at the moment a restore point was taken.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    /// Stable identity (see ModInfo::key) — "<category>/<folder name, prefix stripped>".
    pub key: String,
    pub category: String,
    pub enabled: bool,
}

/// A saved restore point: which mods existed and their enabled state, plus
/// optionally a full file backup that can recreate a mod folder if it's been
/// deleted since the snapshot was taken.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestorePoint {
    pub id: String,
    pub name: String,
    pub created_at: u64,
    pub mods: Vec<SnapshotEntry>,
    /// True if `backup` actually copied mod files to disk. False means this
    /// is a state-only snapshot (enabled/disabled toggling only — cannot
    /// recreate a mod that's been deleted).
    pub has_file_backup: bool,
    /// Total size of the file backup in bytes, if one was taken (for display).
    pub backup_size_bytes: u64,
}

/// Outcome of restoring: what changed, and what couldn't be restored. Restore
/// is additive/corrective, never destructive — mods added after the snapshot
/// are left alone rather than deleted, since silently discarding a user's
/// newer work would be a nasty surprise for a "restore" action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    pub enabled_count: u32,
    pub disabled_count: u32,
    pub recreated_count: u32,
    /// Mods present in the snapshot but missing on disk AND not recoverable
    /// (no file backup was taken for this restore point).
    pub unrecoverable: Vec<String>,
    pub errors: Vec<String>,
}

fn restore_points_dir() -> PathBuf {
    AppState::config_dir().join("restore_points")
}

fn manifest_path(game_id: &str) -> PathBuf {
    restore_points_dir().join(format!("{}.json", game_id))
}

/// Where file backups for a given restore point are stored — kept out of the
/// config dir since backups can be large (whole mod folders), unlike the
/// small JSON manifests.
fn backup_dir(game_id: &str, restore_point_id: &str) -> PathBuf {
    AppState::data_dir()
        .join("restore_backups")
        .join(game_id)
        .join(restore_point_id)
}

fn load_manifest(game_id: &str) -> Result<Vec<RestorePoint>, String> {
    let path = manifest_path(game_id);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

fn save_manifest(game_id: &str, points: &[RestorePoint]) -> Result<(), String> {
    fs::create_dir_all(restore_points_dir()).map_err(|e| e.to_string())?;
    let data = serde_json::to_string_pretty(points).map_err(|e| e.to_string())?;
    fs::write(manifest_path(game_id), data).map_err(|e| e.to_string())
}

pub fn list_restore_points(game_id: &str) -> Result<Vec<RestorePoint>, String> {
    let mut points = load_manifest(game_id)?;
    points.sort_by(|a, b| b.created_at.cmp(&a.created_at)); // newest first
    Ok(points)
}

/// Create a restore point capturing the current state of every mod in
/// `mod_path`. If `with_file_backup` is true, also copies every mod folder's
/// files into the backup directory so deleted mods can be recreated later.
pub fn create_restore_point(
    game_id: &str,
    mod_path: &str,
    name: &str,
    with_file_backup: bool,
) -> Result<RestorePoint, String> {
    let mods = scan_mods(mod_path)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut points = load_manifest(game_id)?;
    let id = format!("{}-{}", now, points.len());

    let entries: Vec<SnapshotEntry> = mods
        .iter()
        .map(|m| SnapshotEntry {
            key: m.key.clone(),
            category: m.category.clone(),
            enabled: m.enabled,
        })
        .collect();

    let backup_size_bytes = if with_file_backup {
        let dest_root = backup_dir(game_id, &id);
        fs::create_dir_all(&dest_root).map_err(|e| e.to_string())?;
        let mut total = 0u64;
        for m in &mods {
            let dest = dest_root.join(sanitize_key(&m.key));
            total += copy_dir_recursive(Path::new(&m.path), &dest)?;
        }
        total
    } else {
        0
    };

    let point = RestorePoint {
        id,
        name: name.to_string(),
        created_at: now,
        mods: entries,
        has_file_backup: with_file_backup,
        backup_size_bytes,
    };

    points.push(point.clone());
    save_manifest(game_id, &points)?;
    Ok(point)
}

pub fn delete_restore_point(game_id: &str, restore_point_id: &str) -> Result<(), String> {
    let mut points = load_manifest(game_id)?;
    let original_len = points.len();
    points.retain(|p| p.id != restore_point_id);
    if points.len() == original_len {
        return Err(format!("Restore point '{}' not found", restore_point_id));
    }
    save_manifest(game_id, &points)?;

    // Clean up its file backup, if any. Not fatal if this fails — the
    // manifest entry is already gone, which is what matters for the UI.
    let dir = backup_dir(game_id, restore_point_id);
    if dir.exists() {
        let _ = fs::remove_dir_all(dir);
    }
    Ok(())
}

/// Restore a snapshot: re-enable/disable every mod that still exists to match
/// the snapshot, and recreate any mod that's gone missing from a file backup
/// (if one was taken). Mods that exist now but weren't in the snapshot are
/// left untouched — restoring never deletes anything.
pub fn restore(game_id: &str, mod_path: &str, restore_point_id: &str) -> Result<RestoreResult, String> {
    let points = load_manifest(game_id)?;
    let point = points
        .iter()
        .find(|p| p.id == restore_point_id)
        .ok_or_else(|| format!("Restore point '{}' not found", restore_point_id))?;

    let current_mods = scan_mods(mod_path)?;
    let mut current_by_key: std::collections::HashMap<&str, &crate::mods::ModInfo> =
        std::collections::HashMap::new();
    for m in &current_mods {
        current_by_key.insert(m.key.as_str(), m);
    }

    let mut enabled_count = 0;
    let mut disabled_count = 0;
    let mut recreated_count = 0;
    let mut unrecoverable = Vec::new();
    let mut errors = Vec::new();

    for entry in &point.mods {
        match current_by_key.get(entry.key.as_str()) {
            Some(current) => {
                if current.enabled != entry.enabled {
                    match crate::mods::set_mod_enabled(&current.path, &current.category, entry.enabled) {
                        Ok(_) => {
                            if entry.enabled {
                                enabled_count += 1;
                            } else {
                                disabled_count += 1;
                            }
                        }
                        Err(e) => errors.push(format!("{}: {}", entry.key, e)),
                    }
                }
            }
            None => {
                // Mod is missing on disk — try to recreate it from the backup.
                if !point.has_file_backup {
                    unrecoverable.push(entry.key.clone());
                    continue;
                }
                let backup_src = backup_dir(game_id, &point.id).join(sanitize_key(&entry.key));
                if !backup_src.exists() {
                    unrecoverable.push(entry.key.clone());
                    continue;
                }
                match recreate_mod(mod_path, entry, &backup_src) {
                    Ok(_) => recreated_count += 1,
                    Err(e) => errors.push(format!("{}: {}", entry.key, e)),
                }
            }
        }
    }

    Ok(RestoreResult {
        enabled_count,
        disabled_count,
        recreated_count,
        unrecoverable,
        errors,
    })
}

/// Recreate a mod folder from a backup directory.
///
/// Handles both layouts:
/// - **Legacy**: places the folder under `mod_path/<category>/<name>`, applying
///   the `DISABLED_` prefix when the mod was disabled at snapshot time.
/// - **Symlink layout**: places the folder under `managed_src/<category>/<name>`
///   and, if the mod was enabled, creates the corresponding symlink in
///   `managed_tgt`.
fn recreate_mod(mod_path: &str, entry: &SnapshotEntry, backup_src: &Path) -> Result<(), String> {
    let base_name = entry.key.rsplit('/').next().unwrap_or(&entry.key);

    if crate::mods::is_symlink_layout(mod_path) {
        // Symlink layout: restore into managed_src, then symlink if enabled.
        let src_root = crate::mods::src_root(mod_path);
        let dest_parent = if entry.category == "Uncategorized" {
            src_root.clone()
        } else {
            src_root.join(&entry.category)
        };
        fs::create_dir_all(&dest_parent).map_err(|e| e.to_string())?;
        let dest = dest_parent.join(base_name);
        if dest.exists() {
            return Err(format!("A folder named '{}' already exists in managed_src", base_name));
        }
        copy_dir_recursive(backup_src, &dest)?;

        if entry.enabled {
            let tgt_root = crate::mods::tgt_root(mod_path);
            let link_parent = if entry.category == "Uncategorized" {
                tgt_root
            } else {
                tgt_root.join(&entry.category)
            };
            fs::create_dir_all(&link_parent).map_err(|e| e.to_string())?;
            let link = link_parent.join(base_name);
            crate::symlink::create_dir_symlink(&dest, &link)?;
        }
    } else {
        // Legacy layout: recreate with DISABLED_ prefix when disabled.
        let folder = if entry.enabled {
            base_name.to_string()
        } else {
            format!("{}{}", crate::mods::DISABLED_PREFIX, base_name)
        };
        let dest_parent = if entry.category == "Uncategorized" {
            PathBuf::from(mod_path)
        } else {
            PathBuf::from(mod_path).join(&entry.category)
        };
        fs::create_dir_all(&dest_parent).map_err(|e| e.to_string())?;
        let dest = dest_parent.join(&folder);
        if dest.exists() {
            return Err(format!("A folder named '{}' already exists", folder));
        }
        copy_dir_recursive(backup_src, &dest)?;
    }

    Ok(())
}

/// Turn a mod key ("Characters/Furina") into a filesystem-safe single
/// component for storage under the backup directory, since keys contain a
/// path separator that shouldn't create nested directories there.
fn sanitize_key(key: &str) -> String {
    key.replace('/', "__")
}

/// Recursively copy a directory's contents to `dest`, creating `dest` if it
/// doesn't exist. Returns the total number of bytes copied. Shared with
/// `downloads` (used when placing an extracted mod alongside conflicting
/// existing files needs a plain directory copy rather than a move).
pub(crate) fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<u64, String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let mut total = 0u64;

    let entries = fs::read_dir(src).map_err(|e| format!("Failed to read {}: {}", src.display(), e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if path.is_dir() {
            total += copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)
                .map_err(|e| format!("Failed to copy {}: {}", path.display(), e))?;
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Restore points are persisted under the real XDG config/data dirs
    /// (there's no dependency injection for storage location), so tests use
    /// a unique fake game_id per run to avoid colliding with real user data
    /// or with each other in parallel, and clean up what they create.
    fn unique_test_game_id() -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("__test_rp_game_{}_{}", std::process::id(), n)
    }

    fn cleanup(game_id: &str) {
        let _ = fs::remove_file(manifest_path(game_id));
        let _ = fs::remove_dir_all(AppState::data_dir().join("restore_backups").join(game_id));
    }

    fn temp_mod_dir_with(mods: &[(&str, &str, bool)]) -> PathBuf {
        // Each entry: (category, name, enabled). category "Uncategorized"
        // places the mod directly under the root, matching scan_mods' rule.
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "mod-manager-rp-test-{}-{}",
            std::process::id(),
            n
        ));
        for (category, name, enabled) in mods {
            let folder_name = if *enabled {
                name.to_string()
            } else {
                format!("DISABLED_{}", name)
            };
            let mod_dir = if *category == "Uncategorized" {
                dir.join(&folder_name)
            } else {
                dir.join(category).join(&folder_name)
            };
            fs::create_dir_all(&mod_dir).unwrap();
            fs::write(mod_dir.join("mod.ini"), "test-contents").unwrap();
        }
        dir
    }

    #[test]
    fn create_and_list_state_only_restore_point() {
        let game_id = unique_test_game_id();
        let mod_path = temp_mod_dir_with(&[("Characters", "Furina", true)]);

        let point = create_restore_point(&game_id, mod_path.to_str().unwrap(), "Before update", false)
            .expect("create should succeed");
        assert!(!point.has_file_backup);
        assert_eq!(point.backup_size_bytes, 0);
        assert_eq!(point.mods.len(), 1);
        assert_eq!(point.mods[0].key, "Characters/Furina");
        assert!(point.mods[0].enabled);

        let listed = list_restore_points(&game_id).expect("list should succeed");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, point.id);

        cleanup(&game_id);
        fs::remove_dir_all(&mod_path).ok();
    }

    #[test]
    fn restore_reapplies_enabled_state_without_file_backup() {
        let game_id = unique_test_game_id();
        let mod_path = temp_mod_dir_with(&[
            ("Characters", "Furina", true),
            ("Characters", "Nahida", false),
        ]);
        let point = create_restore_point(&game_id, mod_path.to_str().unwrap(), "Snapshot", false)
            .unwrap();

        // Flip both mods the opposite way.
        let mods = scan_mods(mod_path.to_str().unwrap()).unwrap();
        for m in &mods {
            crate::mods::set_mod_enabled(&m.path, &m.category, !m.enabled).unwrap();
        }

        let result = restore(&game_id, mod_path.to_str().unwrap(), &point.id).expect("restore should succeed");
        assert_eq!(result.enabled_count, 1);
        assert_eq!(result.disabled_count, 1);
        assert_eq!(result.recreated_count, 0);
        assert!(result.unrecoverable.is_empty());
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
    fn restore_without_backup_reports_deleted_mod_as_unrecoverable() {
        let game_id = unique_test_game_id();
        let mod_path = temp_mod_dir_with(&[("Characters", "Furina", true)]);
        let point = create_restore_point(&game_id, mod_path.to_str().unwrap(), "Snapshot", false)
            .unwrap();

        // Simulate the user deleting the mod folder entirely.
        fs::remove_dir_all(mod_path.join("Characters").join("Furina")).unwrap();

        let result = restore(&game_id, mod_path.to_str().unwrap(), &point.id).expect("restore should succeed");
        assert_eq!(result.recreated_count, 0);
        assert_eq!(result.unrecoverable, vec!["Characters/Furina".to_string()]);

        cleanup(&game_id);
        fs::remove_dir_all(&mod_path).ok();
    }

    #[test]
    fn restore_with_file_backup_recreates_a_deleted_mod() {
        let game_id = unique_test_game_id();
        let mod_path = temp_mod_dir_with(&[("Characters", "Furina", true)]);
        let point = create_restore_point(&game_id, mod_path.to_str().unwrap(), "Full backup", true)
            .expect("create with backup should succeed");
        assert!(point.has_file_backup);
        assert!(point.backup_size_bytes > 0);

        // Delete the mod folder entirely — this is the scenario a file
        // backup exists to recover from.
        fs::remove_dir_all(mod_path.join("Characters").join("Furina")).unwrap();
        assert!(!mod_path.join("Characters").join("Furina").exists());

        let result = restore(&game_id, mod_path.to_str().unwrap(), &point.id).expect("restore should succeed");
        assert_eq!(result.recreated_count, 1);
        assert!(result.unrecoverable.is_empty());
        assert!(result.errors.is_empty());

        // The folder and its file contents should be back.
        let recreated_ini = mod_path.join("Characters").join("Furina").join("mod.ini");
        assert!(recreated_ini.exists());
        assert_eq!(fs::read_to_string(recreated_ini).unwrap(), "test-contents");

        cleanup(&game_id);
        fs::remove_dir_all(&mod_path).ok();
    }

    #[test]
    fn restore_never_touches_mods_added_after_the_snapshot() {
        let game_id = unique_test_game_id();
        let mod_path = temp_mod_dir_with(&[("Characters", "Furina", true)]);
        let point = create_restore_point(&game_id, mod_path.to_str().unwrap(), "Snapshot", false)
            .unwrap();

        // Add a new mod after the snapshot was taken.
        let new_mod_dir = mod_path.join("Characters").join("Nahida");
        fs::create_dir_all(&new_mod_dir).unwrap();
        fs::write(new_mod_dir.join("mod.ini"), "new mod").unwrap();

        restore(&game_id, mod_path.to_str().unwrap(), &point.id).expect("restore should succeed");

        // The newly added mod must still exist untouched — restore is
        // additive/corrective, never destructive.
        assert!(new_mod_dir.exists());
        let restored = scan_mods(mod_path.to_str().unwrap()).unwrap();
        assert!(restored.iter().any(|m| m.name == "Nahida"));

        cleanup(&game_id);
        fs::remove_dir_all(&mod_path).ok();
    }

    #[test]
    fn delete_restore_point_removes_manifest_entry_and_backup() {
        let game_id = unique_test_game_id();
        let mod_path = temp_mod_dir_with(&[("Characters", "Furina", true)]);
        let point = create_restore_point(&game_id, mod_path.to_str().unwrap(), "To delete", true)
            .unwrap();
        let backup_path = backup_dir(&game_id, &point.id);
        assert!(backup_path.exists());

        delete_restore_point(&game_id, &point.id).expect("delete should succeed");

        assert!(list_restore_points(&game_id).unwrap().is_empty());
        assert!(!backup_path.exists());

        cleanup(&game_id);
        fs::remove_dir_all(&mod_path).ok();
    }

    #[test]
    fn operations_on_missing_restore_point_error() {
        let game_id = unique_test_game_id();
        assert!(delete_restore_point(&game_id, "does-not-exist").is_err());
        cleanup(&game_id);
    }
}

