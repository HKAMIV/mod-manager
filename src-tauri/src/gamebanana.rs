use serde::{Deserialize, Serialize};

const API_BASE: &str = "https://gamebanana.com/apiv11";

/// GameBanana's numeric game IDs for the titles this app supports. These are
/// stable identifiers on GameBanana's side (confirmed against gamebanana.com/games/:id)
/// and have no relation to our own `GameId` strings.
pub fn gamebanana_game_id(game_id: &str) -> Option<u32> {
    match game_id {
        "genshin-impact" => Some(8552),
        "honkai-star-rail" => Some(18366),
        "wuthering-waves" => Some(20357),
        "zenless-zone-zero" => Some(19567),
        "arknights-endfield" => Some(21842),
        _ => None,
    }
}

/// Sort orders confirmed to work against Game/:id/Subfeed. GameBanana's API
/// silently 400s on invalid `_sSort` values (e.g. "popular", "downloads"
/// don't work despite being documented informally elsewhere), so this is
/// deliberately narrow to what's been verified.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Default,
    New,
    Updated,
}

impl SortOrder {
    /// Query value for the Subfeed endpoint (used when browsing a game with no
    /// category filter applied).
    fn as_subfeed_query_value(&self) -> &'static str {
        match self {
            SortOrder::Default => "default",
            SortOrder::New => "new",
            SortOrder::Updated => "updated",
        }
    }

    /// Sort alias for the Mod/Index endpoint (used when a category filter is
    /// applied — Mod/Index doesn't recognize Subfeed's "default"/"new"/"updated"
    /// literals, it uses its own `Generic_*` alias set, confirmed via
    /// Mod/ListFilterConfig). Default sort is expressed by omitting `_sSort`
    /// entirely, since Mod/Index has no "default" alias.
    fn as_mod_index_query_value(&self) -> Option<&'static str> {
        match self {
            SortOrder::Default => None,
            SortOrder::New => Some("Generic_Newest"),
            SortOrder::Updated => Some("Generic_LatestUpdated"),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct SubfeedResponse {
    #[serde(rename = "_aMetadata")]
    metadata: SubfeedMetadata,
    #[serde(rename = "_aRecords")]
    records: Vec<SubfeedRecord>,
}

#[derive(Debug, Clone, Deserialize)]
struct SubfeedMetadata {
    #[serde(rename = "_nRecordCount")]
    n_record_count: u32,
    #[serde(rename = "_nPerpage")]
    n_perpage: u32,
    #[serde(rename = "_bIsComplete")]
    b_is_complete: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct SubfeedRecord {
    #[serde(rename = "_idRow")]
    id_row: u32,
    #[serde(rename = "_sModelName")]
    model_name: String,
    #[serde(rename = "_sName")]
    name: String,
    #[serde(rename = "_sProfileUrl")]
    profile_url: String,
    #[serde(rename = "_tsDateAdded")]
    ts_date_added: Option<u64>,
    #[serde(rename = "_tsDateUpdated")]
    ts_date_updated: Option<u64>,
    #[serde(rename = "_aPreviewMedia", default)]
    preview_media: Option<PreviewMedia>,
    #[serde(rename = "_aSubmitter", default)]
    submitter: Option<Submitter>,
    #[serde(rename = "_aRootCategory", default)]
    root_category: Option<Category>,
    #[serde(rename = "_aSubCategory", default)]
    sub_category: Option<Category>,
    #[serde(rename = "_nLikeCount", default)]
    like_count: u32,
    #[serde(rename = "_nViewCount", default)]
    view_count: u32,
    #[serde(rename = "_bHasContentRatings", default)]
    has_content_ratings: bool,
    #[serde(rename = "_sInitialVisibility", default)]
    initial_visibility: Option<String>,
    #[serde(rename = "_bWasFeatured", default)]
    was_featured: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct PreviewMedia {
    #[serde(rename = "_aImages", default)]
    images: Vec<PreviewImage>,
}

#[derive(Debug, Clone, Deserialize)]
struct PreviewImage {
    #[serde(rename = "_sBaseUrl")]
    base_url: String,
    #[serde(rename = "_sFile220", default)]
    file_220: Option<String>,
    #[serde(rename = "_sFile530", default)]
    file_530: Option<String>,
    #[serde(rename = "_sFile100", default)]
    file_100: Option<String>,
    #[serde(rename = "_sFile")]
    file: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Submitter {
    #[serde(rename = "_sName")]
    name: String,
    #[serde(rename = "_sAvatarUrl", default)]
    avatar_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Category {
    #[serde(rename = "_sName")]
    name: String,
}

/// A single mod entry as shown in the browse grid — a trimmed, camelCase
/// projection of GameBanana's Subfeed record shape for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ModSummary {
    pub id: u32,
    pub name: String,
    pub profile_url: String,
    /// Best-effort thumbnail URL (prefers the 220px variant, falls back to
    /// 100px or the raw screenshot file), or None if the mod has no images.
    pub thumbnail_url: Option<String>,
    pub submitter_name: Option<String>,
    pub submitter_avatar_url: Option<String>,
    pub category: Option<String>,
    pub sub_category: Option<String>,
    pub like_count: u32,
    pub view_count: u32,
    pub date_added: Option<u64>,
    pub date_updated: Option<u64>,
    pub featured: bool,
    /// Best-effort heuristic: GameBanana's Subfeed doesn't expose a direct
    /// NSFW boolean, but content-rated submissions that are hidden/warn-gated
    /// behind a click-through are reliably flagged content. Used to honor
    /// the app's NSFW filter setting.
    pub likely_nsfw: bool,
}

impl From<SubfeedRecord> for ModSummary {
    fn from(r: SubfeedRecord) -> Self {
        let thumbnail_url = r.preview_media.and_then(|m| {
            m.images.into_iter().next().map(|img| {
                let file = img
                    .file_220
                    .or(img.file_100)
                    .or(img.file_530)
                    .unwrap_or(img.file);
                format!("{}/{}", img.base_url, file)
            })
        });

        let likely_nsfw = r.has_content_ratings
            && matches!(r.initial_visibility.as_deref(), Some("hide") | Some("warn"));

        Self {
            id: r.id_row,
            name: r.name,
            profile_url: r.profile_url,
            thumbnail_url,
            submitter_name: r.submitter.as_ref().map(|s| s.name.clone()),
            submitter_avatar_url: r.submitter.and_then(|s| s.avatar_url),
            category: r.root_category.map(|c| c.name),
            sub_category: r.sub_category.map(|c| c.name),
            like_count: r.like_count,
            view_count: r.view_count,
            date_added: r.ts_date_added,
            date_updated: r.ts_date_updated,
            featured: r.was_featured,
            likely_nsfw,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowseResult {
    pub mods: Vec<ModSummary>,
    pub total_count: u32,
    pub per_page: u32,
    pub is_complete: bool,
}

/// Browse mods for a game. `page` is 1-indexed. Only `Mod` records are kept —
/// Subfeed/Mod-Index also return Requests/Questions/WiPs mixed into the same
/// feed, which aren't installable mods and don't belong in a mod browser.
///
/// Three request shapes, in priority order:
/// - `search` non-empty: GameBanana's Util/Search/Results endpoint (real
///   server-side full-text search). `sort` and `category_id` don't apply here
///   — verified live that Search/Results silently ignores a category filter
///   (a search scoped to one character's category returned mods tagged with
///   *other* characters), and the endpoint ranks by relevance regardless of
///   `_sSort`.
/// - `category_id` set (no search): Mod/Index filtered by
///   `_aFilters[Generic_Category]`. This is the endpoint GameBanana's own
///   category-browse pages use (confirmed by reading their page bundle) —
///   Subfeed's `_idCategoryRow` param looks plausible but is silently a
///   no-op, verified by requesting the same category twice with and without
///   it and getting identical unfiltered results both times.
/// - neither: the existing Subfeed browse feed.
pub async fn browse_mods(
    game_id: &str,
    page: u32,
    sort: SortOrder,
    category_id: Option<u32>,
    search: Option<&str>,
) -> Result<BrowseResult, String> {
    let gb_id = gamebanana_game_id(game_id).ok_or_else(|| format!("Unknown game: {}", game_id))?;

    // Mirrors Subfeed's own default page size so pagination behaves
    // identically regardless of which endpoint served the page.
    const PER_PAGE: u32 = 15;

    let url = match (search.map(str::trim).filter(|s| !s.is_empty()), category_id) {
        (Some(query), _) => {
            let mut url = reqwest::Url::parse(&format!("{}/Util/Search/Results", API_BASE))
                .map_err(|e| e.to_string())?;
            url.query_pairs_mut()
                .append_pair("_sSearchString", query)
                .append_pair("_idGameRow", &gb_id.to_string())
                .append_pair("_sModelName", "Mod")
                .append_pair("_nPage", &page.to_string());
            url
        }
        (None, Some(cat_id)) => {
            let mut url = reqwest::Url::parse(&format!("{}/Mod/Index", API_BASE))
                .map_err(|e| e.to_string())?;
            {
                let mut pairs = url.query_pairs_mut();
                pairs
                    .append_pair("_nPage", &page.to_string())
                    .append_pair("_nPerpage", &PER_PAGE.to_string())
                    .append_pair("_aFilters[Generic_Category]", &cat_id.to_string());
                if let Some(sort_value) = sort.as_mod_index_query_value() {
                    pairs.append_pair("_sSort", sort_value);
                }
            }
            url
        }
        (None, None) => {
            let mut url = reqwest::Url::parse(&format!("{}/Game/{}/Subfeed", API_BASE, gb_id))
                .map_err(|e| e.to_string())?;
            url.query_pairs_mut()
                .append_pair("_nPage", &page.to_string())
                .append_pair("_sSort", sort.as_subfeed_query_value());
            url
        }
    };

    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to reach GameBanana: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GameBanana returned HTTP {}", response.status()));
    }

    let body: SubfeedResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse GameBanana response: {}", e))?;

    let mods: Vec<ModSummary> = body
        .records
        .into_iter()
        .filter(|r| r.model_name == "Mod")
        .map(ModSummary::from)
        .collect();

    Ok(BrowseResult {
        mods,
        total_count: body.metadata.n_record_count,
        per_page: body.metadata.n_perpage,
        is_complete: body.metadata.b_is_complete,
    })
}


// --- Mod detail ---

#[derive(Debug, Clone, Deserialize)]
struct ModDataResponse {
    #[serde(rename = "_sName")]
    name: String,
    #[serde(rename = "_sProfileUrl")]
    profile_url: String,
    #[serde(rename = "_sText", default)]
    description_html: Option<String>,
    #[serde(rename = "_aPreviewMedia", default)]
    preview_media: Option<PreviewMedia>,
    #[serde(rename = "_aSubmitter", default)]
    submitter: Option<Submitter>,
    #[serde(rename = "_aCategory", default)]
    category: Option<Category>,
    #[serde(rename = "_aFiles", default)]
    files: Vec<ModFile>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModFile {
    #[serde(rename = "_idRow")]
    id_row: u64,
    #[serde(rename = "_sFile")]
    file_name: String,
    #[serde(rename = "_nFilesize")]
    filesize: u64,
    #[serde(rename = "_tsDateAdded", default)]
    date_added: Option<u64>,
    #[serde(rename = "_sDownloadUrl")]
    download_url: String,
    #[serde(rename = "_sMd5Checksum", default)]
    md5_checksum: Option<String>,
    #[serde(rename = "_sVersion", default)]
    version: Option<String>,
    #[serde(rename = "_sDescription", default)]
    description: Option<String>,
    #[serde(rename = "_bIsArchived", default)]
    is_archived: bool,
    #[serde(rename = "_sAvResult", default)]
    av_result: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModFileInfo {
    pub id: u64,
    pub file_name: String,
    pub filesize_bytes: u64,
    pub date_added: Option<u64>,
    pub download_url: String,
    pub md5_checksum: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    /// True for older versions superseded by a newer file upload — the
    /// current/latest file is the one Subfeed and this list surface first.
    pub is_archived: bool,
    /// GameBanana's malware scan verdict for this specific file, e.g. "clean".
    /// Surfaced so the UI can warn if a scan hasn't completed or failed,
    /// rather than silently offering a potentially-unsafe download.
    pub av_result: Option<String>,
}

impl From<ModFile> for ModFileInfo {
    fn from(f: ModFile) -> Self {
        Self {
            id: f.id_row,
            file_name: f.file_name,
            filesize_bytes: f.filesize,
            date_added: f.date_added,
            download_url: f.download_url,
            md5_checksum: f.md5_checksum,
            version: f.version,
            description: f.description,
            is_archived: f.is_archived,
            av_result: f.av_result,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ModDetail {
    pub id: u32,
    pub name: String,
    pub profile_url: String,
    pub description_html: Option<String>,
    /// Full-size image URLs (screenshots gallery), best available size first.
    pub images: Vec<String>,
    pub submitter_name: Option<String>,
    pub submitter_avatar_url: Option<String>,
    pub category: Option<String>,
    pub files: Vec<ModFileInfo>,
}

pub async fn get_mod_detail(mod_id: u32) -> Result<ModDetail, String> {
    let url = format!(
        "{}/Mod/{}?_csvProperties=_sName,_sProfileUrl,_sText,_aPreviewMedia,_aSubmitter,_aCategory,_aFiles",
        API_BASE, mod_id
    );

    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("Failed to reach GameBanana: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GameBanana returned HTTP {}", response.status()));
    }

    let body: ModDataResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse GameBanana response: {}", e))?;

    let images = body
        .preview_media
        .map(|m| {
            m.images
                .into_iter()
                .map(|img| {
                    let file = img.file_530.or(img.file_220).unwrap_or(img.file);
                    format!("{}/{}", img.base_url, file)
                })
                .collect()
        })
        .unwrap_or_default();

    // Newest file (largest _idRow / first in the list) leads; older archived
    // versions still returned so the UI can offer them if needed.
    let files: Vec<ModFileInfo> = body.files.into_iter().map(ModFileInfo::from).collect();

    Ok(ModDetail {
        id: mod_id,
        name: body.name,
        profile_url: body.profile_url,
        description_html: body.description_html,
        images,
        submitter_name: body.submitter.as_ref().map(|s| s.name.clone()),
        submitter_avatar_url: body.submitter.and_then(|s| s.avatar_url),
        category: body.category.map(|c| c.name),
        files,
    })
}

// --- Category browsing ---

#[derive(Debug, Clone, Deserialize)]
struct CategoryRecord {
    #[serde(rename = "_idRow")]
    id_row: u32,
    #[serde(rename = "_sName")]
    name: String,
    #[serde(rename = "_nItemCount")]
    item_count: u32,
    /// Number of child categories nested under this one. 0 means this is a
    /// leaf node (e.g. an individual character) — drilling in further would
    /// just re-request the same empty list, so the frontend uses this to
    /// decide whether a category is "browse into" vs "select as a filter".
    #[serde(rename = "_nCategoryCount")]
    category_count: u32,
    #[serde(rename = "_sIconUrl", default)]
    icon_url: Option<String>,
}

/// A single node in GameBanana's category tree (e.g. "Skins" -> "Characters"
/// -> "Furina"). Depth of the tree varies per game — some go straight from
/// the root category to individual characters, others have an intermediate
/// "Characters" grouping — so this is a generic recursive node rather than a
/// fixed "character" concept.
#[derive(Debug, Clone, Serialize)]
pub struct CategoryNode {
    pub id: u32,
    pub name: String,
    pub item_count: u32,
    pub has_children: bool,
    pub icon_url: Option<String>,
}

impl From<CategoryRecord> for CategoryNode {
    fn from(r: CategoryRecord) -> Self {
        Self {
            id: r.id_row,
            name: r.name,
            item_count: r.item_count,
            has_children: r.category_count > 0,
            icon_url: r.icon_url,
        }
    }
}

/// Fetch one level of a game's category tree. `parent_category_id` of `None`
/// returns the game's root categories (Skins, UI, Weapons, etc.); passing a
/// category's own id returns its children (e.g. passing "Skins" returns
/// "Characters"/"Weapons"/"Other-Misc" for a game with that intermediate
/// grouping, or the character list directly for a game without one).
///
/// This is `Mod/Categories`, not the `Game/:id/Subfeed`'s `_idCategoryRow`
/// param — that param is silently ignored by Subfeed (verified live: passing
/// it returns the same unfiltered result set as omitting it entirely).
///
/// `_idGameRow` and `_idCategoryRow` cannot be combined on this endpoint
/// either — verified live that passing both just re-returns the game's root
/// categories instead of the requested category's children, which is why
/// drilling into "Skins" looked like it did nothing (it silently handed back
/// "Skins" again). `_idGameRow` is only needed for the root-level fetch;
/// every deeper level is looked up by `_idCategoryRow` alone. Category ids
/// are globally unique across games (confirmed live — a Wuthering Waves
/// category id fetched with no game scope returns Wuthering Waves data, not
/// a different game's category that happens to share the id), so this is
/// safe.
pub async fn get_mod_categories(
    game_id: &str,
    parent_category_id: Option<u32>,
) -> Result<Vec<CategoryNode>, String> {
    let mut url =
        reqwest::Url::parse(&format!("{}/Mod/Categories", API_BASE)).map_err(|e| e.to_string())?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("_sSort", "a_to_z");
        match parent_category_id {
            Some(cat_id) => {
                pairs.append_pair("_idCategoryRow", &cat_id.to_string());
            }
            None => {
                let gb_id = gamebanana_game_id(game_id)
                    .ok_or_else(|| format!("Unknown game: {}", game_id))?;
                pairs.append_pair("_idGameRow", &gb_id.to_string());
            }
        }
    }

    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to reach GameBanana: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GameBanana returned HTTP {}", response.status()));
    }

    let records: Vec<CategoryRecord> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse GameBanana response: {}", e))?;

    Ok(records.into_iter().map(CategoryNode::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_supported_games_to_gamebanana_ids() {
        assert_eq!(gamebanana_game_id("genshin-impact"), Some(8552));
        assert_eq!(gamebanana_game_id("honkai-star-rail"), Some(18366));
        assert_eq!(gamebanana_game_id("wuthering-waves"), Some(20357));
        assert_eq!(gamebanana_game_id("zenless-zone-zero"), Some(19567));
        assert_eq!(gamebanana_game_id("arknights-endfield"), Some(21842));
        assert_eq!(gamebanana_game_id("not-a-real-game"), None);
    }

    #[test]
    fn sort_order_query_values_match_gamebanana_api() {
        // These three are the only values confirmed against the live API —
        // "popular"/"downloads" 400. Locking the query strings in a test
        // guards against a future refactor silently changing them.
        assert_eq!(SortOrder::Default.as_subfeed_query_value(), "default");
        assert_eq!(SortOrder::New.as_subfeed_query_value(), "new");
        assert_eq!(SortOrder::Updated.as_subfeed_query_value(), "updated");
    }

    #[test]
    fn mod_index_sort_aliases_match_list_filter_config() {
        // Confirmed against the live Mod/ListFilterConfig response — Mod/Index
        // (used for category-filtered browsing) has its own alias set,
        // distinct from Subfeed's "default"/"new"/"updated" literals, which
        // Mod/Index rejects with UNKNOWN_SORT.
        assert_eq!(SortOrder::Default.as_mod_index_query_value(), None);
        assert_eq!(
            SortOrder::New.as_mod_index_query_value(),
            Some("Generic_Newest")
        );
        assert_eq!(
            SortOrder::Updated.as_mod_index_query_value(),
            Some("Generic_LatestUpdated")
        );
    }

    fn sample_record(has_ratings: bool, visibility: Option<&str>) -> SubfeedRecord {
        SubfeedRecord {
            id_row: 12345,
            model_name: "Mod".to_string(),
            name: "Test Mod".to_string(),
            profile_url: "https://gamebanana.com/mods/12345".to_string(),
            ts_date_added: Some(1000),
            ts_date_updated: Some(2000),
            preview_media: Some(PreviewMedia {
                images: vec![PreviewImage {
                    base_url: "https://images.gamebanana.com/img/ss/mods".to_string(),
                    file_220: Some("220-90_abc.jpg".to_string()),
                    file_530: Some("530-90_abc.jpg".to_string()),
                    file_100: Some("100-90_abc.jpg".to_string()),
                    file: "abc.jpg".to_string(),
                }],
            }),
            submitter: Some(Submitter {
                name: "SomeModder".to_string(),
                avatar_url: Some("https://images.gamebanana.com/img/av/abc.jpg".to_string()),
            }),
            root_category: Some(Category {
                name: "Skins".to_string(),
            }),
            sub_category: Some(Category {
                name: "Furina".to_string(),
            }),
            like_count: 42,
            view_count: 999,
            has_content_ratings: has_ratings,
            initial_visibility: visibility.map(String::from),
            was_featured: true,
        }
    }

    #[test]
    fn thumbnail_prefers_220px_variant() {
        let summary: ModSummary = sample_record(false, Some("show")).into();
        assert_eq!(
            summary.thumbnail_url,
            Some("https://images.gamebanana.com/img/ss/mods/220-90_abc.jpg".to_string())
        );
    }

    #[test]
    fn thumbnail_falls_back_when_220_missing() {
        let mut record = sample_record(false, Some("show"));
        if let Some(media) = record.preview_media.as_mut() {
            media.images[0].file_220 = None;
        }
        let summary: ModSummary = record.into();
        assert_eq!(
            summary.thumbnail_url,
            Some("https://images.gamebanana.com/img/ss/mods/100-90_abc.jpg".to_string())
        );
    }

    #[test]
    fn no_preview_media_means_no_thumbnail() {
        let mut record = sample_record(false, Some("show"));
        record.preview_media = None;
        let summary: ModSummary = record.into();
        assert_eq!(summary.thumbnail_url, None);
    }

    #[test]
    fn nsfw_heuristic_flags_hidden_content_rated_mods() {
        let summary: ModSummary = sample_record(true, Some("hide")).into();
        assert!(summary.likely_nsfw);

        let summary: ModSummary = sample_record(true, Some("warn")).into();
        assert!(summary.likely_nsfw);
    }

    #[test]
    fn nsfw_heuristic_does_not_flag_visible_content() {
        // Content-rated but shown outright (e.g. a mild rating) shouldn't be
        // treated as NSFW just because it has *some* rating on file.
        let summary: ModSummary = sample_record(true, Some("show")).into();
        assert!(!summary.likely_nsfw);

        // No content ratings at all is never NSFW regardless of visibility.
        let summary: ModSummary = sample_record(false, Some("hide")).into();
        assert!(!summary.likely_nsfw);
    }

    #[test]
    fn maps_record_fields_through_to_summary() {
        let summary: ModSummary = sample_record(false, Some("show")).into();
        assert_eq!(summary.id, 12345);
        assert_eq!(summary.name, "Test Mod");
        assert_eq!(summary.submitter_name, Some("SomeModder".to_string()));
        assert_eq!(summary.category, Some("Skins".to_string()));
        assert_eq!(summary.sub_category, Some("Furina".to_string()));
        assert_eq!(summary.like_count, 42);
        assert_eq!(summary.view_count, 999);
        assert!(summary.featured);
    }

    #[test]
    fn deserializes_real_subfeed_shape() {
        // Trimmed but structurally faithful sample of an actual Subfeed
        // response, to catch renames/schema drift in the live API early.
        let json = r#"{
            "_aMetadata": { "_nRecordCount": 100, "_nPerpage": 15, "_bIsComplete": false },
            "_aRecords": [
                {
                    "_idRow": 626136,
                    "_sModelName": "Mod",
                    "_sName": "FocusLines 7.0+",
                    "_sProfileUrl": "https://gamebanana.com/mods/626136",
                    "_tsDateAdded": 1760140607,
                    "_tsDateModified": 1787783713,
                    "_bHasFiles": true,
                    "_aPreviewMedia": { "_aImages": [] },
                    "_aSubmitter": {
                        "_idRow": 2827149,
                        "_sName": "Momoka_",
                        "_bIsOnline": false,
                        "_sProfileUrl": "https://gamebanana.com/members/2827149",
                        "_sAvatarUrl": "https://images.gamebanana.com/img/av/x.jpg"
                    },
                    "_aRootCategory": {
                        "_sName": "Other/Misc",
                        "_sProfileUrl": "https://gamebanana.com/mods/cats/12526",
                        "_sIconUrl": ""
                    },
                    "_nLikeCount": 318,
                    "_nViewCount": 57605,
                    "_bHasContentRatings": false,
                    "_sInitialVisibility": "show",
                    "_bWasFeatured": false
                },
                {
                    "_idRow": 96010,
                    "_sModelName": "Request",
                    "_sName": "Some request",
                    "_sProfileUrl": "https://gamebanana.com/requests/96010",
                    "_bHasFiles": false,
                    "_nViewCount": 70,
                    "_bHasContentRatings": false,
                    "_bWasFeatured": false
                }
            ]
        }"#;

        let parsed: SubfeedResponse = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(parsed.metadata.n_record_count, 100);
        assert_eq!(parsed.records.len(), 2);

        // browse_mods filters to Mod-only; verify that logic directly here
        // since it operates on this same deserialized shape.
        let mods: Vec<ModSummary> = parsed
            .records
            .into_iter()
            .filter(|r| r.model_name == "Mod")
            .map(ModSummary::from)
            .collect();
        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].name, "FocusLines 7.0+");
    }

    #[test]
    fn deserializes_real_mod_detail_shape() {
        let json = r#"{
            "_sName": "FocusLines 7.0+",
            "_sProfileUrl": "https://gamebanana.com/mods/626136",
            "_sText": "<strong>Hello!</strong>",
            "_aPreviewMedia": {
                "_aImages": [
                    {
                        "_sBaseUrl": "https://images.gamebanana.com/img/ss/mods",
                        "_sFile": "abc.jpg",
                        "_sFile530": "530-90_abc.jpg"
                    }
                ]
            },
            "_aSubmitter": {
                "_sName": "Momoka_",
                "_sAvatarUrl": "https://images.gamebanana.com/img/av/x.jpg"
            },
            "_aCategory": {
                "_sName": "Other/Misc"
            },
            "_aFiles": [
                {
                    "_idRow": 1798362,
                    "_sFile": "focuslines_703.7z",
                    "_nFilesize": 92558756,
                    "_tsDateAdded": 1787783417,
                    "_sDownloadUrl": "https://gamebanana.com/dl/1798362",
                    "_sMd5Checksum": "32deadbe83aa41d2d9ad2c0ce13f421e",
                    "_bIsArchived": false,
                    "_sAvResult": "clean"
                }
            ]
        }"#;

        let parsed: ModDataResponse = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(parsed.name, "FocusLines 7.0+");
        assert_eq!(parsed.description_html, Some("<strong>Hello!</strong>".to_string()));
        assert_eq!(parsed.files.len(), 1);
        assert_eq!(parsed.files[0].file_name, "focuslines_703.7z");
        assert_eq!(parsed.files[0].av_result, Some("clean".to_string()));
        assert!(!parsed.files[0].is_archived);
    }

    #[test]
    fn deserializes_real_mod_categories_shape() {
        // Trimmed but structurally faithful sample of an actual Mod/Categories
        // response (Genshin Impact's root categories).
        let json = r#"[
            {
                "_idRow": 17510,
                "_sName": "Skins",
                "_nItemCount": 12207,
                "_nCategoryCount": 3,
                "_sUrl": "https://gamebanana.com/mods/cats/17510",
                "_bIsObsolete": false,
                "_sIconUrl": "https://images.gamebanana.com/img/ico/ModCategory/62ad1d74107fa.png"
            },
            {
                "_idRow": 24279,
                "_sName": "Waverider",
                "_nItemCount": 11,
                "_nCategoryCount": 0,
                "_sUrl": "https://gamebanana.com/mods/cats/24279",
                "_bIsObsolete": false,
                "_sIconUrl": "https://images.gamebanana.com/img/ico/ModCategory/64c717886221c.png"
            }
        ]"#;

        let records: Vec<CategoryRecord> = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(records.len(), 2);

        let nodes: Vec<CategoryNode> = records.into_iter().map(CategoryNode::from).collect();
        assert_eq!(nodes[0].id, 17510);
        assert_eq!(nodes[0].name, "Skins");
        assert!(nodes[0].has_children); // _nCategoryCount: 3
        assert!(!nodes[1].has_children); // _nCategoryCount: 0, a leaf
    }
}
