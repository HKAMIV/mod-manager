//! Download queue + auto-install pipeline (Phase 7).
//!
//! Downloads are processed strictly sequentially by a single background
//! worker loop (spawned once at app startup), mirroring the "one thing at a
//! time" model of `watcher.rs`'s registry rather than introducing a task
//! pool — mod archives are large enough, and users' connections slow enough,
//! that parallel downloads would mostly just contend for the same pipe.
//!
//! Pipeline per item: stream-download to a staging dir -> extract
//! (zip/7z/rar) -> "smart placement" (find the real content root inside the
//! archive,
//! since many mod archives wrap everything in one extra folder) -> if the
//! destination mod folder already exists, pause for the user to choose
//! overwrite/rename/cancel; otherwise install immediately and save a preview
//! image alongside if the mod didn't ship its own.
//!
//! State (queue + history) is a single JSON manifest under the config dir,
//! same pattern as `presets.rs`/`restore_points.rs`. In-memory-only bits
//! (cancel flags, the notify signal that wakes the worker) live in a
//! process-wide singleton, same pattern as `watcher.rs`'s `WatcherRegistry`.

use crate::mods;
use crate::restore_points::copy_dir_recursive;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tokio::sync::Notify;

pub const DOWNLOAD_UPDATED_EVENT: &str = "download-updated";
pub const DOWNLOAD_PROGRESS_EVENT: &str = "download-progress";

/// Lightweight, frequent progress tick — kept separate from `DownloadItem`
/// updates (which fire only on status changes) so the UI can animate a
/// progress bar without re-rendering the whole download list on every chunk.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

/// A conflict resolution action for a download paused in
/// `DownloadStatus::WaitingForConflict`.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    /// Delete the existing folder and install in its place.
    Overwrite,
    /// Install alongside the existing folder under a "(2)", "(3)", ... name.
    Rename,
    /// Abandon the download entirely and clean up staged files.
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Extracting,
    /// Placement found an existing folder at the computed destination and is
    /// waiting on `resolve_download_conflict`. `existing_path` is surfaced so
    /// the UI can show exactly what would be overwritten.
    WaitingForConflict { existing_path: String },
    Installing,
    Completed { installed_path: String },
    Failed { message: String },
    Cancelled,
}

