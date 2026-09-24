import { useCallback, useEffect, useState } from "react";
import { useAppStore } from "../stores/appStore";
import { useGameBananaBrowse, SORT_OPTIONS } from "../hooks/useGameBananaBrowse";
import { useDownloads } from "../hooks/useDownloads";
import GbModCard from "../components/GbModCard";
import GbModDetailPanel from "../components/GbModDetailPanel";
import GbCategoryBrowser from "../components/GbCategoryBrowser";
import {
  Search,
  Globe,
  AlertTriangle,
  RefreshCw,
  ChevronLeft,
  ChevronRight,
  ShieldOff,
  Tags,
  X,
} from "lucide-react";
import type { GbModFile, GbSortOrder } from "../types";

function OnlineView() {
  const { activeGame, settings } = useAppStore();
  const gameName = activeGame.replace(/-/g, " ");

  const {
    filteredMods,
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
    totalCount,
    hasNextPage,
    refresh,
  } = useGameBananaBrowse();

  const [selectedModId, setSelectedModId] = useState<number | null>(null);
  const [categoryBrowserOpen, setCategoryBrowserOpen] = useState(false);
  const [activeCategoryName, setActiveCategoryName] = useState<string | null>(null);
  const nsfwFilterEnabled = settings?.nsfw_filter ?? true;
  const hiddenCount = mods.length - filteredMods.length;

  const handleCategorySelect = (node: { id: number; name: string }) => {
    setCategoryId(node.id);
    setActiveCategoryName(node.name);
    setCategoryBrowserOpen(false);
  };

  const clearCategoryFilter = () => {
    setCategoryId(null);
    setActiveCategoryName(null);
  };

  // The browse hook itself clears categoryId when the active game changes or
  // a search starts (server-side filters that don't compose) — keep the
  // locally-tracked display name in sync with that so the chip doesn't show
  // a stale character name for a filter that's no longer applied.
  useEffect(() => {
    if (categoryId === null) setActiveCategoryName(null);
  }, [categoryId]);

  const { queue } = useDownloads();
  const [downloadToast, setDownloadToast] = useState<{ message: string; isError: boolean } | null>(
    null
  );

  const showToast = (message: string, isError = false) => {
    setDownloadToast({ message, isError });
    window.setTimeout(() => setDownloadToast(null), isError ? 6000 : 4000);
  };

  // Stable identity so the memoized GbModCard grid doesn't re-render every card
  // on each OnlineView render (e.g. while a download toast ticks).
  const handleSelectMod = useCallback((mod: { id: number }) => setSelectedModId(mod.id), []);

  const handleDownload = async (
    file: GbModFile,
    modName: string,
    modId: number,
    categoryHint: string | null,
    previewImageUrl: string | null
  ) => {
    const { item, error } = await queue({
      gameId: activeGame,
      modId,
      modName,
      file,
      categoryHint,
      previewImageUrl,
    });
    if (item) {
      showToast(`Queued "${modName}" — see Downloads for progress.`);
    } else {
      // queue() failing most commonly means this game has no mod directory
      // configured yet (Settings), or the file isn't a .zip/.7z/.rar we can
      // auto-install — either way, this must surface immediately rather
      // than fail silently, since nothing else would ever tell the user.
      showToast(error ?? `Couldn't queue "${modName}" for download.`, true);
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Page heading */}
      <div className="px-5 pt-5 pb-1 flex items-start justify-between">
        <div>
          <h1 className="font-display text-2xl font-semibold text-text-primary tracking-wide">
            Online Mods
          </h1>
          <p className="text-xs text-text-muted mt-0.5 capitalize">
            {gameName}
            {totalCount > 0 && ` · ${totalCount.toLocaleString()} mods on GameBanana`}
          </p>
        </div>
        <button
          onClick={refresh}
          disabled={loading}
          className="game-control p-2 text-text-secondary hover:text-game hover:bg-surface-2 disabled:opacity-40 transition-colors"
          aria-label="Refresh"
        >
          <RefreshCw size={16} className={loading ? "animate-spin" : ""} />
        </button>
      </div>

      {/* Toolbar */}
      <header className="flex items-center gap-3 px-5 py-3">
        <div className="game-control flex-1 flex items-center gap-2 px-3 py-2 bg-surface-1 border border-surface-3 focus-within:border-game2/50 focus-within:shadow-glow-game2 transition-all">
          <Search size={16} className="text-text-muted" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search GameBanana..."
            className="flex-1 bg-transparent text-sm text-text-primary placeholder:text-text-muted focus:outline-none"
          />
        </div>

        {!search && (
          <button
            onClick={() => setCategoryBrowserOpen((v) => !v)}
            className={`game-control flex items-center gap-1.5 px-3 py-2 text-xs font-medium border transition-colors whitespace-nowrap ${
              categoryBrowserOpen || activeCategoryName
                ? "bg-game2/10 border-game2/40 text-game2"
                : "bg-surface-1 border-surface-3 text-text-secondary hover:border-game2/40 hover:text-game2"
            }`}
            aria-expanded={categoryBrowserOpen}
          >
            <Tags size={14} />
            {activeCategoryName ?? "Browse Characters"}
          </button>
        )}

        {!search && (
          <div className="flex items-center gap-1 game-panel bg-surface-1 border border-surface-3 p-1">
            {SORT_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                onClick={() => setSort(opt.value as GbSortOrder)}
                className={`game-control px-2.5 py-1 text-xs font-medium transition-colors whitespace-nowrap ${
                  sort === opt.value
                    ? "bg-game2/15 text-game2"
                    : "text-text-secondary hover:text-text-primary"
                }`}
              >
                {opt.label}
              </button>
            ))}
          </div>
        )}
      </header>

      {/* Category browser — drills into GameBanana's category tree (e.g.
          Skins > Characters > Furina) to filter the grid by a specific
          character rather than only what happens to be on the current page. */}
      {categoryBrowserOpen && !search && (
        <div className="px-5 pb-3">
          <GbCategoryBrowser activeGame={activeGame} onSelect={handleCategorySelect} />
        </div>
      )}

      {/* Active category filter chip */}
      {activeCategoryName && !search && (
        <div className="flex items-center gap-2 px-5 pb-3">
          <span className="game-control flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium bg-game/10 border border-game/30 text-game">
            <Tags size={11} />
            {activeCategoryName}
            <button
              onClick={clearCategoryFilter}
              className="hover:text-game-hover transition-colors"
              aria-label="Clear character filter"
            >
              <X size={12} />
            </button>
          </span>
        </div>
      )}

      {/* NSFW filter notice */}
      {nsfwFilterEnabled && hiddenCount > 0 && (
        <div className="game-panel flex items-center gap-2 mx-5 mb-3 px-3 py-2 bg-surface-2 border border-surface-3 text-xs text-text-muted">
          <ShieldOff size={13} className="shrink-0" />
          {hiddenCount} mod{hiddenCount === 1 ? "" : "s"} hidden by the NSFW filter (Settings)
        </div>
      )}

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-5 pb-5">
        {error ? (
          <EmptyState
            icon={<AlertTriangle size={28} className="text-game2" />}
            title="Couldn't reach GameBanana"
            description={error}
          />
        ) : loading && mods.length === 0 ? (
          <EmptyState
            icon={<RefreshCw size={28} className="text-game2 animate-spin" />}
            title="Loading mods..."
            description="Fetching the latest from GameBanana."
          />
        ) : filteredMods.length === 0 ? (
          <EmptyState
            icon={<Globe size={28} className="text-game2" />}
            title="No mods found"
            description={
              search
                ? "Try a different search term."
                : "This game has no mods listed yet on GameBanana."
            }
          />
        ) : (
          <>
            <div
              className="grid gap-3 pt-2"
              style={{ gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))" }}
            >
              {filteredMods.map((mod) => (
                <GbModCard key={mod.id} mod={mod} onSelect={handleSelectMod} />
              ))}
            </div>

            {/* Pagination */}
            {!search && (
              <div className="flex items-center justify-center gap-3 mt-5">
                <button
                  onClick={() => setPage(Math.max(1, page - 1))}
                  disabled={page <= 1 || loading}
                  className="game-control flex items-center gap-1 px-3 py-1.5 text-xs font-medium bg-surface-1 border border-surface-3 hover:border-game2/40 text-text-secondary hover:text-game2 disabled:opacity-40 transition-colors"
                >
                  <ChevronLeft size={13} />
                  Previous
                </button>
                <span className="text-xs text-text-muted">Page {page}</span>
                <button
                  onClick={() => setPage(page + 1)}
                  disabled={!hasNextPage || loading}
                  className="game-control flex items-center gap-1 px-3 py-1.5 text-xs font-medium bg-surface-1 border border-surface-3 hover:border-game2/40 text-text-secondary hover:text-game2 disabled:opacity-40 transition-colors"
                >
                  Next
                  <ChevronRight size={13} />
                </button>
              </div>
            )}
          </>
        )}
      </div>

      {selectedModId !== null && (
        <GbModDetailPanel
          modId={selectedModId}
          onClose={() => setSelectedModId(null)}
          onDownload={handleDownload}
        />
      )}

      {downloadToast && (
        <div
          className={`fixed bottom-5 right-5 z-50 game-panel px-4 py-2.5 bg-surface-1 text-sm text-text-primary max-w-sm ${
            downloadToast.isError
              ? "border border-game2/40 shadow-glow-game2"
              : "border border-game/30 shadow-glow-game"
          }`}
        >
          {downloadToast.message}
        </div>
      )}
    </div>
  );
}

function EmptyState({
  icon,
  title,
  description,
}: {
  icon: React.ReactNode;
  title: string;
  description: React.ReactNode;
}) {
  return (
    <div className="flex flex-1 h-full items-center justify-center p-8">
      <div className="text-center space-y-4 max-w-sm">
        <div className="game-panel mx-auto w-16 h-16 flex items-center justify-center border shadow-glow-game2 bg-game2/10 border-game2/30">
          {icon}
        </div>
        <div className="space-y-1.5">
          <p className="font-display text-lg font-medium text-text-secondary">{title}</p>
          <p className="text-sm text-text-muted">{description}</p>
        </div>
      </div>
    </div>
  );
}

export default OnlineView;
