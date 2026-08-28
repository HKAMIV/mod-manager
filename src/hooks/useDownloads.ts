import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "../lib/invoke";
import type {
  ConflictResolution,
  DownloadItem,
  DownloadProgress,
  GameId,
  GbModFile,
} from "../types";

const DOWNLOAD_UPDATED_EVENT = "download-updated";
const DOWNLOAD_PROGRESS_EVENT = "download-progress";

/** Params needed to queue a GameBanana file for download + auto-install. */
export interface QueueDownloadArgs {
  gameId: GameId;
  modId: number;
  modName: string;
  file: GbModFile;
  /** Leaf category name (e.g. "Furina"), used to place the mod under a
   * matching subfolder rather than directly in the mod root. */
  categoryHint: string | null;
  previewImageUrl: string | null;
}

/** Per-download live throughput, computed client-side from successive
 * progress ticks — the backend only reports byte counts, not rate, since
 * rate is inherently a client-side-observed-over-time concept. */
interface Rate {
  bytesPerSecond: number;
  lastBytes: number;
  lastTimestamp: number;
}

interface UseDownloadsResult {
  downloads: DownloadItem[];
  loading: boolean;
  error: string | null;
  queue: (args: QueueDownloadArgs) => Promise<{ item: DownloadItem | null; error: string | null }>;
  cancel: (id: string) => Promise<void>;
  retry: (id: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
  resolveConflict: (id: string, action: ConflictResolution) => Promise<void>;
  /** Bytes/sec for a given download, or null if not enough data yet (or the
   * download isn't actively transferring). */
  rateFor: (id: string) => number | null;
  refresh: () => void;
}

/**
 * Owns the full lifecycle of the download queue/history: initial fetch,
 * live updates via the `download-updated`/`download-progress` events the
 * Rust worker emits (see `src-tauri/src/downloads.rs`), and the queue/
 * cancel/retry/remove/resolve-conflict actions. Downloads aren't scoped to
 * the active game the way mods/presets/restore points are — the manifest is
 * shared across the whole app, same as GameBanana browsing itself.
 */
export function useDownloads(): UseDownloadsResult {
  const [downloads, setDownloads] = useState<DownloadItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const ratesRef = useRef<Map<string, Rate>>(new Map());

  const fetchDownloads = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<DownloadItem[]>("list_downloads");
      setDownloads(result);
    } catch (err) {
      setError(typeof err === "string" ? err : "Failed to load downloads");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchDownloads();
  }, [fetchDownloads]);

  // Subscribe once for the component's lifetime — downloads span across
  // whatever page the user is on, so this isn't gated behind the Downloads
  // view being mounted in any special way, just this hook being used.
  useEffect(() => {
    const unlistenUpdated = listen<DownloadItem>(DOWNLOAD_UPDATED_EVENT, (event) => {
      const updated = event.payload;
      setDownloads((prev) => {
        const exists = prev.some((d) => d.id === updated.id);
        return exists
          ? prev.map((d) => (d.id === updated.id ? updated : d))
          : [updated, ...prev];
      });
      if (updated.status.state !== "downloading") {
        ratesRef.current.delete(updated.id);
      }
    });

    const unlistenProgress = listen<DownloadProgress>(DOWNLOAD_PROGRESS_EVENT, (event) => {
      const { id, downloaded_bytes, total_bytes } = event.payload;
      const now = performance.now();
      const prevRate = ratesRef.current.get(id);
      if (prevRate) {
        const elapsedSec = (now - prevRate.lastTimestamp) / 1000;
        const deltaBytes = downloaded_bytes - prevRate.lastBytes;
        if (elapsedSec > 0 && deltaBytes >= 0) {
          // Light exponential smoothing so the displayed rate doesn't jitter
          // wildly between individual 200ms progress ticks.
          const instantRate = deltaBytes / elapsedSec;
          const smoothed =
            prevRate.bytesPerSecond === 0
              ? instantRate
              : prevRate.bytesPerSecond * 0.7 + instantRate * 0.3;
          ratesRef.current.set(id, {
            bytesPerSecond: smoothed,
            lastBytes: downloaded_bytes,
            lastTimestamp: now,
          });
        }
      } else {
        ratesRef.current.set(id, {
          bytesPerSecond: 0,
          lastBytes: downloaded_bytes,
          lastTimestamp: now,
        });
      }

      setDownloads((prev) =>
        prev.map((d) =>
          d.id === id ? { ...d, downloaded_bytes, total_bytes } : d
        )
      );
    });

    return () => {
      unlistenUpdated.then((f) => f());
      unlistenProgress.then((f) => f());
    };
  }, []);

  const queue = useCallback(
    async (
      args: QueueDownloadArgs
    ): Promise<{ item: DownloadItem | null; error: string | null }> => {
      setError(null);
      try {
        const item = await invoke<DownloadItem>("queue_download", {
          gameId: args.gameId,
          modId: args.modId,
          modName: args.modName,
          fileId: args.file.id,
          fileName: args.file.file_name,
          downloadUrl: args.file.download_url,
          totalBytes: args.file.filesize_bytes,
          categoryHint: args.categoryHint,
          previewImageUrl: args.previewImageUrl,
        });
        setDownloads((prev) => [item, ...prev]);
        return { item, error: null };
      } catch (err) {
        const message = typeof err === "string" ? err : "Failed to queue download";
        setError(message);
        return { item: null, error: message };
      }
    },
    []
  );

  const cancel = useCallback(async (id: string) => {
    setError(null);
    try {
      await invoke("cancel_download", { id });
    } catch (err) {
      setError(typeof err === "string" ? err : "Failed to cancel download");
    }
  }, []);

  const retry = useCallback(async (id: string) => {
    setError(null);
    try {
      await invoke("retry_download", { id });
    } catch (err) {
      setError(typeof err === "string" ? err : "Failed to retry download");
    }
  }, []);

  const remove = useCallback(async (id: string) => {
    setError(null);
    try {
      await invoke("remove_download", { id });
      setDownloads((prev) => prev.filter((d) => d.id !== id));
    } catch (err) {
      setError(typeof err === "string" ? err : "Failed to remove download");
    }
  }, []);

  const resolveConflict = useCallback(async (id: string, action: ConflictResolution) => {
    setError(null);
    try {
      await invoke("resolve_download_conflict", { id, action });
    } catch (err) {
      setError(typeof err === "string" ? err : "Failed to resolve conflict");
    }
  }, []);

  const rateFor = useCallback((id: string) => {
    const rate = ratesRef.current.get(id);
    return rate && rate.bytesPerSecond > 0 ? rate.bytesPerSecond : null;
  }, []);

  const sorted = useMemo(
    () => [...downloads].sort((a, b) => b.created_at - a.created_at),
    [downloads]
  );

  return {
    downloads: sorted,
    loading,
    error,
    queue,
    cancel,
    retry,
    remove,
    resolveConflict,
    rateFor,
    refresh: fetchDownloads,
  };
}
