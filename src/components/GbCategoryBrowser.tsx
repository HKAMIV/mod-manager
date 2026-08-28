import { ChevronRight, Folder, FolderOpen, Loader2, AlertTriangle, Layers } from "lucide-react";
import { useGameBananaCategories } from "../hooks/useGameBananaCategories";
import type { GameId, GbCategoryNode } from "../types";

interface Props {
  activeGame: GameId;
  /** Called when the user picks a category (leaf or otherwise) to filter the
   * mod grid by. Closes the browser after selecting. */
  onSelect: (node: GbCategoryNode) => void;
}

/**
 * Drill-down browser for GameBanana's per-game category tree — mirrors the
 * site's own "Skins > Characters > Furina" navigation so users can filter
 * the mod grid down to a specific character (or any other category level)
 * instead of only being able to filter by what happens to already be on the
 * current page. Tree depth varies per game (some have an intermediate
 * "Characters" grouping, some don't), so this always fetches one level at a
 * time rather than assuming a fixed structure.
 */
function GbCategoryBrowser({ activeGame, onSelect }: Props) {
  const { nodes, loading, error, breadcrumbs, enterCategory, goToBreadcrumb } =
    useGameBananaCategories(activeGame);

  const currentLevelName =
    breadcrumbs.length > 0 ? breadcrumbs[breadcrumbs.length - 1].name : null;

  const handleRowClick = (node: GbCategoryNode) => {
    if (node.has_children) {
      enterCategory(node);
    } else {
      onSelect(node);
    }
  };

  return (
    <div className="game-panel bg-surface-1 border border-surface-3 p-3 space-y-2.5 max-h-80 flex flex-col">
      {/* Breadcrumbs */}
      <div className="flex items-center gap-1 text-xs flex-wrap shrink-0">
        <button
          onClick={() => goToBreadcrumb(null)}
          className={`hover:text-game2 transition-colors ${
            breadcrumbs.length === 0 ? "text-game2 font-medium" : "text-text-muted"
          }`}
        >
          All Categories
        </button>
        {breadcrumbs.map((crumb, i) => (
          <span key={crumb.id} className="flex items-center gap-1">
            <ChevronRight size={12} className="text-text-muted shrink-0" />
            <button
              onClick={() => goToBreadcrumb(i)}
              className={`hover:text-game2 transition-colors ${
                i === breadcrumbs.length - 1 ? "text-game2 font-medium" : "text-text-muted"
              }`}
            >
              {crumb.name}
            </button>
          </span>
        ))}
      </div>

      {/* Select the current level itself (useful for intermediate groupings
          like "Characters" that may have their own directly-tagged mods) */}
      {breadcrumbs.length > 0 && (
        <button
          onClick={() =>
            onSelect({
              id: breadcrumbs[breadcrumbs.length - 1].id,
              name: breadcrumbs[breadcrumbs.length - 1].name,
              item_count: 0,
              has_children: true,
              icon_url: null,
            })
          }
          className="game-control flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-medium bg-game2/10 border border-game2/30 text-game2 hover:bg-game2/15 transition-colors shrink-0 self-start"
        >
          <Layers size={12} />
          View all in "{currentLevelName}"
        </button>
      )}

      {/* Category list */}
      <div className="flex-1 overflow-y-auto -mx-1 px-1">
        {loading ? (
          <div className="flex items-center justify-center gap-2 py-8 text-xs text-text-muted">
            <Loader2 size={14} className="animate-spin" />
            Loading categories...
          </div>
        ) : error ? (
          <div className="flex items-center justify-center gap-2 py-8 text-xs text-game2">
            <AlertTriangle size={14} />
            {error}
          </div>
        ) : nodes.length === 0 ? (
          <p className="text-xs text-text-muted text-center py-8">No categories found.</p>
        ) : (
          <div className="grid grid-cols-2 gap-1.5">
            {nodes.map((node) => (
              <button
                key={node.id}
                onClick={() => handleRowClick(node)}
                className="game-control flex items-center justify-between gap-2 px-2.5 py-2 text-left bg-surface-2 border border-surface-3 hover:border-game2/40 transition-colors"
              >
                <span className="flex items-center gap-2 min-w-0">
                  {node.icon_url ? (
                    <img src={node.icon_url} alt="" className="w-4 h-4 shrink-0 object-contain" />
                  ) : node.has_children ? (
                    <FolderOpen size={14} className="text-text-muted shrink-0" />
                  ) : (
                    <Folder size={14} className="text-text-muted shrink-0" />
                  )}
                  <span className="text-xs text-text-primary truncate">{node.name}</span>
                </span>
                <span className="flex items-center gap-1 shrink-0 text-[10px] text-text-muted">
                  {node.item_count > 0 && node.item_count.toLocaleString()}
                  {node.has_children && <ChevronRight size={12} />}
                </span>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export default GbCategoryBrowser;
