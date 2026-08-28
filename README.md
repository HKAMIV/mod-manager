# Mod Manager

A desktop application for managing game mods — browse, download, install, and organize mods for **Wuthering Waves**, **Genshin Impact**, **Zenless Zone Zero**, **Honkai: Star Rail**, and **Arknights: Endfield**.

Built with [Tauri v2](https://v2.tauri.app/) (Rust backend + React/TypeScript frontend), targeting Arch Linux and SteamOS (Steam Deck), with macOS support for development.

![Rust](https://img.shields.io/badge/Rust-000?logo=rust&logoColor=fff)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?logo=typescript&logoColor=fff)
![Tauri](https://img.shields.io/badge/Tauri_v2-24C8D8?logo=tauri&logoColor=fff)
![React](https://img.shields.io/badge/React-61DAFB?logo=react&logoColor=000)

---

## Features

### Local Mod Management
- Auto-detect installed mods from your game's mod folder
- One-click enable/disable (renames with `DISABLED_` prefix, XXMI/GIMI compatible)
- Batch operations via named presets with global hotkey bindings
- Preview images, search, filter by category/status
- Passive conflict detection (warns when 2+ mods target the same character)
- Restore points — snapshot and roll back mod state (with optional full file backup)
- Mod deletion with safety checks

### Online Browsing (GameBanana)
- Browse, search, and filter the GameBanana mod library per-game
- Drill-down category browser (Skins → Characters → specific character)
- Full mod detail view with description, image gallery, and file list
- NSFW content filter

### Download Manager
- One-click download from GameBanana with real-time progress, speed, and ETA
- Automatic extraction (`.zip`, `.7z`, `.rar`) with zip-slip protection
- Smart placement into the correct category subfolder
- Conflict resolution: overwrite / install alongside / cancel
- Sequential queue, persistent history

### Update Tracking
- Automatically links downloaded mods to their GameBanana source
- Checks for newer versions via the GB API
- Gold "Update" badge on mods with available updates
- One-click update (re-downloads through the existing pipeline)

### Per-Game Theming
Each supported game shifts the app's visual identity — accent colors, display font, corner shapes, and background texture all adapt to match the game you're currently modding.

---

## Tech Stack

| Layer | Choice |
|-------|--------|
| Framework | Tauri v2 |
| Frontend | React 18 + TypeScript + Vite |
| Styling | Tailwind CSS |
| State | Zustand |
| Backend | Rust (reqwest, notify, serde, zip, sevenz-rust2, unrar) |
| Packaging | AppImage, .deb, .dmg (CI), PKGBUILD (Arch) |

---

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable, 1.75+)
- [Node.js](https://nodejs.org/) 20+
- System dependencies (Linux only):
  ```bash
  # Arch
  sudo pacman -S webkit2gtk-4.1 gtk3 openssl libappindicator-gtk3

  # Ubuntu/Debian
  sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libssl-dev libgtk-3-dev
  ```

### Development

```bash
# Install frontend dependencies
npm ci

# Run in development mode (hot-reload frontend + Rust backend)
npm run tauri dev
```

### Production Build

```bash
# Build optimized bundles (AppImage + .deb on Linux, .dmg on macOS)
npm run tauri build
```

---

## Project Structure

```
src/                    # React frontend
  components/           # UI components (ModCard, Sidebar, GbCategoryBrowser, etc.)
  hooks/                # Custom hooks (useMods, useDownloads, useUpdateCheck, etc.)
  pages/                # View pages (LocalView, OnlineView, DownloadsView, Settings)
  stores/               # Zustand state (appStore)
  styles/               # Tailwind + design system CSS
  types/                # TypeScript interfaces

src-tauri/              # Rust backend
  src/
    commands/           # Tauri command handlers (mods, downloads, settings, etc.)
    mods.rs             # Mod scanning, toggle, delete
    downloads.rs        # Download queue, extraction, placement
    gamebanana.rs       # GameBanana API client
    update_tracking.rs  # Version tracking per installed mod
    watcher.rs          # Filesystem change detection
    presets.rs          # Named preset configurations
    restore_points.rs   # State snapshot & restore
    power_management.rs # macOS App Nap opt-out
    state.rs            # App settings & persistence

packaging/              # Distribution files
  PKGBUILD              # Arch Linux package recipe
  mod-manager.desktop   # XDG desktop entry

.github/workflows/      # CI/CD
  ci.yml                # Test on every push (Linux + macOS)
  release.yml           # Build + publish on tag push
```

---

## Configuration

All config lives under XDG-standard paths:

| What | Path |
|------|------|
| Settings, presets, restore points, download history, update origins | `~/.config/mod-manager/` |
| Restore point file backups, download staging | `~/.local/share/mod-manager/` |

Settings can be exported/imported as JSON from the Settings page.

---

## Releases

Tagged releases (`v*`) automatically build and publish:
- **Linux**: AppImage + `.deb`
- **macOS**: `.dmg` (unsigned — run `xattr -cr` after install)

Download from the [Releases page](https://github.com/HKAMIV/mod-manager/releases).

### Arch Linux (PKGBUILD)

```bash
cd packaging/
makepkg -si
```

---

## SteamOS / Steam Deck Notes

- The AppImage runs on SteamOS out of the box (no root required)
- Mod directories on an SD card work — the app handles cross-filesystem moves automatically
- Global hotkeys for presets work via `tauri-plugin-global-shortcut`
- The app opts out of App Nap (macOS) and disables DMABUF rendering in AppImage environments to prevent background stalling and white-screen issues

---

## Supported Games

| Game | GameBanana ID | Status |
|------|---------------|--------|
| Wuthering Waves | 20357 | Full support |
| Genshin Impact | 8552 | Full support |
| Zenless Zone Zero | 19567 | Full support |
| Honkai: Star Rail | 18366 | Full support |
| Arknights: Endfield | 21842 | Full support |

---

## License

MIT
