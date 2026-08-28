import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "../lib/invoke";
import { useAppStore } from "../stores/appStore";
import {
  isConflictExemptCategory,
  type BatchDeleteResult,
  type BatchToggleResult,
  type CategoryConflict,
  type ModInfo,
  type ModStatusFilter,
  type ToggleTarget,
} from "../types";

const MODS_CHANGED_EVENT = "mods-changed";

interface UseModsResult {
  mods: ModInfo[];
  filteredMods: ModInfo[];
  loading: boolean;
  error: string | null;
  search: string;
  setSearch: (value: string) => void;
  statusFilter: ModStatusFilter;
  setStatusFilter: (value: ModStatusFilter) => void;
  categoryFilter: string | null;
  setCategoryFilter: (value: string | null) => void;
  categories: string[];
  refresh: () => void;
  toggleMod: (mod: ModInfo, enabled: boolean) => Promise<void>;
  batchToggle: (mods: ModInfo[], enabled: boolean) => Promise<BatchToggleResult>;
  deleteMods: (mods: ModInfo[]) => Promise<BatchDeleteResult>;
  toggleError: string | null;
  /** Categories with 2+ enabled mods — a likely (not certain) conflict. */
  conflicts: CategoryConflict[];
  /** ids of every mod currently involved in a conflict, for fast lookup in ModCard. */
  conflictingIds: Set<string>;
}

/**
 * Owns the full lifecycle of the local mod list for the currently active game:
 * fetching via the scan_game_mods command, re-fetching when the active game or
 * its configured mod path changes, subscribing to filesystem-watcher events
 * (respecting the auto_reload setting), and exposing search/category/status
 * filtering over the raw list.
 */
