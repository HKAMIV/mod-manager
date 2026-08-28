import { useCallback, useEffect, useState } from "react";
import { invoke } from "../lib/invoke";
import type { GbModDetail } from "../types";

interface UseGameBananaModDetailResult {
  detail: GbModDetail | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

/** Fetches full detail (description, images, files) for a single GameBanana
 * mod. `modId` of null means "no mod selected" and skips fetching entirely. */
export function useGameBananaModDetail(modId: number | null): UseGameBananaModDetailResult {
  const [detail, setDetail] = useState<GbModDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchDetail = useCallback(async () => {
    if (modId === null) {
      setDetail(null);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<GbModDetail>("get_gamebanana_mod_detail", { modId });
      setDetail(result);
    } catch (err) {
      setDetail(null);
      setError(typeof err === "string" ? err : "Failed to load mod details");
    } finally {
      setLoading(false);
    }
  }, [modId]);

  useEffect(() => {
    fetchDetail();
  }, [fetchDetail]);

  return { detail, loading, error, refresh: fetchDetail };
}
