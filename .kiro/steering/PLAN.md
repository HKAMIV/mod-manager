# Mod Manager for Arch Linux / SteamOS — Feature Set & Build Plan

> A desktop application for managing game mods (Wuthering Waves, Genshin Impact, Zenless Zone Zero, Honkai Star Rail, Arknights Endfield) built with Tauri v2, targeting Arch Linux and SteamOS (Steam Deck).

---

## Design System — LOCKED

The visual identity ("Void & Astral") is locked in and enforced via `.kiro/steering/design-system.md`.

- [x] Core palette defined (surfaces, cyan primary accent, purple secondary, gold highlight)
- [x] Per-game dynamic accent system (`data-game` attribute + CSS variable overrides)
- [x] Tailwind tokens wired (`tailwind.config.js` + `src/styles/globals.css`)
- [x] Existing components updated to use tokens (Sidebar, GameSelector, LocalView, SettingsView)
- [x] Steering doc created so all future phases follow the same tokens automatically

---

## Full Feature Set

### 1. Local Mod Management

- [x] Directory scanning — auto-detect installed mods from game mod folder
- [x] Categorized display with folder-based organization
- [x] One-click enable/disable toggle
- [x] Batch operations via presets
- [x] Preview images per mod
- [x] Filter by status (enabled/disabled), character, category
- [x] Search by name or category
- [x] Mod deletion
- [x] Conflict detection & resolution (passive, category-based heuristic — see Phase 4)
- [x] Restore points — backup & restore mod state snapshots (state-only or full file backup)

### 2. Online Mode (GameBanana Integration)

- [x] Browse GameBanana mod library with search
- [x] Category filtering (Characters, Skins, UI, etc.) — drill-down category
      browser, added after initial Phase 6 delivery (see Phase 6 notes)
- [x] Mod detail pages with descriptions & images
- [x] Direct download with preview image (Phase 7)
- [x] Update tracking — detect newer versions of installed mods (Phase 8)

### 3. Download Manager

- [x] Real-time download progress tracking
- [x] Batch/queue downloads (sequential)
- [x] Download history (completed, failed)
- [x] Auto-installation post-download (extract + place in correct directory)

### 4. File Operations

- [x] Smart archive extraction (ZIP via `zip` crate, 7z via `sevenz-rust2`, RAR via `unrar`)
- [x] Conflict resolution during extraction (overwrite / install alongside / cancel)
- [x] Auto directory structure creation (category subfolder created on demand)

### 5. Multi-Game Support

- [x] Game switcher UI
- [x] Per-game mod directories and settings
- [x] Supported: Wuthering Waves, Genshin Impact, ZZZ, HSR, Arknights Endfield

### 6. UI/UX

- [x] Multi-panel layout (sidebar nav, main content, detail panel)
- [ ] Responsive/adaptive layout
- [ ] Smooth animations/transitions
- [ ] Customizable sidebar/panel configuration
- [ ] Hotkey support for presets
- [ ] Global search
- [ ] Paste-a-link to open mod in online mode
- [ ] Gamepad-friendly navigation (Steam Deck)

### 7. Configuration & Settings

- [x] Per-game path configuration
- [x] Auto-reload mod lists on filesystem changes (implemented in Phase 2 via `notify` watcher)
- [x] NSFW content filter for online browsing
- [ ] Auto-launch game via mod loader (Proton/Wine compatible)
- [x] Custom hotkeys for presets
- [ ] Import/Export entire app config
- [x] XDG-compliant config paths (`~/.config/mod-manager/`)

### 8. Self-Update System

- [ ] Auto-update checker (GitHub Releases)
- [ ] Changelog viewer
- [ ] In-app download & install of new versions

### 9. Packaging & Distribution

- [ ] Arch Linux PKGBUILD
- [ ] Flatpak manifest
- [ ] AppImage
- [ ] `.desktop` file and icon integration

---

## Tech Stack

| Layer | Choice | Reason |
|-------|--------|--------|
| Framework | Tauri v2 | Native Linux support, small binary, Rust backend |
| Frontend | React + TypeScript + Vite | Fast dev, component ecosystem |
| Styling | Tailwind CSS | Rapid UI, responsive utilities |
| State | Zustand | Lightweight, pairs well with Tauri commands |
| Archive handling | `sevenz-rust` + system `unzip` fallback | No Windows binary dependency |
| HTTP | `reqwest` (Rust) | Async downloads with progress streams |
| FS watching | `notify` crate | Cross-platform filesystem watcher |
| Hotkeys | `evdev` / `libxkbcommon` | Linux-native input handling |
| Packaging | PKGBUILD + Flatpak + AppImage | Full Arch/SteamOS coverage |

