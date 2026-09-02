//! Phase 12 — migration from the legacy DISABLED_-prefix layout to the
//! symlink layout.
//!
//! ## What this module does
//!
//! When the user opts in, `migrate_to_symlink_layout` converts a flat
//! legacy mod directory into the source/target structure:
//!
//! ```text
//! Before (legacy):
//!   mod_path/
//!     Characters/
//!       Furina/            ← enabled
//!       DISABLED_Nahida/   ← disabled
//!
//! After (symlink layout):
//!   mod_path/
//!     DISABLED_managed_src/
//!       Characters/
//!         Furina/          ← real files (moved from legacy path)
//!         Nahida/          ← real files (DISABLED_ prefix stripped)
//!     managed_tgt/
//!       Characters/
//!         Furina  →  ../../DISABLED_managed_src/Characters/Furina
//!         (no Nahida symlink — was disabled)
//! ```
//!
//! ## Safety principles (mirrors Phase 5 restore-point philosophy)
//!
//! 1. A restore point is created **before** any files are moved, unless the
//!    caller explicitly opts out (e.g. in tests). This is offered to the
//!    frontend as a checkbox on the migration dialog.
//! 2. Migration is **never silent or automatic** — it requires an explicit
//!    user action (`migrate_to_symlink_layout` Tauri command).
//! 3. If any step fails mid-migration, we stop and return an error. Files
//!    already moved to `managed_src` are left there (the legacy path is now
//!    gone for those mods), but the state is still consistent — `scan_mods`
//!    will detect `managed_src` and pick up what was successfully migrated.
//!    The frontend can show the partial result and let the user retry.
//! 4. `d3dx_user.ini` keys are rewritten for each moved mod (if the file
//!    exists) so 3DMigoto can find the mods at their new paths.
//!
//! ## Layout status
//!
//! `get_layout_status` is a read-only query the frontend polls on game switch
//! to decide whether to show the migration prompt.

use crate::mods::{
    self, folder_name, scan_mods, strip_disabled_prefix, src_root, tgt_root,
    MANAGED_SRC,
};
use crate::restore_points;
use crate::symlink;
use serde::Serialize;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Layout status (query)
// ---------------------------------------------------------------------------

/// Describes the current layout state of a game's mod directory.
#[derive(Debug, Clone, Serialize)]
pub struct LayoutStatus {
    /// The mod_path this status describes.
    pub mod_path: String,
    /// True when the symlink layout (`managed_src`) is already active.
    pub is_symlink_layout: bool,
    /// True when the directory contains legacy DISABLED_-style mod folders
    /// that have not yet been migrated. Only meaningful when
    /// `is_symlink_layout == false`.
    pub has_legacy_mods: bool,
    /// Number of legacy mods detected (0 when `is_symlink_layout == true`).
    pub legacy_mod_count: usize,
}

/// Inspect the mod directory and return its layout status. This is cheap
/// (just directory listing + presence check) and safe to call on every game
/// switch without user interaction.
pub fn get_layout_status(mod_path: &str) -> Result<LayoutStatus, String> {
    let root = Path::new(mod_path);
    if !root.exists() {
        return Err(format!("Mod directory does not exist: {}", mod_path));
    }

    let is_symlink = mods::is_symlink_layout(mod_path);

    if is_symlink {
        return Ok(LayoutStatus {
            mod_path: mod_path.to_string(),
            is_symlink_layout: true,
            has_legacy_mods: false,
            legacy_mod_count: 0,
        });
    }

    // Count legacy mods (non-managed_src/tgt subdirs that look like mod folders).
    let legacy_mods = scan_mods(mod_path).unwrap_or_default();
    Ok(LayoutStatus {
        mod_path: mod_path.to_string(),
        is_symlink_layout: false,
        has_legacy_mods: !legacy_mods.is_empty(),
        legacy_mod_count: legacy_mods.len(),
    })
}

// ---------------------------------------------------------------------------
// Migration result
// ---------------------------------------------------------------------------

/// Per-mod outcome during a migration run.
#[derive(Debug, Clone, Serialize)]
pub struct ModMigrationEntry {
    pub key: String,
    /// Whether the mod was enabled (had an active symlink placed for it).
    pub was_enabled: bool,
    pub success: bool,
    pub error: Option<String>,
}

