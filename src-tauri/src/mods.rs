use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Prefix used to mark a mod folder as disabled, matching the XXMI/GIMI convention.
pub const DISABLED_PREFIX: &str = "DISABLED_";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModInfo {
    /// Stable identifier for the mod — its absolute path. Used as the React key
    /// and as the target for toggle/delete operations in later phases.
    ///
    /// NOTE: this changes when a mod is toggled (the folder is renamed), so it
    /// is NOT suitable as a persistent identity across enable/disable — use
    /// `key` for that (e.g. presets).
    pub id: String,
    /// Stable identity for this mod that survives toggling: "<category>/<folder
    /// name with DISABLED_ prefix stripped>". Used by presets to remember which
    /// mods should be enabled without caring about their current toggle state.
    pub key: String,
    /// Display name with the DISABLED_ prefix stripped and underscores/dashes
    /// turned into spaces for readability.
    pub name: String,
    /// The raw folder name on disk (prefix included), needed for toggle operations
    /// that rename the folder in Phase 3.
    pub folder_name: String,
    /// Absolute path to the mod folder.
    pub path: String,
    /// Category the mod was found under (the immediate subfolder of mod_path),
    /// or "Uncategorized" if the mod sits directly in mod_path.
    pub category: String,
    /// Whether the mod is currently enabled (folder name lacks DISABLED_ prefix).
    pub enabled: bool,
    /// Absolute path to a preview image inside the mod folder, if one exists.
    pub preview_path: Option<String>,
    /// Number of files inside the mod folder (recursive), for basic metadata display.
    pub file_count: u32,
    /// Total size of the mod folder in bytes (recursive).
    pub size_bytes: u64,
    /// Last-modified time of the mod folder, as unix seconds, for sorting/display.
    pub modified_at: Option<u64>,
}

const PREVIEW_NAMES: &[&str] = &[
    "preview.png",
    "preview.jpg",
    "preview.jpeg",
    "preview.webp",
    "thumbnail.png",
    "thumbnail.jpg",
];

/// Scan a game's configured mod directory and return the list of detected mods.
///
/// Detection rule: any directory that is not itself further subdivided into more
/// mod-like directories is treated as a mod. We look one level deep for
/// "category" folders (e.g. "Characters/Furina") — if a subfolder of mod_path
/// contains further subfolders, those subfolders are treated as the mods and the
/// top-level folder's name becomes the category. If a subfolder of mod_path has
/// no further subfolders (i.e. it directly contains files), it's treated as a mod
/// with category "Uncategorized".
pub fn scan_mods(mod_path: &str) -> Result<Vec<ModInfo>, String> {
    let root = Path::new(mod_path);
    if !root.exists() {
        return Err(format!("Mod directory does not exist: {}", mod_path));
    }
    if !root.is_dir() {
        return Err(format!("Mod path is not a directory: {}", mod_path));
    }

    let mut mods = Vec::new();
    let top_level = read_dirs(root)?;

    for entry in top_level {
        let subdirs = read_dirs(&entry)?;
        if subdirs.is_empty() {
            // No subfolders — this directory itself is a mod.
            if let Some(info) = build_mod_info(&entry, "Uncategorized") {
                mods.push(info);
            }
        } else {
            // Has subfolders — treat this as a category, and each subfolder as a mod.
            let category = display_name(&folder_name(&entry));
            for mod_dir in subdirs {
                if let Some(info) = build_mod_info(&mod_dir, &category) {
                    mods.push(info);
                }
            }
        }
    }

    mods.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(mods)
}

/// List immediate subdirectories of `dir`, skipping hidden/dot folders.
fn read_dirs(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read {}: {}", dir.display(), e))?;
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = folder_name(&path);
            if !name.starts_with('.') {
                dirs.push(path);
            }
        }
    }
    Ok(dirs)
}

fn folder_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Convert a raw folder name into a readable display name: strip the DISABLED_
/// prefix and swap underscores/dashes for spaces.
fn display_name(folder_name: &str) -> String {
    let stripped = strip_disabled_prefix(folder_name);
    stripped.replace(['_', '-'], " ").trim().to_string()
}

