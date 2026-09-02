//! Mod scanning, toggling, and deletion for both layout models.
//!
//! ## Layout models
//!
//! ### Legacy (DISABLED_ prefix rename)
//! The original Phase 2–11 model: mod files live directly under `mod_path`,
//! organised as `mod_path/Category/ModName`. Enable/disable is a folder rename
//! that adds/removes the `DISABLED_` prefix. Simple, zero dependencies, but
//! invalidates 3DMigoto's per-mod path-keyed settings in `d3dx_user.ini` on
//! every toggle.
//!
//! ### Symlink layout (Phase 12)
//! Separates file storage from what the mod loader sees:
//!
//! ```text
//! mod_path/
//!   managed_src/           ← real mod files live here, never moved
//!     Characters/
//!       Furina/
//!         mod.ini
//!   managed_tgt/           ← what 3DMigoto reads; contains only symlinks
//!     Characters/
//!       Furina  →  ../../managed_src/Characters/Furina
//! ```
//!
//! Enable = create symlink in `managed_tgt`. Disable = remove symlink.
//! The physical files in `managed_src` never move, so `d3dx_user.ini`'s
//! path-keyed settings (`$\mods\characters\furina\ = 1`) survive unlimited
//! enable/disable cycles.
//!
//! Detection: if `mod_path/managed_src` exists, the symlink layout is active
//! for that game. Both layouts can coexist for different games.

use crate::symlink;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Prefix used to mark a mod folder as disabled in the legacy layout.
pub const DISABLED_PREFIX: &str = "DISABLED_";

/// The subdirectory name under `mod_path` that holds real mod files in the
/// symlink layout. Its presence is the detection signal for the layout.
pub const MANAGED_SRC: &str = "managed_src";

/// The subdirectory name under `mod_path` that holds symlinks in the
/// symlink layout (what 3DMigoto/XXMI actually reads).
pub const MANAGED_TGT: &str = "managed_tgt";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModInfo {
    /// For the **legacy layout**: the absolute path of the mod folder (changes
    /// on toggle since renaming changes the path).
    /// For the **symlink layout**: the absolute path inside `managed_src`
    /// (stable across enable/disable — only the symlink in `managed_tgt` changes).
    pub id: String,
    /// Stable identity across enable/disable: `"<category>/<clean-folder-name>"`.
    /// Same semantics in both layouts. Used by presets, restore points, update tracking.
    pub key: String,
    /// Human-readable display name (prefix stripped, underscores → spaces).
    pub name: String,
    /// Raw folder name on disk. For the legacy layout includes `DISABLED_` when
    /// disabled; for the symlink layout always the clean name (no prefix).
    pub folder_name: String,
    /// Absolute path used for toggle/delete operations (see `id` note above).
    pub path: String,
    /// Category (immediate parent folder name under the layout root), or
    /// `"Uncategorized"` for mods at the top level.
    pub category: String,
    /// Whether the mod is currently active.
    /// Legacy: folder name does NOT start with `DISABLED_`.
    /// Symlink: a symlink exists in `managed_tgt/<category>/<name>`.
    pub enabled: bool,
    /// Absolute path to a preview image inside the mod folder, if present.
    pub preview_path: Option<String>,
    /// Recursive file count (for display metadata).
    pub file_count: u32,
    /// Total folder size in bytes (for display metadata).
    pub size_bytes: u64,
    /// Last-modified time of the mod folder as unix seconds.
    pub modified_at: Option<u64>,
    /// True when this mod is managed under the symlink layout
    /// (`managed_src`/`managed_tgt`), false for the legacy rename layout.
    /// Drives UI decisions (e.g. whether to show the migration prompt).
    pub using_symlink_layout: bool,
}

const PREVIEW_NAMES: &[&str] = &[
    "preview.png",
    "preview.jpg",
    "preview.jpeg",
    "preview.webp",
    "thumbnail.png",
    "thumbnail.jpg",
];

// ---------------------------------------------------------------------------
// Layout detection
// ---------------------------------------------------------------------------