export function useMods(): UseModsResult {
  const { activeGame, settings } = useAppStore();
  const [mods, setMods] = useState<ModInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<ModStatusFilter>("all");
  const [categoryFilter, setCategoryFilter] = useState<string | null>(null);
  const [toggleError, setToggleError] = useState<string | null>(null);

  const modPath = useMemo(
    () => settings?.games.find((g) => g.id === activeGame)?.mod_path ?? null,
    [settings, activeGame]
  );
  const autoReload = settings?.auto_reload ?? true;

  const fetchMods = useCallback(async () => {
    if (!modPath) {
      setMods([]);
      setError(null);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<ModInfo[]>("scan_game_mods", {
        gameId: activeGame,
      });
      setMods(result);
    } catch (err) {
      setMods([]);
      setError(typeof err === "string" ? err : "Failed to scan mods");
    } finally {
      setLoading(false);
    }
  }, [activeGame, modPath]);

  // Re-fetch whenever the active game or its configured path changes, and
  // reset filters so stale search/category state from a different game
  // doesn't silently hide everything.
  useEffect(() => {
    setSearch("");
    setStatusFilter("all");
    setCategoryFilter(null);
    fetchMods();
  }, [fetchMods]);

  // Watch the mod directory for external changes (mods added/removed/toggled
  // outside the app) and auto-refresh when notified, if the user has
  // auto_reload enabled.
  const watchedRef = useRef<string | null>(null);
  useEffect(() => {
    if (!modPath || !autoReload) {
      return;
    }

    invoke("watch_mod_directory", { gameId: activeGame, path: modPath }).catch(
      (err) => console.error("Failed to watch mod directory:", err)
    );
    watchedRef.current = activeGame;

    const unlistenPromise = listen<string>(MODS_CHANGED_EVENT, (event) => {
      if (event.payload === activeGame) {
        fetchMods();
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
      if (watchedRef.current) {
        invoke("unwatch_mod_directory", { gameId: watchedRef.current }).catch(
          () => {}
        );
        watchedRef.current = null;
      }
    };
  }, [activeGame, modPath, autoReload, fetchMods]);

  // Toggle a single mod's enabled state. Optimistically replaces it in local
  // state with the backend's returned ModInfo (which has a new id/path since
  // toggling renames the folder) rather than waiting for a full rescan, so
  // the UI feels instant. Falls back to a full refresh if the toggle errors,
  // in case local state has drifted from disk.
  const toggleMod = useCallback(
    async (mod: ModInfo, enabled: boolean) => {
      setToggleError(null);
      try {
        const updated = await invoke<ModInfo>("toggle_mod", {
          path: mod.path,
          category: mod.category,
          enabled,
        });
        setMods((prev) => prev.map((m) => (m.id === mod.id ? updated : m)));
      } catch (err) {
        setToggleError(typeof err === "string" ? err : "Failed to toggle mod");
        await fetchMods();
      }
    },
    [fetchMods]
  );

  // Toggle multiple mods at once. Successfully toggled mods are merged into
  // local state by matching on the stable `key` (not `id`, since toggling
  // changes the path/id). Failures are surfaced via the returned result so
  // the caller can show which specific mods failed.
  const batchToggle = useCallback(
    async (targetMods: ModInfo[], enabled: boolean): Promise<BatchToggleResult> => {
      setToggleError(null);
      const targets: ToggleTarget[] = targetMods.map((m) => ({
        path: m.path,
        category: m.category,
      }));
      try {
        const result = await invoke<BatchToggleResult>("batch_toggle_mods", {
          targets,
          enabled,
        });
        setMods((prev) => {
          const byKey = new Map(result.updated.map((m) => [m.key, m]));
          return prev.map((m) => byKey.get(m.key) ?? m);
        });
        if (result.errors.length > 0) {
          setToggleError(result.errors.join("; "));
        }
        return result;
      } catch (err) {
        const message = typeof err === "string" ? err : "Failed to toggle mods";
        setToggleError(message);
        await fetchMods();
        return { updated: [], errors: [message] };
      }
    },
    [fetchMods]
  );

  // Delete one or more mods permanently. Triggers a full rescan after
  // completion since mod paths are invalidated by deletion. Returns the
  // result so the caller can show what succeeded/failed.
  const deleteMods = useCallback(
    async (targetMods: ModInfo[]): Promise<BatchDeleteResult> => {
      setToggleError(null);
      const paths = targetMods.map((m) => m.path);
      try {
        const result = await invoke<BatchDeleteResult>("batch_delete_game_mods", {
          gameId: activeGame,
          paths,
        });
        if (result.errors.length > 0) {
          setToggleError(result.errors.join("; "));
        }
        await fetchMods();
        return result;
      } catch (err) {
        const message = typeof err === "string" ? err : "Failed to delete mods";
        setToggleError(message);
        await fetchMods();
        return { deleted_keys: [], errors: [message] };
      }
    },
    [activeGame, fetchMods]
  );

  const categories = useMemo(() => {
    const set = new Set(mods.map((m) => m.category));
    return Array.from(set).sort((a, b) => a.localeCompare(b));
  }, [mods]);

  // Passive conflict heuristic: character mods are near-universally full
  // replacements of the same model/texture slot, so 2+ enabled mods in the
  // same (non-exempt) category is treated as a likely conflict. This is a
  // warning, not a rule — split mods (e.g. separate hair/outfit mods for the
  // same character) can legitimately coexist, so nothing here blocks toggling
  // or requires the user to "resolve" anything.
  const conflicts = useMemo<CategoryConflict[]>(() => {
    const byCategory = new Map<string, ModInfo[]>();
    for (const mod of mods) {
      if (!mod.enabled || isConflictExemptCategory(mod.category)) continue;
      const list = byCategory.get(mod.category) ?? [];
      list.push(mod);
      byCategory.set(mod.category, list);
    }
    return Array.from(byCategory.entries())
      .filter(([, list]) => list.length > 1)
      .map(([category, list]) => ({ category, mods: list }))
      .sort((a, b) => a.category.localeCompare(b.category));
  }, [mods]);

  const conflictingIds = useMemo(() => {
    const ids = new Set<string>();
    for (const conflict of conflicts) {
      for (const mod of conflict.mods) ids.add(mod.id);
    }
    return ids;
  }, [conflicts]);

  const filteredMods = useMemo(() => {
    const term = search.trim().toLowerCase();
    return mods.filter((mod) => {
      if (term && !mod.name.toLowerCase().includes(term) && !mod.category.toLowerCase().includes(term)) {
        return false;
      }
      if (statusFilter === "enabled" && !mod.enabled) return false;
      if (statusFilter === "disabled" && mod.enabled) return false;
      if (categoryFilter && mod.category !== categoryFilter) return false;
      return true;
    });
  }, [mods, search, statusFilter, categoryFilter]);

  return {
    mods,
    filteredMods,
    loading,
    error,
    search,
    setSearch,
    statusFilter,
    setStatusFilter,
    categoryFilter,
    setCategoryFilter,
    categories,
    refresh: fetchMods,
    toggleMod,
    batchToggle,
    deleteMods,
    toggleError,
    conflicts,
    conflictingIds,
  };
}
