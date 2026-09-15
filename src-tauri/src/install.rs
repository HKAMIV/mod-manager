//! Manual mod installation (Phase 12 follow-up).
//!
//! Provides two entry points:
//!
//! - `install_mod_from_folder`: copy/move an existing folder on disk into the
//!   game's mod directory (legacy or symlink layout).
//! - `install_mod_from_archive`: extract a .zip/.7z/.rar archive and install
//!   the content as a mod, using the same wrapper-folder descent logic the
//!   download pipeline uses so single-extra-folder archives Just Work.
//!
//! Both functions:
//!   - Accept a `category` string — empty means "Uncategorized".
//!   - Accept a `mod_name` override — empty means "use the source folder name".
//!   - Respect the current layout (legacy or symlink) of the game's mod dir.
//!   - Return the newly created `ModInfo` so the frontend can optimistically
//!     add it without a full rescan.
//!   - Reject destination collisions rather than silently overwriting, so the
//!     caller can surface a "name already exists" error and let the user rename.

use crate::downloads::{extract_archive, find_content_root, move_dir, sanitize_component};
use crate::mods::{self, build_mod_info_at, ModInfo};
use crate::restore_points::copy_dir_recursive;
use crate::state::AppState;
use std::fs;
use std::path::{Path, PathBuf};

/// Result returned from both install entry points.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InstallResult {
    /// The newly installed mod.
    pub mod_info: ModInfo,
    /// Absolute path where the mod files now live (inside managed_src for
    /// symlink layout, directly in the category folder for legacy).
    pub installed_path: String,
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Install a mod by copying an existing folder on disk into the game's mod
/// directory. The source folder is **copied** (not moved) so the user's
/// original is left intact.
///
/// # Arguments
/// * `source_path` — absolute path to the folder to install.
/// * `mod_path`    — the game's configured mod directory.
/// * `category`    — category subfolder name. Pass `""` for Uncategorized.
/// * `mod_name`    — display/folder name to use. Pass `""` to derive from source.
pub fn install_mod_from_folder(
    source_path: &str,
    mod_path: &str,
    category: &str,
    mod_name: &str,
) -> Result<InstallResult, String> {
    let src = Path::new(source_path);
    if !src.exists() {
        return Err(format!("Source folder does not exist: {}", source_path));
    }
    if !src.is_dir() {
        return Err(format!("Source path is not a folder: {}", source_path));
    }

    let name = derive_name(src, mod_name);
    let dest = compute_dest(mod_path, category, &name)?;

    // Copy source → destination (never move — user keeps their original).
    copy_dir_recursive(src, &dest)?;

    // For the symlink layout: also create the in-place symlink.
    if mods::is_symlink_layout(mod_path) {
        create_inplace_symlink(mod_path, category, &name, &dest)?;
    }

    let effective_category = if category.is_empty() { "Uncategorized" } else { category };
    let info = build_mod_info_at(&dest, effective_category)
        .ok_or_else(|| "Failed to read new mod info".to_string())?;

    Ok(InstallResult {
        installed_path: dest.to_string_lossy().to_string(),
        mod_info: info,
    })
}