/// Returns `true` when `mod_path/managed_src` exists, indicating the symlink
/// layout is active for this game.
pub fn is_symlink_layout(mod_path: &str) -> bool {
    Path::new(mod_path).join(MANAGED_SRC).is_dir()
}

/// `mod_path/managed_src` — where real mod files live in the symlink layout.
pub fn src_root(mod_path: &str) -> PathBuf {
    Path::new(mod_path).join(MANAGED_SRC)
}

/// `mod_path/managed_tgt` — where symlinks live in the symlink layout.
pub fn tgt_root(mod_path: &str) -> PathBuf {
    Path::new(mod_path).join(MANAGED_TGT)
}

// ---------------------------------------------------------------------------
// Scanning
// ---------------------------------------------------------------------------

/// Scan a game's mod directory and return the list of detected mods.
///
/// Automatically dispatches to the symlink-layout scanner if `managed_src`
/// exists, otherwise uses the legacy `DISABLED_`-prefix scanner.
///
/// Both scanners produce the same `ModInfo` shape so all callers
/// (presets, restore points, conflict detection, …) work transparently.
pub fn scan_mods(mod_path: &str) -> Result<Vec<ModInfo>, String> {
    let root = Path::new(mod_path);
    if !root.exists() {
        return Err(format!("Mod directory does not exist: {}", mod_path));
    }
    if !root.is_dir() {
        return Err(format!("Mod path is not a directory: {}", mod_path));
    }

    if is_symlink_layout(mod_path) {
        scan_symlink_layout(mod_path)
    } else {
        scan_legacy_layout(mod_path)
    }
}

/// Scan the legacy DISABLED_-prefix layout.
fn scan_legacy_layout(mod_path: &str) -> Result<Vec<ModInfo>, String> {
    let root = Path::new(mod_path);
    let mut mods = Vec::new();
    let top_level = read_dirs(root)?;

    for entry in top_level {
        // Skip the managed_src / managed_tgt dirs if they happen to exist
        // alongside legacy mods (shouldn't normally happen, but be safe).
        let name = folder_name(&entry);
        if name == MANAGED_SRC || name == MANAGED_TGT {
            continue;
        }

        let subdirs = read_dirs(&entry)?;
        if subdirs.is_empty() {
            if let Some(info) = build_legacy_mod_info(&entry, "Uncategorized") {
                mods.push(info);
            }
        } else {
            let category = display_name(&folder_name(&entry));
            for mod_dir in subdirs {
                if let Some(info) = build_legacy_mod_info(&mod_dir, &category) {
                    mods.push(info);
                }
            }
        }
    }

    mods.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(mods)
}

/// Scan the symlink layout: read mods from `managed_src`, derive enabled state
/// by checking whether the corresponding symlink exists in `managed_tgt`.
fn scan_symlink_layout(mod_path: &str) -> Result<Vec<ModInfo>, String> {
    let src = src_root(mod_path);
    let tgt = tgt_root(mod_path);

    let mut mods = Vec::new();
    let top_level = read_dirs(&src)?;

    for entry in top_level {
        let subdirs = read_dirs(&entry)?;
        if subdirs.is_empty() {
            // Top-level entry in managed_src with no subfolders → it's a mod
            // directly at the top level, category "Uncategorized".
            let link_path = tgt.join(folder_name(&entry));
            let enabled = symlink::is_symlink(&link_path) || link_path.exists();
            if let Some(info) = build_symlink_mod_info(&entry, "Uncategorized", enabled) {
                mods.push(info);
            }
        } else {
            // Category folder.
            let category = display_name(&folder_name(&entry));
            for mod_dir in subdirs {
                let link_path = tgt.join(folder_name(&entry)).join(folder_name(&mod_dir));
                let enabled = symlink::is_symlink(&link_path) || link_path.exists();
                if let Some(info) = build_symlink_mod_info(&mod_dir, &category, enabled) {
                    mods.push(info);
                }
            }
        }
    }

    mods.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(mods)
}