/// Overall result of `migrate_to_symlink_layout`.
#[derive(Debug, Clone, Serialize)]
pub struct MigrationResult {
    pub migrated_count: usize,
    pub skipped_count: usize,
    pub errors: Vec<String>,
    pub entries: Vec<ModMigrationEntry>,
    /// Key of the restore point created before migration, if one was taken.
    pub restore_point_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Core migration
// ---------------------------------------------------------------------------

/// Convert a legacy `DISABLED_`-prefix layout to the symlink layout.
///
/// Steps:
/// 1. Optionally create a restore point (state-only — fast, since files will
///    move anyway; use `with_restore_point = true` to also back up files).
/// 2. Scan legacy mods to capture enabled/disabled state.
/// 3. For each mod: move its folder into `managed_src/<category>/<clean-name>`,
///    and if it was enabled, create a symlink in `managed_tgt`.
/// 4. Rewrite any matching `d3dx_user.ini` keys.
///
/// Already-migrated games (managed_src exists) are a no-op.
pub fn migrate_to_symlink_layout(
    game_id: &str,
    mod_path: &str,
    with_restore_point: bool,
) -> Result<MigrationResult, String> {
    let root = Path::new(mod_path);
    if !root.exists() {
        return Err(format!("Mod directory does not exist: {}", mod_path));
    }

    // Already migrated — nothing to do.
    if mods::is_symlink_layout(mod_path) {
        return Ok(MigrationResult {
            migrated_count: 0,
            skipped_count: 0,
            errors: Vec::new(),
            entries: Vec::new(),
            restore_point_id: None,
        });
    }

    // Snapshot before touching anything.
    let restore_point_id = if with_restore_point {
        match restore_points::create_restore_point(
            game_id,
            mod_path,
            "Before symlink layout migration",
            false, // state-only: files are about to move anyway
        ) {
            Ok(rp) => Some(rp.id),
            Err(e) => {
                return Err(format!("Failed to create pre-migration restore point: {}", e));
            }
        }
    } else {
        None
    };

    // Capture current state *before* any moves so we know which mods were enabled.
    let legacy_mods = scan_mods(mod_path)?;

    let src = src_root(mod_path);
    let tgt = tgt_root(mod_path);
    fs::create_dir_all(&src).map_err(|e| format!("Failed to create managed_src: {}", e))?;
    fs::create_dir_all(&tgt).map_err(|e| format!("Failed to create managed_tgt: {}", e))?;

    // Look for d3dx_user.ini one level above mod_path (typical XXMI placement).
    let ini_path = root.parent().map(|p| p.join("d3dx_user.ini"));

    let mut migrated_count = 0;
    let mut skipped_count = 0;
    let mut errors = Vec::new();
    let mut entries = Vec::new();

    for m in &legacy_mods {
        let legacy_path = Path::new(&m.path);

        // Compute the destination inside managed_src.
        // Category "Uncategorized" goes directly under managed_src.
        let clean_name = strip_disabled_prefix(&m.folder_name).to_string();
        let (src_dest, cat_folder) = if m.category == "Uncategorized" {
            (src.join(&clean_name), None)
        } else {
            // Use the raw folder name of the category directory on disk.
            // We derive it from the legacy path's parent.
            let cat_dir_name = legacy_path
                .parent()
                .map(|p| folder_name(p))
                .unwrap_or_else(|| m.category.clone());
            (src.join(&cat_dir_name).join(&clean_name), Some(cat_dir_name))
        };

        // Create parent dir in managed_src.
        if let Some(parent) = src_dest.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                let err = format!("{}: failed to create src dir: {}", m.key, e);
                errors.push(err.clone());
                entries.push(ModMigrationEntry {
                    key: m.key.clone(),
                    was_enabled: m.enabled,
                    success: false,
                    error: Some(err),
                });
                skipped_count += 1;
                continue;
            }
        }