---

## Incremental Build Plan

### Phase 1 — Skeleton & Core Infrastructure [DONE]

**Goal:** Bootable Tauri app with project structure, basic navigation, settings persistence.

- [x] Initialize Tauri v2 + React + TypeScript + Vite project
- [x] Set up project directory structure (commands, state, components, types)
- [x] Basic multi-panel layout (sidebar nav, main content area, collapsible right panel)
- [x] Settings persistence — JSON config at `~/.config/mod-manager/`
- [x] Game selector — switch between supported games, store per-game paths
- [x] Theme/styling foundation — Tailwind, dark theme default
- [x] Basic routing between Local / Online / Settings views

---

### Phase 2 — Local Mod Management (Read-Only) [DONE]

**Goal:** Scan a mod directory and display mods with metadata.

- [x] Rust command: walk game mod folder, detect mod folder structure
- [x] Define mod data model (name, path, category, enabled, preview image, metadata)
- [x] Mod list UI — display mods in categorized, scrollable list
- [x] Preview image display (show `preview.png` from mod folder if present)
- [x] Search implementation (frontend text filter)
- [x] Filter by category and status
- [x] Filesystem watcher — `notify` crate to detect external changes
- [x] Refresh mod list on filesystem change notification

---

### Phase 3 — Mod Toggle & State Control [DONE]

**Goal:** Enable/disable mods and manage presets.

- [x] Toggle mechanism — rename folder with `DISABLED_` prefix
- [x] Single mod toggle via UI click
- [x] Batch toggle — select multiple mods, apply enable/disable
- [x] Presets system — save named configurations (which mods enabled)
- [x] Load preset — apply saved state
- [x] Delete/rename presets
- [x] Hotkey binding for presets (global shortcuts via `tauri-plugin-global-shortcut`)
- [x] Handle case-sensitivity correctly on ext4/btrfs (case-insensitive `DISABLED_` prefix match)

---

### Phase 4 — Conflict Detection & Resolution [DONE]

**Goal:** Detect when multiple mods target the same game asset/slot.

**Scope revised during implementation:** character mods are near-universally full
model/texture replacements, so hash-checking or parsing mod `.ini` files to find the
literal overlapping asset target isn't worth the complexity — two enabled mods in the
same character category is already a reliable signal on its own. This turned Phase 4 into
a pure frontend feature (no new Rust code): a derived computation over the existing mod
list, recomputed automatically on every toggle/preset apply/filesystem refresh.

Detection is **passive only** — it warns, but never blocks toggling or requires explicit
resolution, since split mods (e.g. a hair-only mod and an outfit-only mod for the same
character) can legitimately coexist and a hard rule would punish valid setups. We also
deliberately don't track which specific mod combinations a user has "cleared", since that
would need its own persistence layer for a heuristic that's already just a best guess.

- [x] Conflict scanner — group enabled mods by category (character folder), flag any
      category with 2+ enabled mods. `UI`/`Other`/`Uncategorized` are exempt since those
      mods are typically additive (fonts, reshades, UI tweaks) and routinely coexist.
- [x] Conflict UI — passive warning banner listing conflicting categories + mods, plus a
      per-mod accent bar/icon on `ModCard` and a dot indicator on category filter chips
- [x] Resolution — one-click "Disable" per conflicting mod from the banner (manual pick);
      no auto-priority system since there's no reliable signal for which mod "should" win
- [x] Dismiss-for-session — banner can be dismissed but reappears if the conflicting
      category set changes, since dismissal isn't persisted (not tracking cleared combos)
- [ ] ~~Parse mod ini/config files to identify target hashes/asset slots~~ — dropped,
      not beneficial for this genre of mod (see rationale above)
- [ ] ~~Auto-disable lower-priority conflicting mods~~ — dropped in favor of always-manual
      resolution, since "priority" has no reliable basis to compute automatically

---

### Phase 5 — Restore Points [DONE]

**Goal:** Snapshot and restore mod states.

Two tiers, since presets already cover "switch between curated setups" — restore points
exist specifically as a safety net before risky actions (bulk toggling, and later mod
deletion/archive extraction):

- **State-only** (default): snapshot of which mods were enabled/disabled. Fast, no disk
  cost beyond a small JSON manifest.