// ---------------------------------------------------------------------------
// ModInfo construction
// ---------------------------------------------------------------------------

fn build_legacy_mod_info(path: &PathBuf, category: &str) -> Option<ModInfo> {
    let folder = folder_name(path);
    if folder.is_empty() || folder.starts_with('.') {
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
        using_symlink_layout: false,
    })
}

fn build_symlink_mod_info(src_path: &PathBuf, category: &str, enabled: bool) -> Option<ModInfo> {
    let folder = folder_name(src_path);
    if folder.is_empty() || folder.starts_with('.') {
        return None;
    }

    let name = display_name(&folder);
    let preview_path = find_preview(src_path);
    let (file_count, size_bytes) = dir_stats(src_path);
    let modified_at = fs::metadata(src_path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    let key = format!("{}/{}", category, folder);

    Some(ModInfo {
        id: src_path.to_string_lossy().to_string(),
        key,
        name: if name.is_empty() { folder.clone() } else { name },
        folder_name: folder,
        path: src_path.to_string_lossy().to_string(),
        category: category.to_string(),
        enabled,
        preview_path,
        file_count,
        size_bytes,
        modified_at,
        using_symlink_layout: true,
    })
}

// ---------------------------------------------------------------------------
// Toggle — single mod
// ---------------------------------------------------------------------------

/// One mod to toggle in a batch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToggleTarget {
    pub path: String,
    pub category: String,
}

/// Result of a batch toggle operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchToggleResult {
    pub updated: Vec<ModInfo>,
    pub errors: Vec<String>,
}

/// Enable or disable a single mod. Automatically dispatches to the correct
/// mechanism based on which layout is active for the mod's parent directory.
///
/// For the **legacy** layout: renames the folder to add/remove `DISABLED_`.
/// Returns a new `ModInfo` with an updated `id`/`path` (the rename changed them).
///
/// For the **symlink** layout: creates or removes a symlink in `managed_tgt`.
/// Returns a new `ModInfo` with the same `id`/`path` (stable — only the symlink changes).
pub fn set_mod_enabled(path: &str, category: &str, enabled: bool) -> Result<ModInfo, String> {
    let mod_path = Path::new(path);
    if !mod_path.exists() {
        return Err(format!("Mod folder does not exist: {}", path));
    }

    // Detect which layout by checking whether the path sits inside managed_src.
    if is_inside_managed_src(mod_path) {
        set_mod_enabled_symlink(mod_path, category, enabled)
    } else {
        set_mod_enabled_legacy(mod_path, category, enabled)
    }
}

/// Returns true when `path` is rooted inside a `managed_src` directory,
/// i.e. the mod belongs to the symlink layout.
fn is_inside_managed_src(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == MANAGED_SRC)
}

