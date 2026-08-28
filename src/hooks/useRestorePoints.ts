import { useCallback, useEffect, useState } from "react";
import { invoke } from "../lib/invoke";
import { useAppStore } from "../stores/appStore";
import type { RestorePoint, RestoreResult } from "../types";

interface UseRestorePointsResult {
  restorePoints: RestorePoint[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  createRestorePoint: (name: string, withFileBackup: boolean) => Promise<RestorePoint | null>;
  deleteRestorePoint: (restorePointId: string) => Promise<void>;
  restoreFromPoint: (restorePointId: string) => Promise<RestoreResult | null>;
}

/**
 * Owns CRUD + restore for the currently active game's restore points. A
 * restore point snapshots which mods were enabled/disabled at a moment in
 * time, and can optionally include a full file backup so a mod that gets
 * deleted later can be recreated (not just re-toggled) on restore.
 */
export function useRestorePoints(onRestored?: () => void): UseRestorePointsResult {
  const { activeGame } = useAppStore();
  const [restorePoints, setRestorePoints] = useState<RestorePoint[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<RestorePoint[]>("list_restore_points", {
        gameId: activeGame,
      });
      setRestorePoints(result);
    } catch (err) {
      setError(typeof err === "string" ? err : "Failed to load restore points");
    } finally {
      setLoading(false);
    }
  }, [activeGame]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const createRestorePoint = useCallback(
    async (name: string, withFileBackup: boolean) => {
      try {
        const point = await invoke<RestorePoint>("create_restore_point", {
          gameId: activeGame,
          name,
          withFileBackup,
        });
        await refresh();
        return point;
      } catch (err) {
        setError(typeof err === "string" ? err : "Failed to create restore point");
        return null;
      }
    },
    [activeGame, refresh]
  );

  const deleteRestorePoint = useCallback(
    async (restorePointId: string) => {
      try {
        await invoke("delete_restore_point", { gameId: activeGame, restorePointId });
        await refresh();
      } catch (err) {
        setError(typeof err === "string" ? err : "Failed to delete restore point");
      }
    },
    [activeGame, refresh]
  );

  const restoreFromPoint = useCallback(
    async (restorePointId: string) => {
      try {
        const result = await invoke<RestoreResult>("restore_from_point", {
          gameId: activeGame,
          restorePointId,
        });
        onRestored?.();
        return result;
      } catch (err) {
        setError(typeof err === "string" ? err : "Failed to restore");
        return null;
      }
    },
    [activeGame, onRestored]
  );

  return {
    restorePoints,
    loading,
    error,
    refresh,
    createRestorePoint,
    deleteRestorePoint,
    restoreFromPoint,
  };
}