- **Full backup** (opt-in checkbox): additionally copies every mod folder's files to
  `~/.local/share/mod-manager/restore_backups/<game>/<restore_point_id>/`, so a mod that's
  since been deleted can be *recreated* on restore, not just re-toggled.

Restoring is **additive/corrective, never destructive**: mods matching the snapshot are
toggled to match, missing mods are recreated from backup if one exists, but mods added
since the snapshot are left completely untouched — a "restore" that silently deleted newer
work would be a nasty surprise. This mirrors Phase 4's decision to keep destructive-feeling
actions manual/reviewable. Verified with a dedicated unit test
(`restore_never_touches_mods_added_after_the_snapshot`).

- [x] Create restore point — serialize current mod state (stable `key` + category + enabled)
- [x] Store restore points in config directory (`~/.config/mod-manager/restore_points/<game>.json`)
- [x] Restore from point — re-apply a saved state (toggle to match, recreate if missing+backed up)
- [x] List restore points with timestamps (relative time display, newest first)
- [x] Delete restore points (also cleans up the associated file backup, if any)
- [x] Optional: full file backup — recursive copy to XDG data dir, size tracked and shown in UI

7 new Rust unit tests (19 total across the backend): state-only create/list, restore
without backup (reports unrecoverable), restore with backup (recreates deleted folder from
copy, verifying file contents round-trip), the "never touches newer mods" guarantee, and
delete cleanup. UI: `RestorePointsPanel` modal (opened via a toolbar button in Local Mods)
with inline create form, per-point restore confirmation step, and a result summary after
restoring (counts + any unrecoverable mods called out).

---

### Phase 6 — Online Mode (GameBanana Integration) [DONE]

**Goal:** Browse, search, and view mods from GameBanana within the app.

GameBanana has no official public API docs, so the client was built against the
real (unofficial but widely-used, e.g. by Reloaded-III) `apiv11` endpoints, verified with
live requests before writing any Rust: `Game/:id/Subfeed` for browsing, `Util/Search/Results`
for full-text search, and `Mod/:id` for detail. Confirmed GameBanana's numeric game IDs for
all five supported titles (8552/18366/20357/19567/21842) and that only `default`/`new`/
`updated` are valid `_sSort` values — `popular`/`downloads` 400 despite being informally
documented elsewhere.

- [x] GameBanana API client (Rust, `reqwest`) — `gamebanana.rs`, typed response structs
      matching the real (verified) JSON shape, mapped down to a trimmed frontend projection
- [x] Browse view — paginated grid with thumbnails, sort (default/new/updated), like/view
      counts, featured badge
- [x] Search mods by query string — routed to GameBanana's real `Util/Search/Results`
      endpoint (not a client-side filter), debounced, replaces the sort/paginate flow while active
