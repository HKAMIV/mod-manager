use crate::commands::mods::resolve_mod_path;
use crate::downloads::{self, ConflictResolution, DownloadItem};
use crate::state::AppState;
use tauri::{AppHandle, State};

/// List every download (queued, in-flight, and history) for the current
/// process. Not filtered by game — downloads persist and display across the
/// whole app the same way restore points/presets are per-game but the
/// downloads manifest itself is shared.
#[tauri::command]
pub fn list_downloads() -> Vec<DownloadItem> {
    downloads::list()
}

/// Queue a GameBanana file for download + auto-install into the given
/// game's configured mod directory. `category_hint` should be the mod's
/// leaf category name (e.g. "Furina") if known, used to place the mod under
/// a matching subfolder rather than directly in the mod root.
#[tauri::command]
pub fn queue_download(
    state: State<AppState>,
    game_id: String,
    mod_id: u32,
    mod_name: String,
    file_id: u64,
    file_name: String,
    download_url: String,
    total_bytes: u64,
    category_hint: Option<String>,
    preview_image_url: Option<String>,
) -> Result<DownloadItem, String> {
    let mod_path = resolve_mod_path(&state, &game_id)?;
    downloads::queue(
        game_id,
        mod_path,
        mod_id,
        mod_name,
        file_id,
        file_name,
        download_url,
        total_bytes,
        category_hint,
        preview_image_url,
    )
}

#[tauri::command]
pub fn cancel_download(app: AppHandle, id: String) -> Result<(), String> {
    downloads::cancel(&app, &id)
}

#[tauri::command]
pub fn retry_download(app: AppHandle, id: String) -> Result<(), String> {
    downloads::retry(&app, &id)
}

#[tauri::command]
pub fn remove_download(id: String) -> Result<(), String> {
    downloads::remove(&id)
}

/// Resolve a download that's paused because its computed install path
/// already exists on disk (see `DownloadStatus::WaitingForConflict`).
#[tauri::command]
pub async fn resolve_download_conflict(
    app: AppHandle,
    id: String,
    action: ConflictResolution,
) -> Result<(), String> {
    downloads::resolve_conflict(app, id, action).await
}
