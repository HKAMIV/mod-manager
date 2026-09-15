export type GameId =
  | "wuthering-waves"
  | "genshin-impact"
  | "zenless-zone-zero"
  | "honkai-star-rail"
  | "arknights-endfield";

export interface GameConfig {
  id: GameId;
  name: string;
  mod_path: string | null;
  enabled: boolean;
}

export interface Settings {
  active_game: GameId;
  games: GameConfig[];
  nsfw_filter: boolean;
  auto_reload: boolean;
  theme: string;
}

export type View = "local" | "online" | "downloads" | "settings";

export interface ModInfo {
  id: string;
  key: string;
  name: string;
  folder_name: string;
  path: string;
  category: string;
  enabled: boolean;
  preview_path: string | null;
  file_count: number;
  size_bytes: number;
  modified_at: number | null;
  /** True when this mod is managed under the Phase 12 symlink layout
   *  (managed_src / managed_tgt). False = legacy DISABLED_ rename model. */
  using_symlink_layout: boolean;
}

// --- Phase 12: Symlink layout ---

/** Current layout state of a game's mod directory. */
export interface LayoutStatus {
  mod_path: string;
  /** True when managed_src already exists — symlink layout is active. */
  is_symlink_layout: boolean;
  /** True when legacy DISABLED_-style mods are present and not yet migrated. */
  has_legacy_mods: boolean;
  /** Number of legacy mods that would be migrated. */
  legacy_mod_count: number;
}

/** Per-mod outcome entry inside MigrationResult. */
export interface ModMigrationEntry {
  key: string;
  was_enabled: boolean;
  success: boolean;
  error: string | null;
}

/** Result returned by migrate_to_symlink_layout. */
export interface MigrationResult {
  migrated_count: number;
  skipped_count: number;
  errors: string[];
  entries: ModMigrationEntry[];
  /** ID of the restore point taken before migration, if any. */
  restore_point_id: string | null;
}

// --- Manual install ---

/** Returned by install_mod_from_folder and install_mod_from_archive. */
export interface InstallResult {
  mod_info: ModInfo;
  installed_path: string;
}

export type ModStatusFilter = "all" | "enabled" | "disabled";

export interface ToggleTarget {
  path: string;
  category: string;
}

export interface BatchToggleResult {
  updated: ModInfo[];
  errors: string[];
}

export interface BatchDeleteResult {
  deleted_keys: string[];
  errors: string[];
}

export interface Preset {
  id: string;
  name: string;
  enabled_keys: string[];
  created_at: number;
  hotkey: string | null;
}

export interface ApplyPresetResult {
  enabled_count: number;
  disabled_count: number;
  errors: string[];
}

/**
 * Categories that are exempt from conflict detection. Character mods are
 * near-universally full model/texture replacements, so 2+ enabled mods in the
 * same character category is a strong signal of a conflict. UI/Other/
 * Uncategorized mods (fonts, reshades, UI tweaks) are typically additive and
 * routinely coexist, so they're never flagged. Matched case-insensitively
 * against ModInfo.category.
 */
export const CONFLICT_EXEMPT_CATEGORIES = ["ui", "other", "uncategorized"];

export function isConflictExemptCategory(category: string): boolean {
  return CONFLICT_EXEMPT_CATEGORIES.includes(category.trim().toLowerCase());
}

/** A category with more than one enabled mod — a likely (not certain) conflict. */
export interface CategoryConflict {
  category: string;
  mods: ModInfo[];
}

export interface SnapshotEntry {
  key: string;
  category: string;
  enabled: boolean;
}

export interface RestorePoint {
  id: string;
  name: string;
  created_at: number;
  mods: SnapshotEntry[];
  has_file_backup: boolean;
  backup_size_bytes: number;
}

export interface RestoreResult {
  enabled_count: number;
  disabled_count: number;
  recreated_count: number;
  unrecoverable: string[];
  errors: string[];
}

// --- GameBanana online mode ---

/** Sort orders confirmed to work against GameBanana's Subfeed endpoint. */
export type GbSortOrder = "default" | "new" | "updated";

export interface GbModSummary {
  id: number;
  name: string;
  profile_url: string;
  thumbnail_url: string | null;
  submitter_name: string | null;
  submitter_avatar_url: string | null;
  category: string | null;
  sub_category: string | null;
  like_count: number;
  view_count: number;
  date_added: number | null;
  date_updated: number | null;
  featured: boolean;
  likely_nsfw: boolean;
}

export interface GbBrowseResult {
  mods: GbModSummary[];
  total_count: number;
  per_page: number;
  is_complete: boolean;
}

/**
 * A single node in GameBanana's category tree (e.g. "Skins" -> "Characters"
 * -> "Furina"). Tree depth varies per game — some go straight from the root
 * category to individual characters, others have an intermediate grouping —
 * so this is browsed one level at a time rather than assumed to be a fixed
 * "character" concept.
 */
export interface GbCategoryNode {
  id: number;
  name: string;
  item_count: number;
  has_children: boolean;
  icon_url: string | null;
}

export interface GbModFile {
  id: number;
  file_name: string;
  filesize_bytes: number;
  date_added: number | null;
  download_url: string;
  md5_checksum: string | null;
  version: string | null;
  description: string | null;
  is_archived: boolean;
  av_result: string | null;
}

export interface GbModDetail {
  id: number;
  name: string;
  profile_url: string;
  description_html: string | null;
  images: string[];
  submitter_name: string | null;
  submitter_avatar_url: string | null;
  category: string | null;
  files: GbModFile[];
}

// --- Download manager (Phase 7) ---

export type DownloadStatus =
  | { state: "queued" }
  | { state: "downloading" }
  | { state: "extracting" }
  | { state: "waiting_for_conflict"; existing_path: string }
  | { state: "installing" }
  | { state: "completed"; installed_path: string }
  | { state: "failed"; message: string }
  | { state: "cancelled" };

export interface DownloadItem {
  id: string;
  game_id: GameId;
  mod_id: number;
  mod_name: string;
  file_id: number;
  file_name: string;
  download_url: string;
  total_bytes: number;
  downloaded_bytes: number;
  category_hint: string | null;
  preview_image_url: string | null;
  mod_path: string;
  status: DownloadStatus;
  created_at: number;
  updated_at: number;
}

/** Lightweight progress tick, emitted far more often than full DownloadItem
 * updates so the UI can animate a progress bar cheaply. */
export interface DownloadProgress {
  id: string;
  downloaded_bytes: number;
  total_bytes: number;
}

export type ConflictResolution = "overwrite" | "rename" | "cancel";

// --- Update tracking (Phase 8) ---

/** Persisted link between an installed mod and its GameBanana origin. */
export interface ModOrigin {
  mod_key: string;
  gb_mod_id: number;
  installed_file_id: number;
  installed_version: string | null;
  installed_at: number;
}

/** Returned for each mod that has a newer version available on GameBanana. */
export interface UpdateInfo {
  mod_key: string;
  gb_mod_id: number;
  mod_name: string;
  installed_file_id: number;
  installed_version: string | null;
  latest_file_id: number;
  latest_version: string | null;
  latest_file_name: string;
  latest_download_url: string;
  latest_filesize: number;
}