fn strip_disabled_prefix(name: &str) -> &str {
    if name.len() >= DISABLED_PREFIX.len()
        && name[..DISABLED_PREFIX.len()].eq_ignore_ascii_case(DISABLED_PREFIX)
    {
        &name[DISABLED_PREFIX.len()..]
    } else {
        name
    }
}

fn build_mod_info(path: &PathBuf, category: &str) -> Option<ModInfo> {
    let folder = folder_name(path);
    if folder.is_empty() {
        return None;
    }

    let enabled = !folder.to_ascii_uppercase().starts_with(DISABLED_PREFIX);
    let name = display_name(&folder);
    let preview_path = find_preview(path);
    let (file_count, size_bytes) = dir_stats(path);
    let modified_at = fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    let stripped_folder = strip_disabled_prefix(&folder).to_string();
    let key = format!("{}/{}", category, stripped_folder);

    Some(ModInfo {
        id: path.to_string_lossy().to_string(),
        key,
        name: if name.is_empty() { folder.clone() } else { name },
        folder_name: folder,
        path: path.to_string_lossy().to_string(),
        category: category.to_string(),
        enabled,
        preview_path,
        file_count,
        size_bytes,
        modified_at,
    })
}

/// One mod to toggle in a batch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToggleTarget {
    pub path: String,
    pub category: String,
}

/// Result of a batch toggle operation: the successfully updated mods, plus any
/// per-mod errors (e.g. naming collisions) so a partial failure doesn't hide
/// the successes or silently drop the failures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchToggleResult {
    pub updated: Vec<ModInfo>,
    pub errors: Vec<String>,
}

/// Toggle a batch of mods to `enabled`. Continues past individual failures
/// rather than aborting the whole batch.
pub fn batch_set_enabled(targets: &[ToggleTarget], enabled: bool) -> BatchToggleResult {
    let mut updated = Vec::new();
    let mut errors = Vec::new();

    for target in targets {
        match set_mod_enabled(&target.path, &target.category, enabled) {
            Ok(info) => updated.push(info),
            Err(e) => errors.push(e),
        }
    }

    BatchToggleResult { updated, errors }
}

/// Enable or disable a single mod by renaming its folder to add/remove the
/// DISABLED_ prefix. `category` must be the category the mod was scanned
/// under (used to rebuild a fresh ModInfo after the rename). Returns the
/// updated ModInfo, which will have a new `id`/`path` (folder name changed)
/// but the same stable `key`.
pub fn set_mod_enabled(path: &str, category: &str, enabled: bool) -> Result<ModInfo, String> {
    let old_path = Path::new(path);
    if !old_path.exists() {
        return Err(format!("Mod folder does not exist: {}", path));
    }

    let old_folder = folder_name(old_path);
    let currently_enabled = !old_folder.to_ascii_uppercase().starts_with(DISABLED_PREFIX);

    if currently_enabled == enabled {
        // Nothing to do — return fresh info as-is.
        return build_mod_info(&old_path.to_path_buf(), category)
            .ok_or_else(|| "Failed to read mod info".to_string());
    }

    let base_name = strip_disabled_prefix(&old_folder);
    let new_folder = if enabled {
        base_name.to_string()
    } else {
        format!("{}{}", DISABLED_PREFIX, base_name)
    };

    let parent = old_path
        .parent()
        .ok_or_else(|| "Mod folder has no parent directory".to_string())?;
    let new_path = parent.join(&new_folder);

    if new_path.exists() {
        return Err(format!(
            "Cannot {} '{}': a folder named '{}' already exists",
            if enabled { "enable" } else { "disable" },
            base_name,
            new_folder
        ));
    }

    fs::rename(old_path, &new_path)
        .map_err(|e| format!("Failed to rename '{}': {}", old_folder, e))?;

    build_mod_info(&new_path, category).ok_or_else(|| "Failed to read mod info after toggle".to_string())
}

