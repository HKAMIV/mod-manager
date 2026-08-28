import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "../lib/invoke";
import { useAppStore } from "../stores/appStore";
import type { UpdateInfo } from "../types";

interface UseUpdateCheckResult {
  /** Map of mod_key → UpdateInfo for mods with available updates. */
  updates: Map<string, UpdateInfo>;
  /** Whether a check is currently in progress (may take a while for many mods). */
  checking: boolean;
  error: string | null;
  /** Number of mods that have updates available. */
  updateCount: number;
  /** Trigger a fresh check against GameBanana. Called automatically when the
   * active game changes, and can be invoked manually via the toolbar button. */
  checkNow: () => void;
  /** The last time a successful check completed (unix ms), or null if never. */
  lastCheckedAt: number | null;
}

/**
 * Manages update checking for the active game's installed mods. Queries
 * GameBanana in the background (via the Rust `check_for_updates` command,
 * which is sequential and rate-limited) for each mod that has a tracked
 * origin, comparing the installed file_id against the newest non-archived
 * upload on the same GB mod page.
 */
export function useUpdateCheck(): UseUpdateCheckResult {
  const { activeGame } = useAppStore();
  const [updatesRaw, setUpdatesRaw] = useState<UpdateInfo[]>([]);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastCheckedAt, setLastCheckedAt] = useState<number | null>(null);

  const checkNow = useCallback(async () => {
    setChecking(true);
    setError(null);
    try {
      const result = await invoke<UpdateInfo[]>("check_for_updates", {
        gameId: activeGame,
      });
      setUpdatesRaw(result);
      setLastCheckedAt(Date.now());
    } catch (err) {
      setError(typeof err === "string" ? err : "Failed to check for updates");
    } finally {
      setChecking(false);
    }
  }, [activeGame]);

  // Auto-check when the active game changes (but not on every render — only
  // on actual game switch). Debounced slightly so rapidly cycling through
  // games in the selector doesn't fire a barrage of network requests.
  useEffect(() => {
    setUpdatesRaw([]);
    setLastCheckedAt(null);
    const timeout = setTimeout(checkNow, 500);
    return () => clearTimeout(timeout);
  }, [activeGame]); // eslint-disable-line react-hooks/exhaustive-deps

  const updates = useMemo(
    () => new Map(updatesRaw.map((u) => [u.mod_key, u])),
    [updatesRaw]
  );

  return {
    updates,
    checking,
    error,
    updateCount: updatesRaw.length,
    checkNow,
    lastCheckedAt,
  };
}
