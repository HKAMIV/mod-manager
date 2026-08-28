import { useCallback, useEffect, useState } from "react";
import { invoke } from "../lib/invoke";
import { useAppStore } from "../stores/appStore";
import type { ApplyPresetResult, Preset } from "../types";

interface UsePresetsResult {
  presets: Preset[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  savePreset: (name: string) => Promise<Preset | null>;
  deletePreset: (presetId: string) => Promise<void>;
  renamePreset: (presetId: string, newName: string) => Promise<void>;
  applyPreset: (presetId: string) => Promise<ApplyPresetResult | null>;
  setHotkey: (presetId: string, hotkey: string) => Promise<void>;
  clearHotkey: (presetId: string) => Promise<void>;
}

/**
 * Owns CRUD + apply for the currently active game's mod presets. Presets are
 * a named snapshot of "which mods are enabled" that can be saved from the
 * current state and re-applied later (or triggered via a global hotkey, see
 * `setHotkey`).
 */
export function usePresets(onApplied?: () => void): UsePresetsResult {
  const { activeGame } = useAppStore();
  const [presets, setPresets] = useState<Preset[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<Preset[]>("list_presets", { gameId: activeGame });
      setPresets(result);
    } catch (err) {
      setError(typeof err === "string" ? err : "Failed to load presets");
    } finally {
      setLoading(false);
    }
  }, [activeGame]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const savePreset = useCallback(
    async (name: string) => {
      try {
        const preset = await invoke<Preset>("save_preset", { gameId: activeGame, name });
        await refresh();
        return preset;
      } catch (err) {
        setError(typeof err === "string" ? err : "Failed to save preset");
        return null;
      }
    },
    [activeGame, refresh]
  );

  const deletePreset = useCallback(
    async (presetId: string) => {
      try {
        await invoke("delete_preset", { gameId: activeGame, presetId });
        await refresh();
      } catch (err) {
        setError(typeof err === "string" ? err : "Failed to delete preset");
      }
    },
    [activeGame, refresh]
  );

  const renamePreset = useCallback(
    async (presetId: string, newName: string) => {
      try {
        await invoke("rename_preset", { gameId: activeGame, presetId, newName });
        await refresh();
      } catch (err) {
        setError(typeof err === "string" ? err : "Failed to rename preset");
      }
    },
    [activeGame, refresh]
  );

  const applyPreset = useCallback(
    async (presetId: string) => {
      try {
        const result = await invoke<ApplyPresetResult>("apply_preset", {
          gameId: activeGame,
          presetId,
        });
        onApplied?.();
        return result;
      } catch (err) {
        setError(typeof err === "string" ? err : "Failed to apply preset");
        return null;
      }
    },
    [activeGame, onApplied]
  );

  const setHotkey = useCallback(
    async (presetId: string, hotkey: string) => {
      try {
        await invoke("set_preset_hotkey", { gameId: activeGame, presetId, hotkey });
        await refresh();
      } catch (err) {
        setError(typeof err === "string" ? err : "Failed to set hotkey");
      }
    },
    [activeGame, refresh]
  );

  const clearHotkey = useCallback(
    async (presetId: string) => {
      try {
        await invoke("clear_preset_hotkey", { gameId: activeGame, presetId });
        await refresh();
      } catch (err) {
        setError(typeof err === "string" ? err : "Failed to clear hotkey");
      }
    },
    [activeGame, refresh]
  );

  return {
    presets,
    loading,
    error,
    refresh,
    savePreset,
    deletePreset,
    renamePreset,
    applyPreset,
    setHotkey,
    clearHotkey,
  };
}
