use crate::gamebanana::{self, BrowseResult, CategoryNode, ModDetail, SortOrder};

/// Browse GameBanana mods for a game. `page` is 1-indexed. `sort` must be one
/// of "default", "new", "updated" (other values GameBanana's API itself
/// rejects with a 400). `category_id` optionally scopes to a single root
/// category. If `search` is a non-empty string, uses GameBanana's real
/// search endpoint instead of the browse feed (see gamebanana::browse_mods).
#[tauri::command]
pub async fn browse_gamebanana_mods(
    game_id: String,
    page: u32,
    sort: SortOrder,
    category_id: Option<u32>,
    search: Option<String>,
) -> Result<BrowseResult, String> {
    gamebanana::browse_mods(&game_id, page, sort, category_id, search.as_deref()).await
}

/// Fetch full detail (description, images, downloadable files) for a single
/// GameBanana mod by its numeric id.
#[tauri::command]
pub async fn get_gamebanana_mod_detail(mod_id: u32) -> Result<ModDetail, String> {
    gamebanana::get_mod_detail(mod_id).await
}

/// Fetch one level of a game's GameBanana category tree (e.g. root categories
/// like "Skins"/"UI", or — passing `parent_category_id` — the children of one
/// of those, drilling down toward individual characters). See
/// `gamebanana::get_mod_categories` for why this walks one level at a time
/// rather than returning a full tree: depth varies per game and per-level
/// item counts are what the UI needs to decide "browse into this" vs "filter
/// by this leaf".
#[tauri::command]
pub async fn get_gamebanana_categories(
    game_id: String,
    parent_category_id: Option<u32>,
) -> Result<Vec<CategoryNode>, String> {
    gamebanana::get_mod_categories(&game_id, parent_category_id).await
}