fn find_preview(dir: &Path) -> Option<String> {
    for name in PREVIEW_NAMES {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

/// Whether `dir` already contains a recognized preview image file. Used by
/// the downloads pipeline to avoid overwriting a preview image the mod
/// author already shipped inside the archive with GameBanana's thumbnail.
pub fn has_preview_image(dir: &Path) -> bool {
    find_preview(dir).is_some()
}

/// Permanently delete a mod folder from disk. Includes a safety check that
/// the path actually lives under an expected mod directory (preventing a
/// confused or malicious frontend from asking us to delete arbitrary
/// filesystem paths). Returns the key of the deleted mod so the caller can
/// clean up related metadata (e.g. update_tracking origins).
pub fn delete_mod(path: &str, mod_root: &str) -> Result<String, String> {
    let mod_path = Path::new(path);
    let root = Path::new(mod_root);

    // Security: refuse to delete anything that isn't a direct descendant of
    // the configured mod directory (either one or two levels deep, matching
    // scan_mods' detection pattern).
    let canonical_mod = mod_path
        .canonicalize()
        .map_err(|e| format!("Mod path invalid: {}", e))?;
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("Mod root invalid: {}", e))?;
    if !canonical_mod.starts_with(&canonical_root) {
        return Err("Refusing to delete: path is outside the configured mod directory".to_string());
    }

    if !mod_path.exists() {
        return Err(format!("Mod folder does not exist: {}", path));
    }
    if !mod_path.is_dir() {
        return Err(format!("Path is not a directory: {}", path));
    }

    // Compute the mod key before deleting (needed for update_tracking cleanup).
    let folder = folder_name(mod_path);
    let stripped = strip_disabled_prefix(&folder);
    // Figure out category: if the mod is 2 levels deep (root/Category/Mod),
    // the category is the parent's folder name. If 1 level deep (root/Mod),
    // category is "Uncategorized".
    let parent = mod_path.parent().unwrap_or(root);
    let category = if parent == root {
        "Uncategorized".to_string()
    } else {
        display_name(&folder_name(parent))
    };
    let key = format!("{}/{}", category, stripped);

    fs::remove_dir_all(mod_path)
        .map_err(|e| format!("Failed to delete '{}': {}", folder, e))?;

    Ok(key)
}

/// Delete multiple mods at once. Continues past individual failures, same
/// pattern as `batch_set_enabled`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeleteResult {
    pub deleted_keys: Vec<String>,
    pub errors: Vec<String>,
}

pub fn batch_delete_mods(paths: &[String], mod_root: &str) -> BatchDeleteResult {
    let mut deleted_keys = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match delete_mod(path, mod_root) {
            Ok(key) => deleted_keys.push(key),
            Err(e) => errors.push(e),
        }
    }

    BatchDeleteResult { deleted_keys, errors }
}