- [x] Category filtering (Characters, Skins, UI, Weapons, etc.) — added after initial
      Phase 6 delivery. Drill-down browser (`GbCategoryBrowser`) walks GameBanana's real
      per-game category tree one level at a time via `Mod/Categories`, since tree depth
      isn't uniform across games (Genshin/HSR/Endfield have an intermediate "Characters"
      grouping between the root "Skins" category and individual characters; Wuthering
      Waves/ZZZ go straight from "Skins" to character names). Selecting a leaf (or any
      level) filters the grid via `Mod/Index?_aFilters[Generic_Category]=<id>` — the
      Subfeed endpoint's `_idCategoryRow` param turned out to be a silent no-op when
      tested live, so filtered browsing had to move to a different endpoint than the
      unfiltered feed. Mutually exclusive with search, same as sort (GameBanana's search
      endpoint ignores a category filter regardless of whether one's supplied).
- [x] Mod detail view — description (rendered HTML), image gallery with prev/next, file
      list with size/version/AV scan status, link out to the GameBanana page
- [x] NSFW filter toggle — GameBanana's Subfeed has no direct NSFW boolean, so this uses a
      heuristic (`_bHasContentRatings` + `hide`/`warn` visibility) verified against real
      flagged mods; wired to the existing Settings NSFW toggle, hidden-count shown in UI
- [ ] ~~Paste-link handler~~ — deferred to a later polish pass; not blocking core browsing

Download buttons in the detail view currently log the target file/URL rather than actually
downloading — full download + auto-extract + install lands in Phase 7, which is the natural
next step and shares almost no surface with this phase's read-only browse/search/detail work.

10 new Rust unit tests (29 total across the backend): game ID mapping, sort value lock-in,
thumbnail fallback chain, the NSFW heuristic (including the "content-rated but shown" and
"no rating at all" negative cases), and two tests that deserialize trimmed real API response
fixtures end-to-end to catch schema drift early.

Category filtering follow-up added 2 more Rust unit tests (31 total): the `Mod/Index`
sort-alias mapping (distinct from Subfeed's own aliases) and a category-tree response
fixture deserialization test.

---

### Phase 7 — Download Manager & Auto-Install [DONE]

**Goal:** Download mods and install them automatically.

- [x] Download queue — add mods from online view (Download button on
      `GbModDetailPanel` now calls `queue_download` instead of only logging)
- [x] Sequential download execution with `reqwest` streaming — a single
      background worker loop (`downloads::worker_loop`, spawned once at app
      startup) drains the queue one item at a time; parallel downloads were
      considered and dropped since mod archives are large enough, and users'
      connections slow enough, that concurrent transfers would mostly just
      contend for the same pipe rather than actually go faster
- [x] Progress UI — real-time progress bar, speed, ETA per download.
      Backend emits raw byte counts only (`download-progress`, throttled to
      every 200ms); speed/ETA are computed client-side in `useDownloads` via
      lightly-smoothed deltas between ticks, since "rate" is inherently an
      over-time client observation, not something the backend needs to track
- [x] Download history — persisted to `~/.config/mod-manager/downloads.json`,
      same manifest pattern as presets/restore points. Items still `Queued`/
      `Downloading`/etc. at manifest load time (app was closed/crashed mid-
      transfer) are marked `Failed` with an "interrupted by app restart"
      message rather than silently resuming or hanging forever
- [x] Archive extraction — `sevenz-rust2` for .7z (the original `sevenz-rust`
      crate is unmaintained; this is the actively-maintained fork), `zip`
      crate for .zip, and `unrar` for .rar (a wrapper around RARLab's own
      UnRAR source — RAR's compression format is proprietary with no
      pure-Rust decoder, so this is the standard approach; its bundled
      license permits use in other software free of charge). Zip entries are
      extracted via `enclosed_name()`, and rar entries via a hand-written
      equivalent path-safety check (`safe_relative_path`, since `unrar` has
      no built-in guard) — both reject path-traversal/absolute-path entries
      (a "zip slip" archive) rather than following them, verified with unit
      tests for both formats. .rar support was added after initial Phase 7
      delivery in response to user feedback that it's a very common format
      for GameBanana mod downloads.
- [x] Smart placement — `find_content_root` descends through single-child
      "wrapper folder" archives (a common packaging pattern: the whole mod
      zipped inside one extra top-level folder) to find the real content
      root, then places it at `<mod_path>/<category>/<mod name>` using the
      GameBanana leaf category as the category folder (falls back to the mod
      root directly, which `scan_mods` already treats as "Uncategorized")
- [x] Extraction conflict resolution — if the computed destination folder
      already exists, the download pauses in `WaitingForConflict` and the
      Downloads view prompts Overwrite / Install Alongside (renamed with a
      " (2)", " (3)", ... suffix) / Cancel, mirroring Phase 4/5's principle
      of never taking a destructive action silently
- [x] Save preview image alongside mod folder — only if the extracted
      archive didn't already ship one of its own (checked via
      `mods::has_preview_image`, reusing the same `PREVIEW_NAMES` list the
      scanner uses), so a mod's own preview is never clobbered by
      GameBanana's thumbnail

Cross-filesystem note: moving extracted content from the XDG data-dir
staging area into the user's configured mod directory tries a plain rename
first, falling back to a recursive copy+delete (`move_dir`) since a Steam
Deck SD card (or any mod directory on a different filesystem/mount than
`~/.local/share`) would make a rename fail with `EXDEV`.

8 new Rust unit tests (39 total across the backend): archive-name sanitizing,
content-root wrapper-folder descent (including the "don't descend, multiple
entries at this level" and "no wrapper at all" cases), the same-filesystem
move path, unique-sibling-name generation, and the zip-slip rejection test
described above.

**Post-release fixes (user-reported):**

- **.rar support** — the initial delivery only handled .zip/.7z; added
  `unrar`-based extraction (see above) plus a real-archive test fixture
  (`test_fixtures/testfile.rar5.rar`, sourced from a public RAR test corpus
  and embedded so the test has no network dependency) covering both RAR3 and
  RAR5 container formats. 3 more Rust unit tests (42 total).
- **Downloads stalling when the Downloads page wasn't open** — root cause
  was macOS's App Nap, an OS-level feature (not something Tauri or our async
  runtime controls) that throttles a windowed app's background threads once
  its window stops being visible/frontmost, including the Tokio worker
  thread the whole download pipeline runs on. Confirmed via a standalone
  Tokio+reqwest smoke test that the same background-task pattern completes a
  92MB download in ~10s with nothing "watching" it when run as a plain CLI
  process (which is exempt from App Nap) — the windowed `.app` bundle is
  what triggers the throttling, not anything queue/worker-logic related.
  Fixed with a new `power_management` module that calls
  `NSProcessInfo.beginActivityWithOptions` once at startup (macOS-only,
  `cfg(target_os = "macos")`, no-op elsewhere) to opt the whole process out
  of App Nap for its entire lifetime — this also incidentally fixes the
  filesystem watcher and global hotkey listener being throttled in the
  background, not just downloads.

---

### Phase 8 — Update Tracking [DONE]

**Goal:** Detect when installed mods have newer versions online.

- [x] Store GameBanana mod ID + installed version in mod metadata —
      `update_tracking.rs` persists a per-game `origins/<game_id>.json`
      manifest mapping each mod's stable `key` to its GB mod_id +
      installed_file_id + version string. Written automatically by the
      download pipeline on successful install (`finish_install` calls
      `update_tracking::link_mod`), and can also be linked manually.
- [x] Update checker — `check_for_updates` sequentially queries the GB API
      (`get_mod_detail` per tracked mod, with 300ms rate-limiting delay
      between) and compares the installed file_id against the newest
      non-archived file on the same mod page. Returns only mods where the
      installed file is no longer current.
- [x] Update indicator in mod list — gold "Update" badge on `ModCard`
      (top-right corner, `bg-gold text-surface-0 shadow-glow-gold`, per
      design system's "gold = update available" rule). Gold-accented "Check
      Updates" button in LocalView toolbar shows the count when updates are
      found.
- [ ] Update detail view — show changelog/diff info (deferred — requires
      per-file description parsing that's not consistently populated by
      mod authors on GB; the badge + one-click update cover the core need)
- [x] One-click update — the frontend can pass `UpdateInfo` directly into
      `useDownloads.queue()` (same flow as a fresh download from the Online
      page), which re-downloads the latest file and triggers the existing
      conflict resolution flow (overwrite the old version / install
      alongside). No separate "update" endpoint needed.
- [ ] Bulk update option (deferred — the sequential check already surfaces
      all available updates at once; a "update all" button is a trivial
      follow-up that just loops `queue()` over the update list)

4 new Rust unit tests (46 total): link/get origin, link-overwrite (updating
the file_id for the same mod key replaces the entry), unlink/remove, and
empty-game fallback.

---

### Phase 9 — Polish & SteamOS Optimization [DONE]

**Goal:** Production-quality UX, optimized for Steam Deck and desktop.

- [ ] Gamepad navigation — focus rings, D-pad traversal, large touch targets
      (deferred — requires real hardware testing on Steam Deck)
- [ ] Steam Deck/Steam OS display optimization — 1280x800, proper scaling
      (deferred — the layout already works at 1280x800 since that's the app's
      configured minimum size, but touch-target sizing needs HID input testing)
- [x] Animations & transitions — page transitions (CSS `page-enter` keyframe:
      200ms fade+slide on view mount, triggered via `key={currentView}` remount
      in `MainContent`), toggle feedback utility class (`toggle-pop`)
- [ ] Customizable panel layout — resizable sidebar, collapsible panels
      (deferred — the sidebar already collapses via a button; drag-to-resize
      is a polish feature that can land in a later pass without blocking the
      core experience)
- [x] Mod deletion with confirmation dialog — `delete_mod` + `batch_delete_mods`
      in the backend (with path-safety check: refuses to delete anything
      outside the configured mod root via canonicalize + starts_with), exposed
      as `delete_game_mod` / `batch_delete_game_mods` commands. Frontend:
      Delete button in LocalView's batch action bar with a two-click
      confirmation (first click shows "Confirm Delete N", second actually
      deletes). 2 new Rust unit tests (48 total): deletion + outside-root
      rejection.
- [x] Import/Export full app config (JSON file) — `export_config` and
      `import_config` commands serialize/deserialize the `Settings` struct
      to/from a user-chosen path via `tauri-plugin-dialog`'s save/open
      dialogs. Settings section in the UI with Export/Import buttons.
- [x] Auto-reload mod list setting (toggle filesystem watcher) — already
      fully wired since Phase 2: the `auto_reload` toggle in Settings
      persists via `save_settings`, and `useMods` only registers the
      filesystem watcher when `settings.auto_reload` is true.
- [ ] Auto-launch game setting (spawn Proton/Wine process) — deferred to
      Phase 11 stretch goals; requires Proton/Wine detection logic that's
      only relevant on the target Linux platform, not testable on macOS.

---

### Phase 10 — Packaging & Distribution

**Goal:** Distributable packages for Arch Linux ecosystem.

- [ ] Arch `PKGBUILD` — installable via `makepkg`
- [ ] Flatpak manifest — sandboxed distribution
- [ ] AppImage — portable single-file distribution
- [ ] `.desktop` file with proper categories and icon
- [ ] XDG icon installation (multiple sizes)
- [ ] Self-update mechanism — check GitHub Releases, download + replace

---

### Phase 11 — Stretch Goals

- [ ] Proton/Wine mod loader integration (detect and configure XXMI under Proton)
- [ ] Multi-instance support (multiple mod folders per game)
- [ ] Plugin/extension system
- [ ] Cloud sync for presets/config across machines
- [ ] Mod collections — shareable curated lists
- [ ] Steam integration — launch via Steam as non-Steam game

---

## Platform Considerations (Arch Linux / SteamOS)

| Concern | Approach |
|---------|----------|
| Archive extraction | `sevenz-rust` crate (no external binary needed) + system `unzip` fallback |
| Mod loader compatibility | XXMI via Proton/Wine; document setup steps |
| Filesystem | Case-sensitive ext4/btrfs — handle folder naming carefully |
| SteamOS read-only root | Install to `~/.local/` or use Flatpak |
| Global hotkeys | `evdev` for Wayland, X11 bindings for Xorg |
| Gamepad input | Standard Linux gamepad evdev; Steam Input handles mapping |
| Config location | `~/.config/mod-manager/` (XDG_CONFIG_HOME) |
| Data location | `~/.local/share/mod-manager/` (XDG_DATA_HOME) |
| Permissions | No root required for normal operation |

---

## Progress Summary

| Phase | Status | Notes |
|-------|--------|-------|
| Phase 1 — Skeleton | Done | TSC, Vite, cargo check all pass |
| Phase 2 — Local Read | Done | Scan via `notify`-backed watcher, mod grid UI, search/filter, detail panel. 4 Rust unit tests pass |
| Phase 3 — Toggle & Presets | Done | Toggle/batch-toggle, presets (save/apply/rename/delete), global hotkeys via tauri-plugin-global-shortcut. 12 Rust unit tests pass |
| Phase 4 — Conflicts | Done | Passive category-heuristic (frontend-only, no Rust changes). tsc/vite/cargo all pass |
| Phase 5 — Restore Points | Done | State-only + optional full-file-backup snapshots, additive/non-destructive restore. 7 new Rust unit tests (19 total), tsc/vite/cargo all pass |
| Phase 6 — Online Mode | Done | GameBanana browse/search/detail via verified `apiv11` endpoints, NSFW heuristic filter, character/category drill-down browser (`Mod/Categories` + `Mod/Index`). Downloads land in Phase 7. 12 new Rust unit tests (31 total), tsc/vite/cargo all pass |
| Phase 7 — Downloads | Done | Sequential download queue + auto-extract (zip/7z/rar) + smart placement + conflict resolution UI. macOS App Nap opt-out so background downloads don't stall when the window isn't focused. 11 new Rust unit tests (42 total), tsc/vite/cargo all pass |
| Phase 8 — Update Tracking | Done | Per-game origin manifest, sequential GB API check with rate limiting, gold update badge on ModCard, auto-link on install. 4 new Rust unit tests (46 total), tsc/vite/cargo all pass |
| Phase 9 — Polish/SteamOS | Done | Mod deletion (batch, with path-safety + confirmation), page transitions (CSS keyframe), import/export config (JSON), auto-reload already wired. 2 new Rust unit tests (48 total), tsc/vite/cargo clean |
| Phase 10 — Packaging | Not Started | |
| Phase 11 — Stretch | Not Started | |