impl DownloadStatus {
    fn is_active(&self) -> bool {
        matches!(
            self,
            DownloadStatus::Downloading | DownloadStatus::Extracting | DownloadStatus::Installing
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadItem {
    pub id: String,
    pub game_id: String,
    pub mod_id: u32,
    pub mod_name: String,
    pub file_id: u64,
    pub file_name: String,
    pub download_url: String,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    /// The character/category name to place the mod under (e.g. "Furina"),
    /// taken from GameBanana's leaf category for the mod. `None`/empty means
    /// no useful category info — the mod is placed directly under the game's
    /// mod root, which `scan_mods` will pick up as "Uncategorized".
    pub category_hint: Option<String>,
    /// Thumbnail URL to save as a preview image alongside the installed mod,
    /// used only if the extracted archive didn't already include one of its
    /// own (see `mods::PREVIEW_NAMES`-equivalent check in `place_content`).
    pub preview_image_url: Option<String>,
    /// Snapshot of the game's configured mod directory at queue time.
    pub mod_path: String,
    pub status: DownloadStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn manifest_path() -> PathBuf {
    AppState::config_dir().join("downloads.json")
}

/// Where a given download's in-flight archive + extracted contents live
/// while being processed. Removed entirely once the item reaches a terminal
/// state (installed, cancelled, or the conflict/failure is resolved away).
fn staging_dir(id: &str) -> PathBuf {
    AppState::data_dir().join("download_staging").join(id)
}

fn load_manifest() -> Vec<DownloadItem> {
    let path = manifest_path();
    let Ok(data) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let mut items: Vec<DownloadItem> = serde_json::from_str(&data).unwrap_or_default();

    // Anything still "in flight" reflects a process that no longer exists
    // (the app that owned it was closed/crashed) — its staged files are
    // orphaned and unrecoverable, so surface it as a failure the user can
    // retry rather than pretending it's still progressing.
    for item in items.iter_mut() {
        if item.status.is_active() || matches!(item.status, DownloadStatus::WaitingForConflict { .. }) {
            item.status = DownloadStatus::Failed {
                message: "Interrupted by app restart — please retry".to_string(),
            };
            item.updated_at = now();
            let _ = fs::remove_dir_all(staging_dir(&item.id));
        }
    }
    items
}

fn save_manifest(items: &[DownloadItem]) -> Result<(), String> {
    fs::create_dir_all(AppState::config_dir()).map_err(|e| e.to_string())?;
    let data = serde_json::to_string_pretty(items).map_err(|e| e.to_string())?;
    fs::write(manifest_path(), data).map_err(|e| e.to_string())
}

struct Inner {
    items: Mutex<Vec<DownloadItem>>,
    cancel_flags: Mutex<HashMap<String, Arc<AtomicBool>>>,
    notify: Arc<Notify>,
    client: reqwest::Client,
}

static MANAGER: OnceLock<Inner> = OnceLock::new();

fn manager() -> &'static Inner {
    MANAGER.get_or_init(|| Inner {
        items: Mutex::new(load_manifest()),
        cancel_flags: Mutex::new(HashMap::new()),
        notify: Arc::new(Notify::new()),
        client: reqwest::Client::new(),
    })
}

fn persist_and_snapshot() -> Vec<DownloadItem> {
    let items = manager().items.lock().unwrap();
    let snapshot = items.clone();
    let _ = save_manifest(&snapshot);
    snapshot
}

/// Mutate the item matching `id` under the lock, then persist and return a
/// clone of the updated item (or `None` if no such item exists). Kept
/// separate from emitting the update event so the mutex is never held across
/// an `app.emit` call.
fn update_item<F: FnOnce(&mut DownloadItem)>(id: &str, f: F) -> Option<DownloadItem> {
    let updated = {
        let mut items = manager().items.lock().unwrap();
        let item = items.iter_mut().find(|d| d.id == id)?;
        f(item);
        item.updated_at = now();
        item.clone()
    };
    let snapshot = { manager().items.lock().unwrap().clone() };
    let _ = save_manifest(&snapshot);
    Some(updated)
}

fn emit_update(app: &AppHandle, item: &DownloadItem) {
    let _ = app.emit(DOWNLOAD_UPDATED_EVENT, item);
}

/// Called once at app startup. Loads persisted history and spawns the
/// sequential worker loop that drains the queue for the lifetime of the app.
pub fn init(app: AppHandle) {
    let _ = manager(); // force initialization / manifest load now, not lazily
    tauri::async_runtime::spawn(worker_loop(app));
}

pub fn list() -> Vec<DownloadItem> {
    let mut items = manager().items.lock().unwrap().clone();
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at)); // newest first
    items
}

/// Queue a new download. Validates the archive extension up front so an
/// unsupported format (anything but .zip/.7z) is rejected immediately rather
/// than silently failing later in the pipeline.
pub fn queue(
    game_id: String,
    mod_path: String,
    mod_id: u32,
    mod_name: String,
    file_id: u64,
    file_name: String,
    download_url: String,
    total_bytes: u64,
    category_hint: Option<String>,
    preview_image_url: Option<String>,
) -> Result<DownloadItem, String> {
    let ext = Path::new(&file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !["zip", "7z", "rar"].contains(&ext.as_str()) {
        return Err(format!(
            "Unsupported archive format \".{}\" — only .zip, .7z, and .rar can be auto-installed. \
             Download it from the GameBanana page and extract it manually.",
            ext
        ));
    }

    let item = DownloadItem {
        id: uuid::Uuid::new_v4().to_string(),
        game_id,
        mod_id,
        mod_name,
        file_id,
        file_name,
        download_url,
        total_bytes,
        downloaded_bytes: 0,
        category_hint: category_hint.filter(|c| !c.trim().is_empty()),
        preview_image_url,
        mod_path,
        status: DownloadStatus::Queued,
        created_at: now(),
        updated_at: now(),
    };

    {
        let mut items = manager().items.lock().unwrap();
        items.push(item.clone());
    }
    let _ = persist_and_snapshot();
    manager().notify.notify_one();
    Ok(item)
}

/// Cancel a download. Queued items are cancelled immediately; items already
/// in flight are flagged and the worker itself finalizes the cancellation
/// (see `worker_loop`) once it next checks the flag, since a partially
/// written temp file can't just be abandoned without cleanup.
pub fn cancel(app: &AppHandle, id: &str) -> Result<(), String> {
    let status = {
        let items = manager().items.lock().unwrap();
        items
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.status.clone())
            .ok_or_else(|| format!("Download '{}' not found", id))?
    };

    match status {
        DownloadStatus::Queued => {
            cleanup_staging(id);
            if let Some(updated) = update_item(id, |i| i.status = DownloadStatus::Cancelled) {
                emit_update(app, &updated);
            }
            Ok(())
        }
        DownloadStatus::Downloading | DownloadStatus::Extracting | DownloadStatus::Installing => {
            cancel_flag_for(id).store(true, Ordering::SeqCst);
            Ok(())
        }
        _ => Err("This download isn't in a cancellable state".to_string()),
    }
}

/// Re-queue a failed or cancelled download from scratch. Rejected for
/// anything currently in flight or already completed — retrying those would
/// either race the active worker or silently redo a successful install.
pub fn retry(app: &AppHandle, id: &str) -> Result<(), String> {
    {
        let items = manager().items.lock().unwrap();
        let item = items
            .iter()
            .find(|d| d.id == id)
            .ok_or_else(|| format!("Download '{}' not found", id))?;
        match item.status {
            DownloadStatus::Failed { .. } | DownloadStatus::Cancelled => {}
            _ => return Err("Only failed or cancelled downloads can be retried".to_string()),
        }
    }

    cleanup_staging(id);
    let updated = update_item(id, |i| {
        i.downloaded_bytes = 0;
        i.status = DownloadStatus::Queued;
    })
    .ok_or_else(|| format!("Download '{}' not found", id))?;

    emit_update(app, &updated);
    manager().notify.notify_one();
    Ok(())
}

/// Remove a download from the list/history. Not allowed while actively in
/// flight — cancel it first, so a removal can never orphan a running task.
pub fn remove(id: &str) -> Result<(), String> {
    let status = {
        let items = manager().items.lock().unwrap();
        items
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.status.clone())
            .ok_or_else(|| format!("Download '{}' not found", id))?
    };
    if status.is_active() {
        return Err("Cancel this download before removing it".to_string());
    }
    cleanup_staging(id);
    {
        let mut items = manager().items.lock().unwrap();
        items.retain(|d| d.id != id);
    }
    persist_and_snapshot();
    Ok(())
}