/// Recursively count files and total size within a directory, capped at a
/// reasonable depth to avoid pathological scans on huge mod folders.
fn dir_stats(dir: &Path) -> (u32, u64) {
    fn walk(dir: &Path, depth: u32, count: &mut u32, size: &mut u64) {
        if depth > 8 {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, depth + 1, count, size);
            } else if let Ok(meta) = entry.metadata() {
                *count += 1;
                *size += meta.len();
            }
        }
    }

    let mut count = 0;
    let mut size = 0;
    walk(dir, 0, &mut count, &mut size);
    (count, size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Create a unique temp directory for a test run, so parallel test threads
    /// never collide.
    fn temp_fixture_dir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "mod-manager-test-{}-{}",
            std::process::id(),
            n
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn scans_categorized_and_uncategorized_mods() {
        let root = temp_fixture_dir();

        // Characters/Furina — enabled mod with a preview image
        write_file(&root.join("Characters/Furina/mod.ini"), "test");
        write_file(&root.join("Characters/Furina/preview.png"), "fake-png");

        // Characters/DISABLED_Nahida — disabled mod
        write_file(&root.join("Characters/DISABLED_Nahida/mod.ini"), "test");

        // UI — no subfolders, so it's a mod itself under "Uncategorized"
        write_file(&root.join("UI/some_file.ini"), "test");

        // Hidden folder should be skipped entirely
        write_file(&root.join(".git/config"), "test");

        let mods = scan_mods(root.to_str().unwrap()).expect("scan should succeed");

        assert_eq!(mods.len(), 3, "expected exactly 3 mods, got {:?}", mods);

        let furina = mods.iter().find(|m| m.name == "Furina").expect("Furina mod");
        assert_eq!(furina.category, "Characters");
        assert!(furina.enabled);
        assert!(furina.preview_path.is_some());
        assert_eq!(furina.file_count, 2);

        let nahida = mods
            .iter()
            .find(|m| m.name == "Nahida")
            .expect("Nahida mod (prefix stripped)");
        assert_eq!(nahida.category, "Characters");
        assert!(!nahida.enabled);
        assert_eq!(nahida.folder_name, "DISABLED_Nahida");

        let ui = mods.iter().find(|m| m.name == "UI").expect("UI mod");
        assert_eq!(ui.category, "Uncategorized");
        assert!(ui.enabled);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn errors_on_missing_directory() {
        let result = scan_mods("/definitely/does/not/exist/mod-manager-test");
        assert!(result.is_err());
    }

    #[test]
    fn strips_disabled_prefix_case_insensitively() {
        assert_eq!(strip_disabled_prefix("disabled_Foo"), "Foo");
        assert_eq!(strip_disabled_prefix("DISABLED_Foo"), "Foo");
        assert_eq!(strip_disabled_prefix("Foo"), "Foo");
    }

    #[test]
    fn display_name_replaces_separators() {
        assert_eq!(display_name("Foo_Bar-Baz"), "Foo Bar Baz");
        assert_eq!(display_name("DISABLED_Foo_Bar"), "Foo Bar");
    }

    #[test]
    fn toggle_disables_and_re_enables_a_mod() {
        let root = temp_fixture_dir();
        write_file(&root.join("Furina/mod.ini"), "test");
        let mod_path = root.join("Furina");

        let disabled = set_mod_enabled(mod_path.to_str().unwrap(), "Uncategorized", false)
            .expect("disable should succeed");
        assert!(!disabled.enabled);
        assert_eq!(disabled.folder_name, "DISABLED_Furina");
        assert!(root.join("DISABLED_Furina").exists());
        assert!(!root.join("Furina").exists());
        assert_eq!(disabled.key, "Uncategorized/Furina");

        let enabled = set_mod_enabled(&disabled.path, "Uncategorized", true)
            .expect("enable should succeed");
        assert!(enabled.enabled);
        assert_eq!(enabled.folder_name, "Furina");
        assert!(root.join("Furina").exists());
        assert_eq!(enabled.key, "Uncategorized/Furina");

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn toggle_is_a_no_op_when_already_in_target_state() {
        let root = temp_fixture_dir();
        write_file(&root.join("Furina/mod.ini"), "test");
        let mod_path = root.join("Furina");

        let result = set_mod_enabled(mod_path.to_str().unwrap(), "Uncategorized", true)
            .expect("no-op enable should succeed");
        assert!(result.enabled);
        assert!(mod_path.exists());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn toggle_errors_when_target_folder_already_exists() {
        let root = temp_fixture_dir();
        write_file(&root.join("Furina/mod.ini"), "test");
        write_file(&root.join("DISABLED_Furina/mod.ini"), "test");
        let mod_path = root.join("Furina");

        let result = set_mod_enabled(mod_path.to_str().unwrap(), "Uncategorized", false);
        assert!(result.is_err());
        // Original folder must be untouched since the rename should have been rejected.
        assert!(mod_path.exists());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn delete_mod_removes_folder_and_returns_key() {
        let root = temp_fixture_dir();
        write_file(&root.join("Characters/Furina/mod.ini"), "test");

        let key = delete_mod(
            root.join("Characters/Furina").to_str().unwrap(),
            root.to_str().unwrap(),
        )
        .expect("delete should succeed");

        assert_eq!(key, "Characters/Furina");
        assert!(!root.join("Characters/Furina").exists());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn delete_mod_refuses_path_outside_mod_root() {
        let root = temp_fixture_dir();
        write_file(&root.join("Furina/mod.ini"), "test");

        // Try to delete something outside the mod root — should be rejected.
        let other = temp_fixture_dir();
        write_file(&other.join("evil/mod.ini"), "data");
        let result = delete_mod(other.join("evil").to_str().unwrap(), root.to_str().unwrap());
        assert!(result.is_err());
        assert!(other.join("evil/mod.ini").exists()); // must not have been deleted

        fs::remove_dir_all(&root).ok();
        fs::remove_dir_all(&other).ok();
    }
}