/// Install a mod by extracting an archive (.zip / .7z / .rar) into the game's
/// mod directory. Uses the same single-wrapper-folder descent as the download
/// pipeline so archives like `Furina_v2.zip → Furina_v2/ → mod.ini` work
/// without an extra nesting level.
///
/// # Arguments
/// * `archive_path` — absolute path to the archive file.
/// * `mod_path`     — the game's configured mod directory.
/// * `category`     — category subfolder name. Pass `""` for Uncategorized.
/// * `mod_name`     — display/folder name to use. Pass `""` to derive from archive stem.
pub fn install_mod_from_archive(
    archive_path: &str,
    mod_path: &str,
    category: &str,
    mod_name: &str,
) -> Result<InstallResult, String> {
    let archive = Path::new(archive_path);
    if !archive.exists() {
        return Err(format!("Archive does not exist: {}", archive_path));
    }

    // Validate extension up front so we get a clear error before doing I/O.
    let ext = archive
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !["zip", "7z", "rar"].contains(&ext.as_str()) {
        return Err(format!(
            "Unsupported archive format \".{}\" — only .zip, .7z, and .rar are supported.",
            ext
        ));
    }

    // Extract into a temp staging directory.
    let staging = staging_dir(archive_path)?;
    let extracted = staging.join("extracted");
    if let Err(e) = extract_archive(archive, &extracted) {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }

    // Descend past single-folder wrappers.
    let content_root = find_content_root(&extracted);

    // Derive the mod name from the archive stem if not supplied.
    let name = if mod_name.trim().is_empty() {
        sanitize_component(
            archive
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("mod"),
        )
    } else {
        sanitize_component(mod_name)
    };

    let dest = compute_dest(mod_path, category, &name);
    let dest = match dest {
        Ok(d) => d,
        Err(e) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(e);
        }
    };

    // Move extracted content to destination.
    if let Err(e) = move_dir(&content_root, &dest) {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }

    // Clean up staging (the extracted dir is gone after move_dir, but the
    // staging root itself may still exist).
    let _ = fs::remove_dir_all(&staging);

    // For the symlink layout: create the in-place symlink.
    if mods::is_symlink_layout(mod_path) {
        create_inplace_symlink(mod_path, category, &name, &dest)?;
    }

    let effective_category = if category.is_empty() { "Uncategorized" } else { category };
    let info = build_mod_info_at(&dest, effective_category)
        .ok_or_else(|| "Failed to read new mod info".to_string())?;

    Ok(InstallResult {
        installed_path: dest.to_string_lossy().to_string(),
        mod_info: info,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Derive the on-disk folder name to use, sanitizing it.
fn derive_name(source: &Path, override_name: &str) -> String {
    if override_name.trim().is_empty() {
        sanitize_component(
            source
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("mod"),
        )
    } else {
        sanitize_component(override_name)
    }
}

/// Compute the destination path and confirm it doesn't already exist.
///
/// For the **legacy** layout: `mod_path/[category/]name`
/// For the **symlink** layout: `mod_path/DISABLED_managed_src/[category/]name`
/// (the in-place symlink at `mod_path/[category/]name` is created separately).
fn compute_dest(mod_path: &str, category: &str, name: &str) -> Result<PathBuf, String> {
    let base = if mods::is_symlink_layout(mod_path) {
        mods::src_root(mod_path)
    } else {
        PathBuf::from(mod_path)
    };

    let parent = if category.is_empty() || category == "Uncategorized" {
        base
    } else {
        base.join(sanitize_component(category))
    };

    let dest = parent.join(name);
    if dest.exists() {
        return Err(format!(
            "A mod named '{}' already exists{}. Choose a different name.",
            name,
            if category.is_empty() || category == "Uncategorized" {
                String::new()
            } else {
                format!(" in '{}'", category)
            }
        ));
    }

    fs::create_dir_all(&parent)
        .map_err(|e| format!("Failed to create category folder: {}", e))?;
    Ok(dest)
}

/// For the symlink layout, create the in-place symlink at
/// `mod_path/[category/]name` → `DISABLED_managed_src/[category/]name`.
///
/// The symlink makes the mod immediately visible to 3DMigoto/XXMI at its
/// expected path (same as the legacy layout would use), so ini keys remain
/// stable and the mod is enabled by default after install.
fn create_inplace_symlink(
    mod_path: &str,
    category: &str,
    name: &str,
    src_dest: &Path,
) -> Result<(), String> {
    let link_parent = if category.is_empty() || category == "Uncategorized" {
        PathBuf::from(mod_path)
    } else {
        PathBuf::from(mod_path).join(sanitize_component(category))
    };
    fs::create_dir_all(&link_parent)
        .map_err(|e| format!("Failed to create category folder for symlink: {}", e))?;

    let link = link_parent.join(name);

    // Remove any stale/dangling link at this location before creating a new one.
    if crate::symlink::is_symlink(&link) {
        crate::symlink::remove_symlink(&link)?;
    }

    crate::symlink::create_dir_symlink(src_dest, &link)?;
    Ok(())
}

/// Unique staging directory for a single install operation, under the app's
/// XDG data dir. Cleaned up by the caller on success or failure.
fn staging_dir(source_hint: &str) -> Result<PathBuf, String> {
    // Use the source path's hash to produce a short unique suffix.
    let hash = source_hint.len().wrapping_mul(31).wrapping_add(
        source_hint.chars().fold(0usize, |acc, c| acc.wrapping_add(c as usize)),
    );
    let dir = AppState::data_dir()
        .join("install_staging")
        .join(format!("{:x}", hash));
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| format!("Failed to clear old staging dir: {}", e))?;
    }
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create staging dir: {}", e))?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "mod-manager-install-test-{}-{}",
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

    /// A source mod folder on disk that the user might pick.
    fn make_source(name: &str) -> PathBuf {
        let dir = temp_dir().join(name);
        write_file(&dir.join("mod.ini"), "source-mod-data");
        write_file(&dir.join("preview.png"), "img");
        dir
    }

    #[test]
    fn install_from_folder_into_legacy_layout_with_category() {
        let mod_root = temp_dir();
        let source = make_source("Furina");

        let result = install_mod_from_folder(
            source.to_str().unwrap(),
            mod_root.to_str().unwrap(),
            "Characters",
            "", // derive name from source
        )
        .expect("install should succeed");

        // Files land at mod_root/Characters/Furina
        let dest = mod_root.join("Characters").join("Furina");
        assert!(dest.join("mod.ini").exists());
        assert_eq!(fs::read_to_string(dest.join("mod.ini")).unwrap(), "source-mod-data");
        // Source is left intact (copy, not move).
        assert!(source.join("mod.ini").exists(), "user's original must be untouched");

        assert_eq!(result.mod_info.category, "Characters");
        assert_eq!(result.mod_info.name, "Furina");
        assert!(result.mod_info.enabled);
        assert!(!result.mod_info.using_symlink_layout);

        fs::remove_dir_all(&mod_root).ok();
        fs::remove_dir_all(source.parent().unwrap()).ok();
    }

    #[test]
    fn install_from_folder_uncategorized_when_category_blank() {
        let mod_root = temp_dir();
        let source = make_source("SomeReshade");

        let result = install_mod_from_folder(
            source.to_str().unwrap(),
            mod_root.to_str().unwrap(),
            "",
            "",
        )
        .expect("install should succeed");

        // Uncategorized mods sit directly under mod_root.
        let dest = mod_root.join("SomeReshade");
        assert!(dest.join("mod.ini").exists());
        assert_eq!(result.mod_info.category, "Uncategorized");

        fs::remove_dir_all(&mod_root).ok();
        fs::remove_dir_all(source.parent().unwrap()).ok();
    }

    #[test]
    fn install_from_folder_uses_name_override() {
        let mod_root = temp_dir();
        let source = make_source("raw_folder_name");

        let result = install_mod_from_folder(
            source.to_str().unwrap(),
            mod_root.to_str().unwrap(),
            "Characters",
            "Nicer Display Name",
        )
        .expect("install should succeed");

        assert!(mod_root.join("Characters").join("Nicer Display Name").exists());
        assert_eq!(result.mod_info.name, "Nicer Display Name");

        fs::remove_dir_all(&mod_root).ok();
        fs::remove_dir_all(source.parent().unwrap()).ok();
    }

    #[test]
    fn install_from_folder_rejects_duplicate_name() {
        let mod_root = temp_dir();
        let source = make_source("Furina");

        install_mod_from_folder(source.to_str().unwrap(), mod_root.to_str().unwrap(), "Characters", "")
            .expect("first install should succeed");

        // Second install of the same name+category must be rejected.
        let err = install_mod_from_folder(
            source.to_str().unwrap(),
            mod_root.to_str().unwrap(),
            "Characters",
            "",
        );
        assert!(err.is_err(), "duplicate install should be rejected");

        fs::remove_dir_all(&mod_root).ok();
        fs::remove_dir_all(source.parent().unwrap()).ok();
    }

    #[cfg(unix)]
    #[test]
    fn install_from_folder_into_symlink_layout_creates_inplace_symlink() {
        let mod_root = temp_dir();
        // Activate symlink layout by creating the managed_src dir.
        fs::create_dir_all(mod_root.join(mods::MANAGED_SRC)).unwrap();
        let source = make_source("Furina");

        let result = install_mod_from_folder(
            source.to_str().unwrap(),
            mod_root.to_str().unwrap(),
            "Characters",
            "",
        )
        .expect("install should succeed");

        // Real files land inside DISABLED_managed_src.
        let src_dest = mod_root
            .join(mods::MANAGED_SRC)
            .join("Characters")
            .join("Furina");
        assert!(src_dest.join("mod.ini").exists(), "source files should be in managed_src");

        // In-place symlink is created at mod_root/Characters/Furina so 3DMigoto sees it.
        let link = mod_root.join("Characters").join("Furina");
        assert!(crate::symlink::is_symlink(&link), "in-place symlink should exist");
        assert!(link.join("mod.ini").exists(), "files reachable through the symlink");

        assert!(result.mod_info.using_symlink_layout);
        assert!(result.mod_info.enabled);
        assert!(result.installed_path.contains(mods::MANAGED_SRC));

        fs::remove_dir_all(&mod_root).ok();
        fs::remove_dir_all(source.parent().unwrap()).ok();
    }

    #[test]
    fn install_from_archive_rejects_unsupported_extension() {
        let mod_root = temp_dir();
        let bogus = temp_dir().join("mod.tar.gz");
        write_file(&bogus, "not a supported archive");

        let err = install_mod_from_archive(
            bogus.to_str().unwrap(),
            mod_root.to_str().unwrap(),
            "Characters",
            "",
        );
        assert!(err.is_err(), "unsupported extension should be rejected");
        assert!(err.unwrap_err().contains("Unsupported archive format"));

        fs::remove_dir_all(&mod_root).ok();
        fs::remove_dir_all(bogus.parent().unwrap()).ok();
    }

    #[test]
    fn install_from_archive_extracts_zip_and_descends_wrapper_folder() {
        let mod_root = temp_dir();
        let staging = temp_dir();
        let archive_path = staging.join("Furina_v2.zip");

        // Build a zip where everything is wrapped in one extra top-level folder,
        // the common packaging pattern find_content_root descends through.
        {
            let file = fs::File::create(&archive_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let opts: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
            zip.add_directory("WrapperFolder/", opts).unwrap();
            zip.start_file("WrapperFolder/mod.ini", opts).unwrap();
            std::io::Write::write_all(&mut zip, b"archive-mod-data").unwrap();
            zip.finish().unwrap();
        }

        let result = install_mod_from_archive(
            archive_path.to_str().unwrap(),
            mod_root.to_str().unwrap(),
            "Characters",
            "", // derive name from archive stem → "Furina_v2"
        )
        .expect("archive install should succeed");

        // Content root descended past WrapperFolder, so mod.ini is at the top.
        let dest = mod_root.join("Characters").join("Furina_v2");
        assert!(dest.join("mod.ini").exists(), "mod.ini should be directly in the mod folder");
        assert_eq!(fs::read_to_string(dest.join("mod.ini")).unwrap(), "archive-mod-data");
        assert_eq!(result.mod_info.name, "Furina v2");

        fs::remove_dir_all(&mod_root).ok();
        fs::remove_dir_all(&staging).ok();
    }
}
