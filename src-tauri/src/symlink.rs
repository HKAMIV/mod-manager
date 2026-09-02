//! Cross-platform symlink primitives for Phase 12.
//!
//! On Unix (Linux, macOS) `std::os::unix::fs::symlink` is available directly.
//! On Windows, directory symlinks require either Developer Mode or admin rights —
//! we detect the failure and return a clear error rather than a cryptic OS code.
//!
//! All operations work with absolute paths. The caller is responsible for
//! ensuring parents exist before calling `create_dir_symlink`.

use std::path::Path;

/// Create a directory symlink: `link` → `target`.
/// Both paths should be absolute. `link` must NOT already exist.
pub fn create_dir_symlink(target: &Path, link: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
            .map_err(|e| format!("Failed to create symlink '{}' → '{}': {}", link.display(), target.display(), e))
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(target, link).map_err(|e| {
            let hint = if e.raw_os_error() == Some(1314) {
                // ERROR_PRIVILEGE_NOT_HELD
                " — on Windows, creating symlinks requires Developer Mode or elevated privileges. \
                 Enable Developer Mode in Settings → Privacy & Security → For Developers."
                    .to_string()
            } else {
                String::new()
            };
            format!(
                "Failed to create symlink '{}' → '{}': {}{}",
                link.display(),
                target.display(),
                e,
                hint
            )
        })
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(format!(
            "Symlink creation not supported on this platform (tried '{}' → '{}')",
            link.display(),
            target.display()
        ))
    }
}

/// Remove a symlink. Only removes the link itself, not the target it points to.
/// Returns `Ok(())` if the link didn't exist (idempotent).
pub fn remove_symlink(link: &Path) -> Result<(), String> {
    if !link.exists() && !is_symlink(link) {
        return Ok(()); // already gone — idempotent
    }

    // On all platforms a symlink to a directory can be removed with
    // `fs::remove_file` on Unix, but requires `fs::remove_dir` on Windows.
    #[cfg(unix)]
    {
        std::fs::remove_file(link)
            .map_err(|e| format!("Failed to remove symlink '{}': {}", link.display(), e))
    }
    #[cfg(windows)]
    {
        // On Windows, symlinks to directories must be removed as directories.
        std::fs::remove_dir(link)
            .map_err(|e| format!("Failed to remove symlink '{}': {}", link.display(), e))
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(format!("Symlink removal not supported on this platform (tried '{}')", link.display()))
    }
}

/// Returns `true` if `path` is a symlink (regardless of whether the target exists).
/// Uses `symlink_metadata` so it doesn't follow the link — `path.exists()` would
/// return `false` for a dangling symlink, masking the fact that the link itself exists.
pub fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir() -> std::path::PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "mod-manager-symlink-test-{}-{}",
            std::process::id(),
            n
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Symlink tests are Unix-only because creating directory symlinks on
    /// Windows requires Developer Mode, which CI machines may not have.
    #[cfg(unix)]
    #[test]
    fn create_and_detect_symlink() {
        let root = temp_dir();
        let target = root.join("target");
        let link = root.join("link");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("mod.ini"), "test").unwrap();

        create_dir_symlink(&target, &link).expect("create should succeed");
        assert!(is_symlink(&link), "link path should be a symlink");
        assert!(link.join("mod.ini").exists(), "files should be accessible through the link");

        fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn remove_symlink_removes_only_the_link() {
        let root = temp_dir();
        let target = root.join("target");
        let link = root.join("link");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("mod.ini"), "data").unwrap();

        create_dir_symlink(&target, &link).unwrap();
        assert!(is_symlink(&link));

        remove_symlink(&link).expect("remove should succeed");
        assert!(!link.exists(), "link should be gone");
        assert!(target.join("mod.ini").exists(), "target files must be untouched");

        fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn remove_symlink_is_idempotent() {
        let root = temp_dir();
        let link = root.join("nonexistent");
        // Removing a path that doesn't exist should not error.
        remove_symlink(&link).expect("idempotent remove should succeed");
        fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn is_symlink_returns_false_for_real_dirs() {
        let root = temp_dir();
        let real_dir = root.join("real");
        fs::create_dir_all(&real_dir).unwrap();
        assert!(!is_symlink(&real_dir));
        fs::remove_dir_all(&root).ok();
    }
}
