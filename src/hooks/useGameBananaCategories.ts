import { useCallback, useEffect, useState } from "react";
import { invoke } from "../lib/invoke";
import type { GameId, GbCategoryNode } from "../types";

/** One level of the category drill-down, with enough context to render a
 * breadcrumb trail and go back up a level. */
export interface CategoryBreadcrumb {
  id: number;
  name: string;
}

interface UseGameBananaCategoriesResult {
  /** Nodes at the current tree level (root categories, or the children of
   * the last breadcrumb entry). */
  nodes: GbCategoryNode[];
  loading: boolean;
  error: string | null;
  /** Path from the root down to (but not including) the current level —
   * empty at the root. */
  breadcrumbs: CategoryBreadcrumb[];
  /** Drill into a node that has children. */
  enterCategory: (node: GbCategoryNode) => void;
  /** Jump back to a specific breadcrumb (or the root, if none given). */
  goToBreadcrumb: (index: number | null) => void;
  refresh: () => void;
}

/**
 * Browses GameBanana's per-game category tree one level at a time (root
 * categories like "Skins"/"UI", then their children, and so on down to
 * individual characters). Tree depth varies per game — some have an
 * intermediate "Characters" grouping, others go straight from the root
 * category to character names — so this deliberately doesn't assume a fixed
 * depth or a "character" concept; the caller decides what a leaf node means
 * (a node with `has_children: false` is one the mod browser can filter by).
 */
export function useGameBananaCategories(activeGame: GameId): UseGameBananaCategoriesResult {
  const [nodes, setNodes] = useState<GbCategoryNode[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [breadcrumbs, setBreadcrumbs] = useState<CategoryBreadcrumb[]>([]);

  const currentParentId = breadcrumbs.length > 0 ? breadcrumbs[breadcrumbs.length - 1].id : null;

  const fetchLevel = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<GbCategoryNode[]>("get_gamebanana_categories", {
        gameId: activeGame,
        parentCategoryId: currentParentId,
      });
      setNodes(result);
    } catch (err) {
      setNodes([]);
      setError(typeof err === "string" ? err : "Failed to load categories");
    } finally {
      setLoading(false);
    }
  }, [activeGame, currentParentId]);

  // Reset to the root whenever the active game changes — category ids from
  // one game's tree aren't meaningful for another.
  useEffect(() => {
    setBreadcrumbs([]);
  }, [activeGame]);

  useEffect(() => {
    fetchLevel();
  }, [fetchLevel]);

  const enterCategory = useCallback((node: GbCategoryNode) => {
    setBreadcrumbs((prev) => [...prev, { id: node.id, name: node.name }]);
  }, []);

  const goToBreadcrumb = useCallback((index: number | null) => {
    if (index === null) {
      setBreadcrumbs([]);
      return;
    }
    setBreadcrumbs((prev) => prev.slice(0, index + 1));
  }, []);

  return {
    nodes,
    loading,
    error,
    breadcrumbs,
    enterCategory,
    goToBreadcrumb,
    refresh: fetchLevel,
  };
}