fn set_mod_enabled_legacy(old_path: &Path, category: &str, enabled: bool) -> Result<ModInfo, String> {
    let old_folder = folder_name(old_path);
    let currently_enabled = !old_folder.to_ascii_uppercase().starts_with(DISABLED_PREFIX);

    if currently_enabled == enabled {
        return build_legacy_mod_info(&old_path.to_path_buf(), category)
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

    // After a rename, also migrate d3dx_user.ini keys if the file exists.
    if let Some(grand_parent) = parent.parent() {
        let ini_path = grand_parent.join("d3dx_user.ini");
        if ini_path.exists() {
            let old_key_segment = format!("\\mods\\{}\\{}\\", parent.file_name().and_then(|n| n.to_str()).unwrap_or(""), old_folder).to_lowercase();
            let new_key_segment = format!("\\mods\\{}\\{}\\", parent.file_name().and_then(|n| n.to_str()).unwrap_or(""), new_folder).to_lowercase();
            let _ = migrate_d3dx_ini_keys(&ini_path, &old_key_segment, &new_key_segment);
        }
    }

    build_legacy_mod_info(&new_path, category)
        .ok_or_else(|| "Failed to read mod info after toggle".to_string())
}

fn set_mod_enabled_symlink(src_path: &Path, category: &str, enabled: bool) -> Result<ModInfo, String> {
    // Reconstruct the mod_path by walking up from src_path past managed_src.
    let mod_path = find_mod_path_from_src(src_path)
        .ok_or_else(|| format!("Cannot locate mod root from path: {}", src_path.display()))?;

    let tgt = tgt_root(&mod_path.to_string_lossy());

    // The symlink lives at managed_tgt/<category-folder>/<mod-folder-name>
    // where <category-folder> is the raw folder name of category's parent.
    // For "Uncategorized" mods the symlink is directly under managed_tgt.
    let mod_name = folder_name(src_path);
    let link_path = if category == "Uncategorized" {
        tgt.join(&mod_name)
    } else {
        // The category folder name on disk under managed_src/managed_tgt is
        // derived from the display name in reverse — but since we stored it
        // as the raw folder name, we can find it via the src parent.
        let src_cat_dir = src_path
            .parent()
            .ok_or_else(|| "Mod folder has no parent in managed_src".to_string())?;
        let cat_folder = folder_name(src_cat_dir);
        tgt.join(cat_folder).join(&mod_name)
    };

    let currently_enabled = symlink::is_symlink(&link_path) || link_path.exists();
    if currently_enabled == enabled {
        // Nothing to do — return current state.
        return build_symlink_mod_info(&src_path.to_path_buf(), category, enabled)
            .ok_or_else(|| "Failed to read mod info".to_string());
    }

    if enabled {
        // Create symlink: managed_tgt/<cat>/<name> → managed_src/<cat>/<name>
        if let Some(parent) = link_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create tgt category dir: {}", e))?;
        }
        symlink::create_dir_symlink(src_path, &link_path)?;
    } else {
        symlink::remove_symlink(&link_path)?;
    }

    build_symlink_mod_info(&src_path.to_path_buf(), category, enabled)
        .ok_or_else(|| "Failed to read mod info after toggle".to_string())
}

/// Walk up from a path inside `managed_src` to find the `mod_path` root.
/// e.g. `/mods/managed_src/Characters/Furina` → `/mods`
fn find_mod_path_from_src(path: &Path) -> Option<PathBuf> {
    let mut current = path;
    loop {
        if let Some(parent) = current.parent() {
            if folder_name(parent) == MANAGED_SRC {
                return parent.parent().map(|p| p.to_path_buf());
            }
            if folder_name(current) == MANAGED_SRC {
                return current.parent().map(|p| p.to_path_buf());
            }
            current = parent;
        } else {
            return None;
        }
    }
}

// ---------------------------------------------------------------------------
// Batch toggle
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Delete
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeleteResult {
    pub deleted_keys: Vec<String>,
    pub errors: Vec<String>,
}

/// Delete a mod permanently. For the symlink layout this deletes both the
/// symlink in `managed_tgt` (if present) AND the source files in `managed_src`.
/// Includes a path-safety check in both cases.
pub fn delete_mod(path: &str, mod_root: &str) -> Result<String, String> {
    let mod_path = Path::new(path);

    if is_inside_managed_src(mod_path) {
        delete_symlink_mod(mod_path, mod_root)
    } else {
        delete_legacy_mod(mod_path, mod_root)
    }
}

fn delete_legacy_mod(mod_path: &Path, mod_root: &str) -> Result<String, String> {
    let root = Path::new(mod_root);
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
        return Err(format!("Mod folder does not exist: {}", mod_path.display()));
    }
    if !mod_path.is_dir() {
        return Err(format!("Path is not a directory: {}", mod_path.display()));
    }

    let folder = folder_name(mod_path);
    let stripped = strip_disabled_prefix(&folder);
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