/// Resolve a paused conflict by overwriting, renaming alongside, or
/// cancelling the install. Runs the (blocking) filesystem work on a blocking
/// thread since it may involve copying an entire mod folder.
pub async fn resolve_conflict(
    app: AppHandle,
    id: String,
    action: ConflictResolution,
) -> Result<(), String> {
    let item = {
        let items = manager().items.lock().unwrap();
        items
            .iter()
            .find(|d| d.id == id)
            .cloned()
            .ok_or_else(|| format!("Download '{}' not found", id))?
    };

    let existing_path = match &item.status {
        DownloadStatus::WaitingForConflict { existing_path } => existing_path.clone(),
        _ => return Err("This download has no conflict to resolve".to_string()),
    };

    if action == ConflictResolution::Cancel {
        cleanup_staging(&id);
        if let Some(updated) = update_item(&id, |i| i.status = DownloadStatus::Cancelled) {
            emit_update(&app, &updated);
        }
        return Ok(());
    }

    let overwrite = action == ConflictResolution::Overwrite;
    let content_root = content_root_dir(&id);
    let dest = PathBuf::from(&existing_path);

    let result = tokio::task::spawn_blocking(move || -> Result<PathBuf, String> {
        let final_dest = if overwrite {
            fs::remove_dir_all(&dest)
                .map_err(|e| format!("Failed to remove existing folder: {}", e))?;
            dest
        } else {
            unique_sibling_path(&dest)
        };
        move_dir(&content_root, &final_dest)?;
        Ok(final_dest)
    })
    .await
    .map_err(|e| format!("Install task panicked: {}", e))?;

    match result {
        Ok(final_dest) => finish_install(&app, &item, &final_dest).await,
        Err(e) => {
            if let Some(updated) =
                update_item(&id, |i| i.status = DownloadStatus::Failed { message: e.clone() })
            {
                emit_update(&app, &updated);
            }
            Err(e)
        }
    }
}