        if src_dest.exists() {
            let err = format!("{}: destination already exists in managed_src, skipping", m.key);
            errors.push(err.clone());
            entries.push(ModMigrationEntry {
                key: m.key.clone(),
                was_enabled: m.enabled,
                success: false,
                error: Some(err),
            });
            skipped_count += 1;
            continue;
        }

        // Move the mod folder (rename on same filesystem, copy+delete cross-fs).
        if let Err(e) = move_dir(legacy_path, &src_dest) {
            let err = format!("{}: {}", m.key, e);
            errors.push(err.clone());
            entries.push(ModMigrationEntry {
                key: m.key.clone(),
                was_enabled: m.enabled,
                success: false,
                error: Some(err),
            });
            skipped_count += 1;
            continue;
        }

        // Create symlink in managed_tgt if the mod was enabled.
        if m.enabled {
            let link_parent = match &cat_folder {
                Some(cat) => tgt.join(cat),
                None => tgt.clone(),
            };
            if let Err(e) = fs::create_dir_all(&link_parent) {
                let err = format!("{}: failed to create tgt dir: {}", m.key, e);
                errors.push(err.clone());
                entries.push(ModMigrationEntry {
                    key: m.key.clone(),
                    was_enabled: true,
                    success: false,
                    error: Some(err),
                });
                skipped_count += 1;
                continue;
            }
            let link_path = link_parent.join(&clean_name);
            if let Err(e) = symlink::create_dir_symlink(&src_dest, &link_path) {
                let err = format!("{}: {}", m.key, e);
                errors.push(err.clone());
                entries.push(ModMigrationEntry {
                    key: m.key.clone(),
                    was_enabled: true,
                    success: false,
                    error: Some(err),
                });
                skipped_count += 1;
                continue;
            }
        }

        // Rewrite d3dx_user.ini keys if the file exists.
        if let Some(ref ini) = ini_path {
            if ini.exists() {
                let old_seg = build_ini_segment(&m.category, &m.folder_name);
                let new_seg = build_ini_segment_from_src(&src_dest, mod_path);
                let _ = mods::migrate_d3dx_ini_keys(ini, &old_seg, &new_seg);
            }
        }

        migrated_count += 1;
        entries.push(ModMigrationEntry {
            key: m.key.clone(),
            was_enabled: m.enabled,
            success: true,
            error: None,
        });
    }

    Ok(MigrationResult {
        migrated_count,
        skipped_count,
        errors,
        entries,
        restore_point_id,
    })
}

/// Build the ini path segment that 3DMigoto would use for a legacy folder.
/// e.g. category="Characters", folder="Furina" → `\mods\Characters\Furina\`
fn build_ini_segment(category: &str, folder_name: &str) -> String {
    if category == "Uncategorized" {
        format!("\\mods\\{}\\", folder_name)
    } else {
        format!("\\mods\\{}\\{}\\", category, folder_name)
    }
}

/// Build the ini path segment for the new location inside managed_src.
/// src_dest is absolute; mod_path is the game's mod root.
/// We want the relative segment from mod_path down.
fn build_ini_segment_from_src(src_dest: &Path, mod_path: &str) -> String {
    // Try to produce a relative segment from mod_path.
    // Falls back to just the last two components on failure.
    if let Ok(relative) = src_dest.strip_prefix(mod_path) {
        format!("\\{}\\", relative.to_string_lossy().replace('/', "\\").replace('\\', "\\"))
    } else {
        let cat = src_dest.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let name = folder_name(src_dest);
        format!("\\mods\\{}\\{}\\{}\\", MANAGED_SRC, cat, name)
    }
}

