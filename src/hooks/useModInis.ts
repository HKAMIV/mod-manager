import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "../lib/invoke";
import type { HashEdit, HashUpdateResult, ModIniFile } from "../types";

interface UseModInisResult {
  /** Parsed ini files for the current mod (empty until loaded). */
  files: ModIniFile[];
  loading: boolean;
  error: string | null;
  /** Re-read the mod's inis from disk (bypasses cache). */
  refresh: () => void;
  /** Apply staged hash edits; on success the affected files are re-read. */
  saveHashEdits: (edits: HashEdit[]) => Promise<HashUpdateResult>;
}

/**
 * Loads a mod's `.ini` files (keybinds + hashes) for the detail panel.
 *
 * Fetches asynchronously whenever `modPath` changes so opening the panel never
 * blocks on disk I/O, and caches results per mod path so switching back to a
 * previously-viewed mod is instant. Pass `null` (no mod selected) to clear.
 */
export function useModInis(modPath: string | null): UseModInisResult {
  const [files, setFiles] = useState<ModIniFile[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Per-path cache. Cleared implicitly when the app restarts; a manual refresh
  // or a hash save re-reads from disk and overwrites the cached entry.
  const cacheRef = useRef<Map<string, ModIniFile[]>>(new Map());

  const load = useCallback(
    async (path: string, force: boolean) => {
      if (!force) {
        const cached = cacheRef.current.get(path);
        if (cached) {
          setFiles(cached);
          setError(null);
          setLoading(false);
          return;
        }
      }
      setLoading(true);
      setError(null);
      try {
        const result = await invoke<ModIniFile[]>("read_mod_inis", { modPath: path });
        cacheRef.current.set(path, result);
        setFiles(result);
      } catch (err) {
        setFiles([]);
        setError(typeof err === "string" ? err : "Failed to read mod ini files");
      } finally {
        setLoading(false);
      }
    },
    []
  );

  useEffect(() => {
    if (!modPath) {
      setFiles([]);
      setError(null);
      setLoading(false);
      return;
    }
    load(modPath, false);
  }, [modPath, load]);

  const refresh = useCallback(() => {
    if (modPath) load(modPath, true);
  }, [modPath, load]);

  const saveHashEdits = useCallback(
    async (edits: HashEdit[]): Promise<HashUpdateResult> => {
      const result = await invoke<HashUpdateResult>("update_mod_hashes", { edits });
      // Re-read from disk so line indices and values reflect what was written
      // (a saved edit shifts nothing, but this keeps the cache authoritative).
      if (modPath && result.files_written > 0) {
        await load(modPath, true);
      }
      return result;
    },
    [modPath, load]
  );

  return { files, loading, error, refresh, saveHashEdits };
}