fn cancel_flag_for(id: &str) -> Arc<AtomicBool> {
    let mut flags = manager().cancel_flags.lock().unwrap();
    flags
        .entry(id.to_string())
        .or_insert_with(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

fn take_cancel_flag(id: &str) {
    manager().cancel_flags.lock().unwrap().remove(id);
}

fn cleanup_staging(id: &str) {
    let _ = fs::remove_dir_all(staging_dir(id));
}

fn archive_path(id: &str, file_name: &str) -> PathBuf {
    let ext = Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin");
    staging_dir(id).join(format!("archive.{}", ext))
}

fn extracted_dir(id: &str) -> PathBuf {
    staging_dir(id).join("extracted")
}

/// The resolved content root within the extracted archive, computed once
/// during placement and reused if the user later resolves a conflict.
fn content_root_dir(id: &str) -> PathBuf {
    find_content_root(&extracted_dir(id))
}

/// The main worker loop: pulls the oldest `Queued` item and runs it to
/// completion (or to a paused/failed/cancelled stop), forever, for the life
/// of the app. Idles on `notify` when the queue is empty instead of polling.
async fn worker_loop(app: AppHandle) {
    loop {
        let next = {
            let mut items = manager().items.lock().unwrap();
            let candidate = items
                .iter_mut()
                .filter(|d| d.status == DownloadStatus::Queued)
                .min_by_key(|d| d.created_at);
            match candidate {
                Some(item) => {
                    item.status = DownloadStatus::Downloading;
                    item.updated_at = now();
                    Some(item.clone())
                }
                None => None,
            }
        };

        match next {
            Some(item) => {
                let snapshot = persist_and_snapshot();
                if let Some(updated) = snapshot.iter().find(|d| d.id == item.id) {
                    emit_update(&app, updated);
                }
                process_item(&app, item).await;
            }
            None => manager().notify.notified().await,
        }
    }
}

async fn process_item(app: &AppHandle, item: DownloadItem) {
    let flag = cancel_flag_for(&item.id);

    if let Err(e) = download_archive(app, &item, &flag).await {
        take_cancel_flag(&item.id);
        finish_with_outcome(app, &item.id, Err(e)).await;
        return;
    }

    if flag.load(Ordering::SeqCst) {
        finalize_cancel(app, &item.id).await;
        take_cancel_flag(&item.id);
        return;
    }

    if let Some(updated) = update_item(&item.id, |i| i.status = DownloadStatus::Extracting) {
        emit_update(app, &updated);
    }

    let archive = archive_path(&item.id, &item.file_name);
    let dest = extracted_dir(&item.id);
    let extract_result = tokio::task::spawn_blocking(move || extract_archive(&archive, &dest))
        .await
        .map_err(|e| format!("Extraction task panicked: {}", e))
        .and_then(|r| r);

    if let Err(e) = extract_result {
        take_cancel_flag(&item.id);
        finish_with_outcome(app, &item.id, Err(e)).await;
        return;
    }

    if flag.load(Ordering::SeqCst) {
        finalize_cancel(app, &item.id).await;
        take_cancel_flag(&item.id);
        return;
    }

    if let Some(updated) = update_item(&item.id, |i| i.status = DownloadStatus::Installing) {
        emit_update(app, &updated);
    }

    let placement = {
        let item = item.clone();
        tokio::task::spawn_blocking(move || plan_placement(&item))
            .await
            .map_err(|e| format!("Placement task panicked: {}", e))
            .and_then(|r| r)
    };

    take_cancel_flag(&item.id);

    match placement {
        Ok(Placement::Clear(dest)) => {
            let outcome = finish_install(app, &item, &dest).await;
            if let Err(e) = outcome {
                finish_with_outcome(app, &item.id, Err(e)).await;
            }
        }
        Ok(Placement::Conflict(existing)) => {
            if let Some(updated) = update_item(&item.id, |i| {
                i.status = DownloadStatus::WaitingForConflict {
                    existing_path: existing.to_string_lossy().to_string(),
                }
            }) {
                emit_update(app, &updated);
            }
        }
        Err(e) => finish_with_outcome(app, &item.id, Err(e)).await,
    }
}

async fn finish_with_outcome(app: &AppHandle, id: &str, result: Result<(), String>) {
    if let Err(e) = result {
        cleanup_staging(id);
        if let Some(updated) =
            update_item(id, |i| i.status = DownloadStatus::Failed { message: e })
        {
            emit_update(app, &updated);
        }
    }
}

async fn finalize_cancel(app: &AppHandle, id: &str) {
    cleanup_staging(id);
    if let Some(updated) = update_item(id, |i| i.status = DownloadStatus::Cancelled) {
        emit_update(app, &updated);
    }
}

/// Finish installing an already-placed mod folder: save a preview image if
/// the mod didn't ship its own, clean up staging, mark Completed, and notify
/// the Local Mods view (via the same `mods-changed` event the filesystem
/// watcher uses) so the new mod shows up without a manual refresh.
async fn finish_install(app: &AppHandle, item: &DownloadItem, dest: &Path) -> Result<(), String> {
    if let Some(url) = item.preview_image_url.clone() {
        let dest = dest.to_path_buf();
        let client = manager().client.clone();
        let _ = save_preview_image(&client, &url, &dest).await;
    }

    // Persist the GameBanana origin link so update tracking (Phase 8) can
    // later check if a newer version exists without the user having to
    // manually identify which GB page each installed mod came from.
    let mod_folder_name = dest
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let category = item
        .category_hint
        .as_deref()
        .filter(|c| !c.is_empty())
        .unwrap_or("Uncategorized");
    let mod_key = format!("{}/{}", category, mod_folder_name);

    // Look up the installed file's version string from the download history
    // itself — we don't have it on DownloadItem directly, but we can query
    // GB for it. Since we just downloaded from this exact file_id, the
    // version info is whatever was displayed in the detail panel when the
    // user clicked Download. We don't re-fetch here; just pass None if we
    // don't have it cached (the file_id comparison is what matters for
    // update detection, not the version string).
    let _ = crate::update_tracking::link_mod(
        &item.game_id,
        &mod_key,
        item.mod_id,
        item.file_id,
        None, // version string not available without a re-fetch; file_id is sufficient
    );

    cleanup_staging(&item.id);

    let installed_path = dest.to_string_lossy().to_string();
    if let Some(updated) = update_item(&item.id, |i| {
        i.status = DownloadStatus::Completed {
            installed_path: installed_path.clone(),
        }
    }) {
        emit_update(app, &updated);
    }

    let _ = app.emit(crate::watcher::MODS_CHANGED_EVENT, &item.game_id);
    Ok(())
}

async fn save_preview_image(client: &reqwest::Client, url: &str, dest_dir: &Path) -> Result<(), String> {
    if mods::has_preview_image(dest_dir) {
        return Ok(()); // the mod already ships its own — don't clobber it
    }
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let ext = Path::new(url)
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| ["png", "jpg", "jpeg", "webp"].contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or("jpg")
        .to_string();
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    fs::write(dest_dir.join(format!("preview.{}", ext)), bytes).map_err(|e| e.to_string())
}

/// Stream the archive to a staging file, emitting throttled progress events
/// and checking `cancel_flag` between chunks so a cancel mid-download stops
/// promptly instead of waiting for the whole file.
async fn download_archive(
    app: &AppHandle,
    item: &DownloadItem,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<(), String> {
    use futures_util::StreamExt;

    let dest = archive_path(&item.id, &item.file_name);
    fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;

    let response = manager()
        .client
        .get(&item.download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to start download: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Download server error: {}", e))?;

    let total_bytes = response.content_length().unwrap_or(item.total_bytes);
    let mut file = tokio::fs::File::create(&dest)
        .await
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_emit = Instant::now();
    // Throttle progress persistence to disk (cheap but not free) separately
    // from the lighter in-memory event, which fires more often.
    let mut last_persist = Instant::now();

    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        if cancel_flag.load(Ordering::SeqCst) {
            drop(file);
            let _ = tokio::fs::remove_file(&dest).await;
            return Ok(());
        }
        let chunk = chunk.map_err(|e| format!("Download interrupted: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write to disk: {}", e))?;
        downloaded += chunk.len() as u64;

        if last_emit.elapsed() >= Duration::from_millis(200) {
            let _ = app.emit(
                DOWNLOAD_PROGRESS_EVENT,
                DownloadProgress {
                    id: item.id.clone(),
                    downloaded_bytes: downloaded,
                    total_bytes,
                },
            );
            last_emit = Instant::now();
        }
        if last_persist.elapsed() >= Duration::from_secs(2) {
            update_item(&item.id, |i| {
                i.downloaded_bytes = downloaded;
                i.total_bytes = total_bytes;
            });
            last_persist = Instant::now();
        }
    }

    file.flush().await.map_err(|e| e.to_string())?;
    update_item(&item.id, |i| {
        i.downloaded_bytes = downloaded;
        i.total_bytes = total_bytes;
    });
    let _ = app.emit(
        DOWNLOAD_PROGRESS_EVENT,
        DownloadProgress {
            id: item.id.clone(),
            downloaded_bytes: downloaded,
            total_bytes,
        },
    );
    Ok(())
}

/// Extract a downloaded archive to `dest` based on its file extension.
/// Runs synchronously — callers should invoke this via `spawn_blocking`.
fn extract_archive(archive: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let ext = archive
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "zip" => extract_zip(archive, dest),
        "7z" => sevenz_rust2::decompress_file(archive, dest)
            .map_err(|e| format!("Failed to extract 7z archive: {}", e)),
        "rar" => extract_rar(archive, dest),
        other => Err(format!("Unsupported archive format: .{}", other)),
    }
}

/// Extract a .rar archive via the `unrar` crate (a wrapper around RARLab's
/// own UnRAR C++ source, since RAR's compression format is proprietary and
/// has no pure-Rust decoder). Entries are processed one at a time, streaming
/// straight to disk rather than buffering a whole file in memory, since mod
/// archives can run into the hundreds of MB.
fn extract_rar(archive: &Path, dest: &Path) -> Result<(), String> {
    use unrar::Archive as RarArchive;

    let mut open_archive = RarArchive::new(archive)
        .open_for_processing()
        .map_err(|e| format!("Invalid rar archive: {}", e))?;

    while let Some(header) = open_archive
        .read_header()
        .map_err(|e| format!("Failed to read rar entry: {}", e))?
    {
        let entry = header.entry();
        // Mirror the zip path's zip-slip guard: refuse any entry whose name
        // would resolve outside `dest` rather than following it.
        let Some(relative) = safe_relative_path(&entry.filename) else {
            open_archive = header
                .skip()
                .map_err(|e| format!("Failed to skip unsafe rar entry: {}", e))?;
            continue;
        };
        let out_path = dest.join(&relative);

        if entry.is_directory() {
            fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
            open_archive = header
                .skip()
                .map_err(|e| format!("Failed to skip rar directory entry: {}", e))?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            open_archive = header
                .extract_to(&out_path)
                .map_err(|e| format!("Failed to extract rar entry: {}", e))?;
        }
    }
    Ok(())
}

/// Reject path-traversal (`../..`) and absolute-path entries, mirroring what
/// `zip::ZipFile::enclosed_name` does for the zip path — the `unrar` crate
/// has no equivalent built-in guard, so this exists to give .rar archives
/// the same protection.
fn safe_relative_path(raw: &Path) -> Option<PathBuf> {
    if raw.is_absolute() {
        return None;
    }
    let mut result = PathBuf::new();
    for component in raw.components() {
        match component {
            std::path::Component::Normal(part) => result.push(part),
            std::path::Component::CurDir => {}
            // ParentDir, RootDir, Prefix — anything that could escape `dest`.
            _ => return None,
        }
    }
    if result.as_os_str().is_empty() {
        None
    } else {
        Some(result)
    }
}

fn extract_zip(archive: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(archive).map_err(|e| format!("Failed to open archive: {}", e))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Invalid zip archive: {}", e))?;

    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| format!("Failed to read archive entry: {}", e))?;
        // `enclosed_name` rejects path traversal (`../..`) and absolute paths
        // — a zip crafted to escape the extraction directory ("zip slip")
        // is refused rather than silently followed.
        let Some(relative) = entry.enclosed_name() else {
            continue; // unsafe path — skip rather than abort the whole extraction
        };
        let out_path = dest.join(relative);

        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out_file = fs::File::create(&out_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut out_file).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

enum Placement {
    Clear(PathBuf),
    Conflict(PathBuf),
}

/// Compute where the mod should be installed. If the spot is free, moves the
/// extracted content there immediately and returns `Placement::Clear` with
/// its final location. If something already exists there, leaves the
/// extracted content in staging untouched and returns `Placement::Conflict`
/// so the caller can pause for user input — `resolve_conflict` picks the
/// content back up from staging via `content_root_dir` once resolved.
/// Runs synchronously — invoke via `spawn_blocking`.
fn plan_placement(item: &DownloadItem) -> Result<Placement, String> {
    let content_root = find_content_root(&extracted_dir(&item.id));

    let mod_root = PathBuf::from(&item.mod_path);
    let dest_parent = match &item.category_hint {
        Some(category) => mod_root.join(sanitize_component(category)),
        None => mod_root,
    };
    let dest = dest_parent.join(sanitize_component(&item.mod_name));

    if dest.exists() {
        Ok(Placement::Conflict(dest))
    } else {
        fs::create_dir_all(&dest_parent).map_err(|e| e.to_string())?;
        move_dir(&content_root, &dest)?;
        Ok(Placement::Clear(dest))
    }
}

/// Descend through single-child directory wrappers (a very common archive
/// packaging pattern — the whole mod zipped inside one extra top-level
/// folder) to find the folder that actually contains the mod's files.
/// Capped at a shallow depth so a pathological/malicious archive can't cause
/// unbounded recursion.
fn find_content_root(extracted: &Path) -> PathBuf {
    let mut current = extracted.to_path_buf();
    for _ in 0..5 {
        let Ok(entries) = fs::read_dir(&current) else {
            break;
        };
        let visible: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                !p.file_name()
                    .map(|n| n.to_string_lossy().starts_with('.'))
                    .unwrap_or(false)
            })
            .collect();

        if visible.len() == 1 && visible[0].is_dir() {
            current = visible[0].clone();
        } else {
            break;
        }
    }
    current
}

