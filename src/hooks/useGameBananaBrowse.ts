import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "../lib/invoke";
import { useAppStore } from "../stores/appStore";
import type { GbBrowseResult, GbModSummary, GbSortOrder } from "../types";

interface UseGameBananaBrowseResult {
  mods: GbModSummary[];
  loading: boolean;
  error: string | null;
  page: number;
  setPage: (page: number) => void;
  sort: GbSortOrder;
  setSort: (sort: GbSortOrder) => void;
  search: string;
  setSearch: (value: string) => void;
  categoryId: number | null;
  setCategoryId: (id: number | null) => void;
  filteredMods: GbModSummary[];
  totalCount: number;
  hasNextPage: boolean;
  refresh: () => void;
}

const SORT_OPTIONS: { value: GbSortOrder; label: string }[] = [
  { value: "default", label: "Default" },
  { value: "new", label: "Newest" },
  { value: "updated", label: "Recently Updated" },
];

export { SORT_OPTIONS };

/**
 * Browses GameBanana's mod catalog for the active game. Pagination and sort
 * are server-side, routed to one of three endpoints depending on state:
 * - `search` non-empty: GameBanana's real full-text search endpoint (Subfeed
 *   has no free-text query parameter of its own); sort and category filter
 *   don't apply to search results — GameBanana ranks by relevance and the
 *   search endpoint ignores a category filter regardless.
 * - `categoryId` set (no search): Mod/Index filtered to that category (e.g.
 *   a specific character) — see useGameBananaCategories for browsing the
 *   category tree that produces this id.
 * - neither: the default Subfeed browse feed.
 * The NSFW filter setting is applied client-side using the `likely_nsfw`
 * heuristic each mod carries, since none of these endpoints expose a direct flag.
 */
export function useGameBananaBrowse(): UseGameBananaBrowseResult {
  const { activeGame, settings } = useAppStore();
  const [mods, setMods] = useState<GbModSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(1);
  const [sort, setSort] = useState<GbSortOrder>("default");
  const [search, setSearch] = useState("");
  const [categoryId, setCategoryId] = useState<number | null>(null);
  const [totalCount, setTotalCount] = useState(0);
  const [isComplete, setIsComplete] = useState(true);

  const nsfwFilterEnabled = settings?.nsfw_filter ?? true;

  const fetchPage = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<GbBrowseResult>("browse_gamebanana_mods", {
        gameId: activeGame,
        page,
        sort,
        categoryId,
        search: search.trim() || null,
      });
      setMods(result.mods);
      setTotalCount(result.total_count);
      setIsComplete(result.is_complete);
    } catch (err) {
      setMods([]);
      setError(typeof err === "string" ? err : "Failed to browse GameBanana");
    } finally {
      setLoading(false);
    }
  }, [activeGame, page, sort, search, categoryId]);

  // Clear the search box and character filter whenever the active game
  // changes, so state typed/picked for one game doesn't silently carry over
  // and filter another (category ids aren't meaningful across games anyway).
  useEffect(() => {
    setSearch("");
    setCategoryId(null);
  }, [activeGame]);

  // Search and category filtering are mutually exclusive server-side —
  // GameBanana's search endpoint ignores a category filter (verified live),
  // so starting a search always clears any active character filter.
  useEffect(() => {
    if (search) setCategoryId(null);
  }, [search]);

  // Reset to page 1 whenever the active game, sort, search query, or category
  // filter changes, so switching context never leaves a stale deep page with
  // no results. This runs before the fetch effect below on the same render pass.
  useEffect(() => {
    setPage(1);
  }, [activeGame, sort, search, categoryId]);

  // Fetch the current page, debounced only while the user is actively typing
  // a search query (page/sort/game changes fetch immediately).
  useEffect(() => {
    const handle = setTimeout(fetchPage, search ? 350 : 0);
    return () => clearTimeout(handle);
  }, [fetchPage, search]);

  const filteredMods = useMemo(() => {
    if (!nsfwFilterEnabled) return mods;
    return mods.filter((mod) => !mod.likely_nsfw);
  }, [mods, nsfwFilterEnabled]);

  return {
    mods,
    loading,
    error,
    page,
    setPage,
    sort,
    setSort,
    search,
    setSearch,
    categoryId,
    setCategoryId,
    filteredMods,
    totalCount,
    hasNextPage: !isComplete,
    refresh: fetchPage,
  };
}
