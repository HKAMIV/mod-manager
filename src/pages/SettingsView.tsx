import { useEffect, useState } from "react";
import { useAppStore } from "../stores/appStore";
import { invoke } from "../lib/invoke";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Keyboard, X } from "lucide-react";
import { formatAccelerator } from "../lib/hotkeyRecorder";
import type { GameConfig, Preset, Settings } from "../types";

interface GameHotkeys {
  game: GameConfig;
  presets: Preset[];
}

function SettingsView() {
  const { settings, setSettings, activeGame } = useAppStore();
  const [gameHotkeys, setGameHotkeys] = useState<GameHotkeys[]>([]);

  useEffect(() => {
    if (!settings) return;
    loadAllHotkeys(settings.games);
  }, [settings]);

  const loadAllHotkeys = async (games: GameConfig[]) => {
    const results: GameHotkeys[] = [];
    for (const game of games) {
      try {
        const presets = await invoke<Preset[]>("list_presets", { gameId: game.id });
        const withHotkeys = presets.filter((p) => p.hotkey);
        if (withHotkeys.length > 0) {
          results.push({ game, presets: withHotkeys });
        }
      } catch {
        // No presets file yet for this game — skip silently.
      }
    }
    setGameHotkeys(results);
  };

  const handleClearHotkey = async (gameId: string, presetId: string) => {
    try {
      await invoke("clear_preset_hotkey", { gameId, presetId });
      if (settings) await loadAllHotkeys(settings.games);
    } catch (err) {
      console.error("Failed to clear hotkey:", err);
    }
  };

  const refreshSettings = async () => {
    try {
      const s = await invoke<Settings>("get_settings");
      setSettings(s);
    } catch (err) {
      console.error("Failed to load settings:", err);
    }
  };

  const handleSetPath = async (gameId: string) => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select mod folder",
    });
    if (!selected) return;

    try {
      await invoke("set_game_path", { gameId, path: selected });
      await refreshSettings();
    } catch (err) {
      console.error("Failed to set path:", err);
    }
  };

  const handleToggleSetting = async (key: "nsfw_filter" | "auto_reload") => {
    if (!settings) return;
    const updated: Settings = { ...settings, [key]: !settings[key] };
    setSettings(updated);
    try {
      await invoke("save_settings", { settings: updated });
    } catch (err) {
      console.error("Failed to save settings:", err);
      setSettings(settings);
    }
  };

  const handleExportConfig = async () => {
    const path = await save({
      title: "Export Mod Manager Config",
      defaultPath: "mod-manager-config.json",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;
    try {
      await invoke("export_config", { path });
    } catch (err) {
      console.error("Export failed:", err);
    }
  };

  const handleImportConfig = async () => {
    const selected = await open({
      title: "Import Mod Manager Config",
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!selected) return;
    try {
      const imported = await invoke<Settings>("import_config", { path: selected });
      setSettings(imported);
    } catch (err) {
      console.error("Import failed:", err);
    }
  };

  if (!settings) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <p className="text-text-muted">Loading settings...</p>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto p-6 space-y-8">
      <div>
        <h1 className="font-display text-2xl font-semibold text-text-primary tracking-wide">
          Settings
        </h1>
        <p className="text-xs text-text-muted mt-0.5">
          Configure mod folders and app behavior
        </p>
      </div>

      {/* Mod Folder Paths */}
      <section className="space-y-3">
        <div className="space-y-1.5">
          <h2 className="hud-label text-xs text-game2">Mod Folders</h2>
          <div className="hud-rule" />
          <p className="text-xs text-text-muted">
            Point each game at the folder your mod loader (e.g. XXMI) reads mods from —
            not the game's install directory.
          </p>
        </div>
        <div className="space-y-2">
          {settings?.games.map((game) => {
            const isActive = game.id === activeGame;
            return (
              <div
                key={game.id}
                className={`game-panel flex items-center justify-between p-3.5 border transition-colors ${
                  isActive
                    ? "bg-game/5 border-game/30 shadow-glow-game"
                    : "bg-surface-1 border-surface-3"
                }`}
              >
                <div className="flex items-center gap-2.5">
                  <span
                    className={`w-1.5 h-1.5 rounded-full shrink-0 ${
                      isActive ? "bg-game shadow-glow-game" : "bg-surface-3"
                    }`}
                  />
                  <div>
                    <p className="text-sm text-text-primary font-medium">
                      {game.name}
                    </p>
                    <p className="text-xs text-text-muted mt-0.5 font-mono">
                      {game.mod_path ?? "Not configured"}
                    </p>
                  </div>
                </div>
                <button
                  onClick={() => handleSetPath(game.id)}
                  className="game-control px-3 py-1.5 text-xs font-medium bg-surface-2 border border-surface-3 hover:border-game/40 hover:text-game text-text-secondary transition-colors"
                >
                  Set Path
                </button>
              </div>
            );
          })}
        </div>
      </section>

      {/* General Settings */}
      <section className="space-y-3">
        <div className="space-y-1.5">
          <h2 className="hud-label text-xs text-game2">General</h2>
          <div className="hud-rule" />
        </div>
        <div className="space-y-2">
          <label className="game-panel flex items-center justify-between p-3.5 bg-surface-1 border border-surface-3 cursor-pointer hover:border-game/30 transition-colors">
            <span className="text-sm text-text-primary">NSFW Filter</span>
            <input
              type="checkbox"
              checked={settings?.nsfw_filter ?? true}
              onChange={() => handleToggleSetting("nsfw_filter")}
              className="w-4 h-4"
              style={{ accentColor: "var(--game-accent)" }}
            />
          </label>
          <label className="game-panel flex items-center justify-between p-3.5 bg-surface-1 border border-surface-3 cursor-pointer hover:border-game/30 transition-colors">
            <span className="text-sm text-text-primary">
              Auto-reload on file changes
            </span>
            <input
              type="checkbox"
              checked={settings?.auto_reload ?? true}
              onChange={() => handleToggleSetting("auto_reload")}
              className="w-4 h-4"
              style={{ accentColor: "var(--game-accent)" }}
            />
          </label>
        </div>
      </section>

      {/* Import/Export Configuration */}
      <section className="space-y-3">
        <div className="space-y-1.5">
          <h2 className="hud-label text-xs text-game2">Configuration</h2>
          <div className="hud-rule" />
          <p className="text-xs text-text-muted">
            Export your full app config (game paths, preferences, hotkeys) to a JSON file,
            or import one to restore settings on a new machine.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleExportConfig}
            className="game-control px-3 py-2 text-xs font-medium bg-surface-2 border border-surface-3 hover:border-game/40 hover:text-game text-text-secondary transition-colors"
          >
            Export Config
          </button>
          <button
            onClick={handleImportConfig}
            className="game-control px-3 py-2 text-xs font-medium bg-surface-2 border border-surface-3 hover:border-game/40 hover:text-game text-text-secondary transition-colors"
          >
            Import Config
          </button>
        </div>
      </section>

      {/* Preset Hotkeys */}
      {gameHotkeys.length > 0 && (
        <section className="space-y-3">
          <div className="space-y-1.5">
            <h2 className="hud-label text-xs text-game2">Preset Hotkeys</h2>
            <div className="hud-rule" />
            <p className="text-xs text-text-muted">
              Global shortcuts trigger their preset even while the app is unfocused. Manage
              them from the presets bar in Local Mods, or clear one here.
            </p>
          </div>
          <div className="space-y-2">
            {gameHotkeys.map(({ game, presets }) => (
              <div key={game.id} className="space-y-1.5">
                <p className="text-xs text-text-muted font-medium">{game.name}</p>
                {presets.map((preset) => (
                  <div
                    key={preset.id}
                    className="game-panel flex items-center justify-between p-3 bg-surface-1 border border-surface-3"
                  >
                    <div className="flex items-center gap-2">
                      <Keyboard size={13} className="text-game2" />
                      <span className="text-sm text-text-primary">{preset.name}</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="hud-label text-[10px] text-game2 bg-game2/10 px-1.5 py-0.5 rounded">
                        {formatAccelerator(preset.hotkey!)}
                      </span>
                      <button
                        onClick={() => handleClearHotkey(game.id, preset.id)}
                        className="p-1 text-text-muted hover:text-game2 transition-colors"
                        aria-label={`Clear hotkey for ${preset.name}`}
                      >
                        <X size={13} />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

export default SettingsView;
