use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Event name emitted to the frontend whenever a watched mod directory changes
/// on disk. Payload is the game_id whose directory changed.
pub const MODS_CHANGED_EVENT: &str = "mods-changed";

/// Registry of active filesystem watchers, keyed by game_id, so we can stop a
/// watcher when the user changes a game's mod path or disables auto-reload.
pub struct WatcherRegistry {
    watchers: Mutex<HashMap<String, RecommendedWatcher>>,
}

impl WatcherRegistry {
    fn new() -> Self {
        Self {
            watchers: Mutex::new(HashMap::new()),
        }
    }
}

static REGISTRY: std::sync::OnceLock<WatcherRegistry> = std::sync::OnceLock::new();

fn registry() -> &'static WatcherRegistry {
    REGISTRY.get_or_init(WatcherRegistry::new)
}

/// Called once at app startup. Currently a no-op hook reserved for future
/// initialization (e.g. restoring watchers for games with auto_reload enabled).
pub fn init(_app: AppHandle) {}

/// Start watching `path` for changes and emit `MODS_CHANGED_EVENT` (debounced)
/// whenever something changes inside it. Replaces any existing watcher for the
/// same `game_id`.
pub fn watch(app: AppHandle, game_id: String, path: String) -> Result<(), String> {
    let (tx, rx) = channel::<Event>();

    let mut watcher: RecommendedWatcher =
        notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })
        .map_err(|e| format!("Failed to create watcher: {}", e))?;

    watcher
        .watch(std::path::Path::new(&path), RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch {}: {}", path, e))?;

    {
        let mut watchers = registry().watchers.lock().map_err(|e| e.to_string())?;
        watchers.insert(game_id.clone(), watcher);
    }

    // Debounce: collect events for a short window before notifying the frontend,
    // so a batch of file operations (e.g. extracting an archive) triggers one
    // refresh instead of dozens.
    std::thread::spawn(move || {
        loop {
            match rx.recv_timeout(Duration::from_secs(3600)) {
                Ok(_first_event) => {
                    // Drain any further events that arrive within the debounce window.
                    while rx.recv_timeout(Duration::from_millis(500)).is_ok() {}
                    let _ = app.emit(MODS_CHANGED_EVENT, &game_id);
                }
                Err(_) => break, // sender dropped (watcher removed) or timed out
            }
        }
    });

    Ok(())
}

/// Stop watching the directory associated with `game_id`, if any.
pub fn unwatch(game_id: &str) -> Result<(), String> {
    let mut watchers = registry().watchers.lock().map_err(|e| e.to_string())?;
    watchers.remove(game_id);
    Ok(())
}