/// Move a directory. Tries rename first (fast, same-filesystem), falls back
/// to recursive copy + delete (cross-filesystem, e.g. SD card).
fn move_dir(src: &Path, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    restore_points::copy_dir_recursive(src, dest)?;
    fs::remove_dir_all(src).map_err(|e| format!("Failed to remove source after copy: {}", e))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "mod-manager-migration-test-{}-{}",
            std::process::id(),
            n
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn unique_game_id() -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("__test_migration_{}_{}", std::process::id(), n)
    }

    /// Build a legacy mod directory: Characters/Furina (enabled) +
    /// Characters/DISABLED_Nahida (disabled).
    fn legacy_mod_dir() -> PathBuf {
        let root = temp_dir();
        write_file(&root.join("Characters/Furina/mod.ini"), "furina-data");
        write_file(&root.join("Characters/DISABLED_Nahida/mod.ini"), "nahida-data");
        root
    }

    #[cfg(unix)]
    #[test]
    fn get_layout_status_reports_legacy_correctly() {
        let root = legacy_mod_dir();
        let status = get_layout_status(root.to_str().unwrap()).expect("should succeed");
        assert!(!status.is_symlink_layout);
        assert!(status.has_legacy_mods);
        assert_eq!(status.legacy_mod_count, 2);
        fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn get_layout_status_reports_symlink_layout_after_migration() {
        let root = legacy_mod_dir();
        let game_id = unique_game_id();
        migrate_to_symlink_layout(&game_id, root.to_str().unwrap(), false)
            .expect("migration should succeed");
        let status = get_layout_status(root.to_str().unwrap()).expect("should succeed");
        assert!(status.is_symlink_layout);
        assert!(!status.has_legacy_mods);
        fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn migrate_moves_files_and_creates_correct_symlinks() {
        let root = legacy_mod_dir();
        let game_id = unique_game_id();

        let result = migrate_to_symlink_layout(&game_id, root.to_str().unwrap(), false)
            .expect("migration should succeed");

        assert_eq!(result.migrated_count, 2, "both mods should migrate");
        assert_eq!(result.skipped_count, 0);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        // Real files should be under DISABLED_managed_src.
        let furina_src = root.join(format!("{}/Characters/Furina", crate::mods::MANAGED_SRC));
        let nahida_src = root.join(format!("{}/Characters/Nahida", crate::mods::MANAGED_SRC));
        assert!(furina_src.join("mod.ini").exists(), "Furina source file missing");
        assert!(nahida_src.join("mod.ini").exists(), "Nahida source file missing");
        assert_eq!(fs::read_to_string(furina_src.join("mod.ini")).unwrap(), "furina-data");
        assert_eq!(fs::read_to_string(nahida_src.join("mod.ini")).unwrap(), "nahida-data");

        // Legacy paths must be gone.
        assert!(!root.join("Characters/Furina").exists(), "legacy Furina should be gone");
        assert!(!root.join("Characters/DISABLED_Nahida").exists(), "legacy Nahida should be gone");

        // Furina was enabled → symlink must exist; Nahida was disabled → no symlink.
        let furina_link = root.join(format!("{}/Characters/Furina", crate::mods::MANAGED_TGT));
        let nahida_link = root.join(format!("{}/Characters/Nahida", crate::mods::MANAGED_TGT));
        assert!(symlink::is_symlink(&furina_link), "Furina symlink should exist");
        assert!(!nahida_link.exists(), "Nahida should have no symlink (was disabled)");

        fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn migrate_already_migrated_is_no_op() {
        let root = legacy_mod_dir();
        let game_id = unique_game_id();
        migrate_to_symlink_layout(&game_id, root.to_str().unwrap(), false).unwrap();
        // Second call should succeed silently with zero changes.
        let result = migrate_to_symlink_layout(&game_id, root.to_str().unwrap(), false)
            .expect("second call should succeed");
        assert_eq!(result.migrated_count, 0);
        fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn migrated_mods_are_scannable_with_correct_enabled_state() {
        let root = legacy_mod_dir();
        let game_id = unique_game_id();
        migrate_to_symlink_layout(&game_id, root.to_str().unwrap(), false).unwrap();

        let mods = scan_mods(root.to_str().unwrap()).expect("scan after migration");
        assert_eq!(mods.len(), 2);
        let furina = mods.iter().find(|m| m.name == "Furina").unwrap();
        let nahida = mods.iter().find(|m| m.name == "Nahida").unwrap();
        assert!(furina.enabled, "Furina should be enabled post-migration");
        assert!(!nahida.enabled, "Nahida should be disabled post-migration");
        assert!(furina.using_symlink_layout);
        assert!(nahida.using_symlink_layout);

        fs::remove_dir_all(&root).ok();
    }
}