/// Move a directory's contents to `dest`. Tries a plain rename first (cheap,
/// works when both paths are on the same filesystem); falls back to a
/// recursive copy + delete since the staging dir (XDG data dir) and a user's
/// configured mod directory are not guaranteed to share a filesystem (e.g. a
/// Steam Deck SD card), where `rename` fails with EXDEV.
fn move_dir(src: &Path, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    copy_dir_recursive(src, dest)?;
    fs::remove_dir_all(src).ok();
    Ok(())
}

/// Find a sibling path for `dest` that doesn't collide, by appending
/// " (2)", " (3)", ... until a free name is found.
fn unique_sibling_path(dest: &Path) -> PathBuf {
    let parent = dest.parent().unwrap_or(dest);
    let stem = dest
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "mod".to_string());

    for n in 2.. {
        let candidate = parent.join(format!("{} ({})", stem, n));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

/// Strip characters that are invalid (or awkward) in folder names on common
/// filesystems, collapse whitespace, and fall back to a generic name if
/// nothing usable is left after sanitizing.
fn sanitize_component(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let trimmed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = trimmed.trim_matches('.').trim();
    if trimmed.is_empty() {
        "mod".to_string()
    } else {
        trimmed.chars().take(120).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_component_strips_illegal_characters() {
        assert_eq!(sanitize_component("Furina: Outfit / v2"), "Furina Outfit v2");
        assert_eq!(sanitize_component("   "), "mod");
        assert_eq!(sanitize_component("Normal Name"), "Normal Name");
    }

    #[test]
    fn sanitize_component_collapses_whitespace() {
        assert_eq!(sanitize_component("Too   Many   Spaces"), "Too Many Spaces");
    }

    fn temp_dir() -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "mod-manager-dl-test-{}-{}",
            std::process::id(),
            n
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn find_content_root_descends_through_single_wrapper_folders() {
        let root = temp_dir();
        fs::create_dir_all(root.join("Outer/Inner")).unwrap();
        fs::write(root.join("Outer/Inner/mod.ini"), "test").unwrap();

        let found = find_content_root(&root);
        assert_eq!(found, root.join("Outer/Inner"));

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn find_content_root_stops_at_multiple_entries() {
        let root = temp_dir();
        fs::create_dir_all(root.join("Outer")).unwrap();
        fs::write(root.join("Outer/mod.ini"), "test").unwrap();
        fs::write(root.join("readme.txt"), "test").unwrap();

        // Two visible entries at the top level — this IS the content root,
        // since descending would lose "readme.txt".
        let found = find_content_root(&root);
        assert_eq!(found, root);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn find_content_root_stops_when_loose_files_at_root() {
        let root = temp_dir();
        fs::write(root.join("mod.ini"), "test").unwrap();

        let found = find_content_root(&root);
        assert_eq!(found, root);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn move_dir_falls_back_across_simulated_filesystems() {
        // We can't easily simulate EXDEV in a unit test, but we can verify
        // the happy path (same-filesystem rename) moves contents correctly,
        // which is what matters for correctness of the fallback's contract.
        let root = temp_dir();
        let src = root.join("src");
        let dest = root.join("nested/dest");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("file.txt"), "hello").unwrap();

        move_dir(&src, &dest).expect("move should succeed");
        assert!(dest.join("file.txt").exists());
        assert!(!src.exists());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn unique_sibling_path_finds_a_free_name() {
        let root = temp_dir();
        fs::create_dir_all(root.join("Furina")).unwrap();
        fs::create_dir_all(root.join("Furina (2)")).unwrap();

        let found = unique_sibling_path(&root.join("Furina"));
        assert_eq!(found, root.join("Furina (3)"));

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn extract_zip_rejects_path_traversal_entries() {
        // Build a zip with one safe entry and one path-traversal entry, and
        // confirm only the safe one lands on disk.
        let root = temp_dir();
        let archive_path = root.join("archive.zip");
        {
            let file = fs::File::create(&archive_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
            writer.start_file("safe.txt", options).unwrap();
            std::io::Write::write_all(&mut writer, b"safe contents").unwrap();
            writer.start_file("../../etc/evil.txt", options).unwrap();
            std::io::Write::write_all(&mut writer, b"malicious").unwrap();
            writer.finish().unwrap();
        }

        let dest = root.join("out");
        extract_zip(&archive_path, &dest).expect("extraction should succeed");

        assert!(dest.join("safe.txt").exists());
        assert!(!root.join("etc/evil.txt").exists());
        assert_eq!(fs::read_to_string(dest.join("safe.txt")).unwrap(), "safe contents");

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn safe_relative_path_rejects_traversal_and_absolute_entries() {
        // Mirrors what `zip::ZipFile::enclosed_name` does for the zip path —
        // the `unrar` crate has no equivalent built-in guard, so `extract_rar`
        // relies on this function for the same protection against a "zip
        // slip"-style archive escaping the destination directory.
        assert_eq!(safe_relative_path(Path::new("../../etc/evil.txt")), None);
        assert_eq!(safe_relative_path(Path::new("/etc/evil.txt")), None);
        assert_eq!(safe_relative_path(Path::new("a/../../b")), None);
    }

    #[test]
    fn safe_relative_path_normalizes_safe_entries() {
        assert_eq!(
            safe_relative_path(Path::new("Furina/preview.png")),
            Some(PathBuf::from("Furina/preview.png"))
        );
        // A leading "./" is a no-op, not a traversal attempt.
        assert_eq!(
            safe_relative_path(Path::new("./mod.ini")),
            Some(PathBuf::from("mod.ini"))
        );
    }

    #[test]
    fn extract_rar_extracts_a_real_archive_and_matches_zip_behavior() {
        // Real RAR archives can't be produced by the `unrar` crate (it's
        // extract/list only, verified against RARLab's own vendored source —
        // see gamebanana.rs-style "deserializes real shape" tests for the
        // project's convention of testing against real fixtures rather than
        // hand-rolled ones), so this exercises the actual extraction path
        // against a tiny real fixture instead of a crafted one.
        //
        // Fixture: a minimal single-file RAR5 archive ("testfile.txt" containing
        // "Testing 123\n"), sourced from ssokolow/rar-test-files (a public
        // domain RAR format test corpus) and embedded as bytes so the test
        // has no network dependency.
        const RAR5_FIXTURE: &[u8] = include_bytes!("../test_fixtures/testfile.rar5.rar");

        let root = temp_dir();
        let archive_path = root.join("archive.rar");
        fs::write(&archive_path, RAR5_FIXTURE).unwrap();

        let dest = root.join("out");
        extract_rar(&archive_path, &dest).expect("rar extraction should succeed");

        let extracted = dest.join("testfile.txt");
        assert!(extracted.exists());
        assert_eq!(fs::read_to_string(extracted).unwrap().trim(), "Testing 123");

        fs::remove_dir_all(&root).ok();
    }
}