fn delete_symlink_mod(src_path: &Path, _mod_root: &str) -> Result<String, String> {
    // Safety check: must be inside managed_src.
    if !is_inside_managed_src(src_path) {
        return Err("Refusing to delete: path is not inside managed_src".to_string());
    }
    if !src_path.exists() {
        return Err(format!("Mod folder does not exist: {}", src_path.display()));
    }

    let mod_root = find_mod_path_from_src(src_path)
        .ok_or_else(|| format!("Cannot locate mod root from path: {}", src_path.display()))?;

    let folder = folder_name(src_path);
    let src_parent = src_path.parent().ok_or("No parent in managed_src")?;
    let cat_folder = folder_name(src_parent);
    let category = if cat_folder == MANAGED_SRC {
        "Uncategorized".to_string()
    } else {
        display_name(&cat_folder)
    };
    let key = format!("{}/{}", category, folder);

    // Remove the symlink in managed_tgt first (if it exists), then the source.
    let tgt = tgt_root(&mod_root.to_string_lossy());
    let link_path = if category == "Uncategorized" {
        tgt.join(&folder)
    } else {
        tgt.join(&cat_folder).join(&folder)
    };
    if symlink::is_symlink(&link_path) || link_path.exists() {
        symlink::remove_symlink(&link_path)?;
    }

    fs::remove_dir_all(src_path)
        .map_err(|e| format!("Failed to delete source '{}': {}", folder, e))?;
    Ok(key)
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

// ---------------------------------------------------------------------------
// d3dx_user.ini key migration (task #4)
// ---------------------------------------------------------------------------

/// Migrate path-keyed settings in a 3DMigoto `d3dx_user.ini` file when a mod
/// folder is renamed or moved. This happens automatically during legacy toggles
/// (DISABLED_ rename) and during the one-time conversion to symlink layout.
///
/// The ini file uses lines like:
/// ```text
/// $\mods\characters\furina\ = 1
/// ```
/// When `old_segment` appears (case-insensitive) in a key, it is replaced with
/// `new_segment`. A backup is written to `d3dx_user_pre_imm.ini.bak` before any
/// change, mirroring IMM's safety step.
///
/// Returns `Ok(n)` where `n` is the number of lines rewritten (0 = no-op).
pub fn migrate_d3dx_ini_keys(ini_path: &Path, old_segment: &str, new_segment: &str) -> Result<usize, String> {
    if old_segment == new_segment {
        return Ok(0);
    }
    let data = fs::read_to_string(ini_path)
        .map_err(|e| format!("Failed to read {}: {}", ini_path.display(), e))?;

    let mut changed = 0usize;
    let new_data: String = data
        .lines()
        .map(|line| {
            // Only rewrite lines that look like key-value pairs (contain '=')
            // and whose left-hand side contains the old segment.
            if line.contains('=') {
                let lower = line.to_lowercase();
                if lower.starts_with('$') && lower.contains(&old_segment.to_lowercase()) {
                    changed += 1;
                    return line.replace(old_segment, new_segment);
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");

    if changed == 0 {
        return Ok(0);
    }

    // Write backup before modifying.
    let bak_path = ini_path.with_extension("ini.bak");
    fs::write(&bak_path, &data)
        .map_err(|e| format!("Failed to write backup {}: {}", bak_path.display(), e))?;

    // Write updated file.
    fs::write(ini_path, new_data)
        .map_err(|e| format!("Failed to write updated ini {}: {}", ini_path.display(), e))?;

    Ok(changed)
}

// ---------------------------------------------------------------------------
// Preview + stats helpers (shared between layouts)
// ---------------------------------------------------------------------------

fn find_preview(dir: &Path) -> Option<String> {
    for name in PREVIEW_NAMES {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

/// Whether `dir` already contains a recognized preview image file.
pub fn has_preview_image(dir: &Path) -> bool {
    find_preview(dir).is_some()
}

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
            if path.is_dir() && !symlink::is_symlink(&path) {
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

// ---------------------------------------------------------------------------
// Shared filesystem helpers
// ---------------------------------------------------------------------------

pub(crate) fn read_dirs(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read {}: {}", dir.display(), e))?;
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        // For managed_src we use real directories. For managed_tgt symlinks
        // read as directories on most platforms; we still skip dot-prefixed names.
        if path.is_dir() || symlink::is_symlink(&path) {
            let name = folder_name(&path);
            if !name.starts_with('.') {
                dirs.push(path);
            }
        }
    }
    Ok(dirs)
}

pub(crate) fn folder_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Strip the `DISABLED_` prefix (case-insensitive).
///
/// Uses `to_ascii_uppercase` on the prefix-length char slice rather than
/// raw byte indexing, which would panic if a multi-byte UTF-8 character
/// (e.g. a CJK character in a mod folder name) straddles the byte boundary.
pub fn strip_disabled_prefix(name: &str) -> &str {
    // DISABLED_PREFIX is pure ASCII so its byte length == its char length.
    // We collect only that many *chars* from `name` to avoid slicing mid-char.
    let prefix_chars = DISABLED_PREFIX.len(); // 9 — all ASCII bytes
    let char_boundary: Option<usize> = name.char_indices().nth(prefix_chars).map(|(i, _)| i);
    if let Some(end) = char_boundary {
        if name[..end].eq_ignore_ascii_case(DISABLED_PREFIX) {
            return &name[end..];
        }
    }
    name
}

/// Human-readable display name from a raw folder name: strip prefix + convert
/// underscores/dashes to spaces.
pub fn display_name(folder_name: &str) -> String {
    let stripped = strip_disabled_prefix(folder_name);
    stripped.replace(['_', '-'], " ").trim().to_string()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_fixture_dir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "mod-manager-mods-test-{}-{}",
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

    // ------------------------------------------------------------------
    // Legacy layout tests (unchanged behaviour from Phases 2–11)
    // ------------------------------------------------------------------

    #[test]
    fn scans_categorized_and_uncategorized_mods() {
        let root = temp_fixture_dir();
        write_file(&root.join("Characters/Furina/mod.ini"), "test");
        write_file(&root.join("Characters/Furina/preview.png"), "fake-png");
        write_file(&root.join("Characters/DISABLED_Nahida/mod.ini"), "test");
        write_file(&root.join("UI/some_file.ini"), "test");
        write_file(&root.join(".git/config"), "test"); // must be skipped

        let mods = scan_mods(root.to_str().unwrap()).expect("scan should succeed");
        assert_eq!(mods.len(), 3, "got {:?}", mods);

        let furina = mods.iter().find(|m| m.name == "Furina").unwrap();
        assert_eq!(furina.category, "Characters");
        assert!(furina.enabled);
        assert!(furina.preview_path.is_some());
        assert!(!furina.using_symlink_layout);

        let nahida = mods.iter().find(|m| m.name == "Nahida").unwrap();
        assert!(!nahida.enabled);
        assert_eq!(nahida.folder_name, "DISABLED_Nahida");

        let ui = mods.iter().find(|m| m.name == "UI").unwrap();
        assert_eq!(ui.category, "Uncategorized");

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn errors_on_missing_directory() {
        assert!(scan_mods("/definitely/does/not/exist/mod-manager-test").is_err());
    }

    #[test]
    fn strips_disabled_prefix_case_insensitively() {
        assert_eq!(strip_disabled_prefix("disabled_Foo"), "Foo");
        assert_eq!(strip_disabled_prefix("DISABLED_Foo"), "Foo");
        assert_eq!(strip_disabled_prefix("Foo"), "Foo");
    }

    #[test]
    fn strip_disabled_prefix_safe_with_multibyte_utf8() {
        // '丨' is a 3-byte UTF-8 character. A naive byte-index slice of
        // DISABLED_PREFIX.len() (9) into this string would panic because byte 9
        // falls inside the second '丨' (bytes 9..12). This must not panic.
        let name = "丨丨丨some_mod";
        assert_eq!(strip_disabled_prefix(name), name); // no prefix → unchanged
        // A mod folder name that starts with multi-byte chars and is shorter
        // than DISABLED_PREFIX.len() in bytes must also be safe.
        assert_eq!(strip_disabled_prefix("丨"), "丨");
    }

    #[test]
    fn display_name_replaces_separators() {
        assert_eq!(display_name("Foo_Bar-Baz"), "Foo Bar Baz");
        assert_eq!(display_name("DISABLED_Foo_Bar"), "Foo Bar");
    }

    #[test]
    fn legacy_toggle_disables_and_re_enables() {
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
    fn legacy_toggle_no_op_when_already_in_target_state() {
        let root = temp_fixture_dir();
        write_file(&root.join("Furina/mod.ini"), "test");
        let result = set_mod_enabled(root.join("Furina").to_str().unwrap(), "Uncategorized", true)
            .expect("no-op enable should succeed");
        assert!(result.enabled);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn legacy_toggle_errors_when_target_folder_exists() {
        let root = temp_fixture_dir();
        write_file(&root.join("Furina/mod.ini"), "test");
        write_file(&root.join("DISABLED_Furina/mod.ini"), "test");
        let result = set_mod_enabled(root.join("Furina").to_str().unwrap(), "Uncategorized", false);
        assert!(result.is_err());
        assert!(root.join("Furina").exists());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn delete_mod_removes_folder_and_returns_key() {
        let root = temp_fixture_dir();
        write_file(&root.join("Characters/Furina/mod.ini"), "test");
        let key = delete_mod(
            root.join("Characters/Furina").to_str().unwrap(),
            root.to_str().unwrap(),
        ).expect("delete should succeed");
        assert_eq!(key, "Characters/Furina");
        assert!(!root.join("Characters/Furina").exists());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn delete_mod_refuses_path_outside_mod_root() {
        let root = temp_fixture_dir();
        let other = temp_fixture_dir();
        write_file(&other.join("evil/mod.ini"), "data");
        let result = delete_mod(other.join("evil").to_str().unwrap(), root.to_str().unwrap());
        assert!(result.is_err());
        assert!(other.join("evil/mod.ini").exists());
        fs::remove_dir_all(&root).ok();
        fs::remove_dir_all(&other).ok();
    }

    // ------------------------------------------------------------------
    // Symlink layout tests
    // ------------------------------------------------------------------

    #[cfg(unix)]
    fn setup_symlink_layout(root: &Path) {
        // Creates: root/managed_src/Characters/Furina/mod.ini
        //          root/managed_src/Characters/Nahida/mod.ini
        write_file(&root.join("managed_src/Characters/Furina/mod.ini"), "test");
        write_file(&root.join("managed_src/Characters/Furina/preview.png"), "img");
        write_file(&root.join("managed_src/Characters/Nahida/mod.ini"), "test");
        // Furina enabled (symlink exists), Nahida disabled (no symlink)
        fs::create_dir_all(root.join("managed_tgt/Characters")).unwrap();
        symlink::create_dir_symlink(
            &root.join("managed_src/Characters/Furina"),
            &root.join("managed_tgt/Characters/Furina"),
        ).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_layout_detected_by_managed_src() {
        let root = temp_fixture_dir();
        fs::create_dir_all(root.join(MANAGED_SRC)).unwrap();
        assert!(is_symlink_layout(root.to_str().unwrap()));
        fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn scans_symlink_layout_correctly() {
        let root = temp_fixture_dir();
        setup_symlink_layout(&root);

        let mods = scan_mods(root.to_str().unwrap()).expect("scan should succeed");
        assert_eq!(mods.len(), 2, "got {:?}", mods);

        let furina = mods.iter().find(|m| m.name == "Furina").unwrap();
        assert!(furina.enabled, "Furina has a symlink so should be enabled");
        assert!(furina.using_symlink_layout);
        assert!(furina.preview_path.is_some());
        assert_eq!(furina.category, "Characters");
        assert_eq!(furina.key, "Characters/Furina");

        let nahida = mods.iter().find(|m| m.name == "Nahida").unwrap();
        assert!(!nahida.enabled, "Nahida has no symlink so should be disabled");
        assert!(nahida.using_symlink_layout);
        // id/path should point into managed_src (stable)
        assert!(nahida.path.contains("managed_src"), "path should be in managed_src");

        fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_toggle_enable_creates_symlink() {
        let root = temp_fixture_dir();
        setup_symlink_layout(&root);

        let nahida_src = root.join("managed_src/Characters/Nahida");
        let result = set_mod_enabled(nahida_src.to_str().unwrap(), "Characters", true)
            .expect("enable should succeed");
        assert!(result.enabled);
        assert!(result.using_symlink_layout);
        // The id/path must NOT have changed (stable in symlink layout)
        assert_eq!(result.path, nahida_src.to_string_lossy());

        let link = root.join("managed_tgt/Characters/Nahida");
        assert!(symlink::is_symlink(&link), "symlink should now exist");

        fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_toggle_disable_removes_symlink() {
        let root = temp_fixture_dir();
        setup_symlink_layout(&root);

        let furina_src = root.join("managed_src/Characters/Furina");
        let result = set_mod_enabled(furina_src.to_str().unwrap(), "Characters", false)
            .expect("disable should succeed");
        assert!(!result.enabled);

        let link = root.join("managed_tgt/Characters/Furina");
        assert!(!symlink::is_symlink(&link), "symlink should be removed");
        // Real files must be untouched
        assert!(furina_src.join("mod.ini").exists(), "source files must survive");

        fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_toggle_no_op_when_already_in_target_state() {
        let root = temp_fixture_dir();
        setup_symlink_layout(&root);

        let furina_src = root.join("managed_src/Characters/Furina");
        // Furina is already enabled; enabling again should not error.
        let result = set_mod_enabled(furina_src.to_str().unwrap(), "Characters", true)
            .expect("no-op should succeed");
        assert!(result.enabled);

        fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_delete_removes_link_and_source() {
        let root = temp_fixture_dir();
        setup_symlink_layout(&root);

        let furina_src = root.join("managed_src/Characters/Furina");
        let key = delete_mod(furina_src.to_str().unwrap(), root.to_str().unwrap())
            .expect("delete should succeed");
        assert_eq!(key, "Characters/Furina");
        assert!(!furina_src.exists(), "source files should be deleted");
        let link = root.join("managed_tgt/Characters/Furina");
        assert!(!link.exists(), "symlink should also be removed");

        fs::remove_dir_all(&root).ok();
    }

    // ------------------------------------------------------------------
    // d3dx_user.ini migration test
    // ------------------------------------------------------------------

    #[test]
    fn d3dx_ini_migration_rewrites_matching_keys_and_writes_backup() {
        let root = temp_fixture_dir();
        let ini_path = root.join("d3dx_user.ini");
        write_file(
            &ini_path,
            "$\\mods\\characters\\furina\\ = 1\n\
             $\\mods\\characters\\DISABLED_Furina\\ = 0\n\
             ; some comment\n\
             $\\other_key\\ = 5\n",
        );

        let n = migrate_d3dx_ini_keys(
            &ini_path,
            "\\mods\\characters\\furina\\",
            "\\mods\\characters\\DISABLED_Furina\\",
        ).expect("migration should succeed");
        assert_eq!(n, 1, "only the matching key-value line should change");

        let bak_path = ini_path.with_extension("ini.bak");
        assert!(bak_path.exists(), "backup must be written");

        let updated = fs::read_to_string(&ini_path).unwrap();
        assert!(updated.contains("\\mods\\characters\\DISABLED_Furina\\"), "key should be rewritten");

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn d3dx_ini_migration_no_op_when_no_match() {
        let root = temp_fixture_dir();
        let ini_path = root.join("d3dx_user.ini");
        write_file(&ini_path, "$\\mods\\characters\\nahida\\ = 1\n");
        let n = migrate_d3dx_ini_keys(
            &ini_path,
            "\\mods\\characters\\furina\\",
            "\\mods\\characters\\DISABLED_Furina\\",
        ).unwrap();
        assert_eq!(n, 0);
        // No backup should be created when nothing changed.
        assert!(!ini_path.with_extension("ini.bak").exists());
        fs::remove_dir_all(&root).ok();
    }
}
