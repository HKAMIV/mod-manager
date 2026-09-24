import { useMemo, useState } from "react";
import { useAppStore } from "../stores/appStore";
import { useMods } from "../hooks/useMods";
import { useUpdateCheck } from "../hooks/useUpdateCheck";
import ModCard from "../components/ModCard";
import PresetBar from "../components/PresetBar";
import ConflictBanner from "../components/ConflictBanner";
import RestorePointsPanel from "../components/RestorePointsPanel";
import SymlinkMigrationBanner from "../components/SymlinkMigrationBanner";
import AddModDialog from "../components/AddModDialog";
import {
  Search,
  SlidersHorizontal,
  PanelRightOpen,
  FolderSearch,
  FolderX,
  AlertTriangle,
  RefreshCw,
  X,
  Eye,
  EyeOff,
  ListFilter,
  History,
  ArrowUpCircle,
  Trash2,
  FolderPlus,
} from "lucide-react";
import type { ModInfo, ModStatusFilter } from "../types";

const STATUS_OPTIONS: { value: ModStatusFilter; label: string }[] = [
  { value: "all", label: "All" },
  { value: "enabled", label: "Enabled" },
  { value: "disabled", label: "Disabled" },
];

function LocalView() {
  const { activeGame, detailPanelOpen, toggleDetailPanel, settings, setSelectedMod } =
    useAppStore();
  const gameName = activeGame.replace(/-/g, " ");
  const modPath = settings?.games.find((g) => g.id === activeGame)?.mod_path ?? null;

  const {
    filteredMods,
    mods,
    loading,
    error,
    search,
    setSearch,
    statusFilter,
    setStatusFilter,
    categoryFilter,
    setCategoryFilter,
    categories,
    refresh,
    toggleMod,
    batchToggle,
    deleteMods,
    toggleError,
    conflicts,
    conflictingIds,
    layoutStatus,
    migrating,
    migrationError,
    migrateToSymlinkLayout,
  } = useMods();

  const [filtersOpen, setFiltersOpen] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [checkedIds, setCheckedIds] = useState<Set<string>>(new Set());
  const [togglingId, setTogglingId] = useState<string | null>(null);
  const [batchWorking, setBatchWorking] = useState(false);
  const [restorePanelOpen, setRestorePanelOpen] = useState(false);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const [addModOpen, setAddModOpen] = useState(false);

  const { updates, checking: updateChecking, updateCount, checkNow: checkForUpdates } =
    useUpdateCheck();

  // Categories that currently have a conflict, for the small warning dot on
  // category filter chips — quick visual scan without opening the banner.
  const conflictingCategories = useMemo(
    () => new Set(conflicts.map((c) => c.category)),
    [conflicts]
  );

  const handleDisableConflicting = async (mod: ModInfo) => {
    setTogglingId(mod.id);
    await toggleMod(mod, false);
    setTogglingId(null);
  };

  const handleSelect = (mod: ModInfo) => {
    setSelectedId(mod.id);
    setSelectedMod(mod);
    if (!detailPanelOpen) toggleDetailPanel();
  };

  const handleCheckChange = (mod: ModInfo, checked: boolean) => {
    setCheckedIds((prev) => {
      const next = new Set(prev);
      if (checked) next.add(mod.id);
      else next.delete(mod.id);
      return next;
    });
  };

  const handleToggleEnabled = async (mod: ModInfo) => {
    setTogglingId(mod.id);
    await toggleMod(mod, !mod.enabled);
    setTogglingId(null);
  };

  const checkedMods = useMemo(
    () => mods.filter((m) => checkedIds.has(m.id)),
    [mods, checkedIds]
  );

  const clearSelection = () => setCheckedIds(new Set());

  const handleBatch = async (enabled: boolean) => {
    setBatchWorking(true);
    await batchToggle(checkedMods, enabled);
    setBatchWorking(false);
    clearSelection();
  };

  const handleBatchDelete = async () => {
    setBatchWorking(true);
    await deleteMods(checkedMods);
    setBatchWorking(false);
    setConfirmingDelete(false);
    clearSelection();
  };

  return (
    <div className="flex flex-col h-full">
      {/* Page heading */}
      <div className="px-5 pt-5 pb-1 flex items-start justify-between">
        <div>
          <h1 className="font-display text-2xl font-semibold text-text-primary tracking-wide">
            Local Mods
          </h1>
          <p className="text-xs text-text-muted mt-0.5 capitalize">
            {gameName}
            {mods.length > 0 && ` · ${mods.length} mod${mods.length === 1 ? "" : "s"}`}
          </p>
        </div>
        <div className="flex items-center gap-1.5">
          <button
            onClick={() => setAddModOpen(true)}
            disabled={
              !modPath ||
              // Blocked only when a migration is pending (legacy mods present,
              // symlink layout not yet active). A fresh directory is allowed —
              // the install auto-initializes the symlink layout.
              (!!layoutStatus && !layoutStatus.is_symlink_layout && layoutStatus.has_legacy_mods)
            }
            title={
              layoutStatus && !layoutStatus.is_symlink_layout && layoutStatus.has_legacy_mods
                ? "Migrate this game to the symlink layout to add mods manually"
                : undefined
            }
            className="game-control flex items-center gap-1.5 px-2.5 py-2 text-xs font-medium text-text-secondary hover:text-game hover:bg-surface-2 disabled:opacity-40 transition-colors"
            aria-label="Add mod"
          >
            <FolderPlus size={15} />
            Add Mod
          </button>
          <button
            onClick={() => setRestorePanelOpen(true)}
            disabled={!modPath}
            className="game-control flex items-center gap-1.5 px-2.5 py-2 text-xs font-medium text-text-secondary hover:text-game2 hover:bg-surface-2 disabled:opacity-40 transition-colors"
            aria-label="Restore points"
          >
            <History size={15} />
            Restore Points
          </button>
          <button
            onClick={checkForUpdates}
            disabled={!modPath || updateChecking}
            className="game-control flex items-center gap-1.5 px-2.5 py-2 text-xs font-medium text-text-secondary hover:text-gold hover:bg-surface-2 disabled:opacity-40 transition-colors"
            aria-label="Check for updates"
          >
            <ArrowUpCircle size={15} className={updateChecking ? "animate-spin" : ""} />
            {updateCount > 0
              ? `${updateCount} Update${updateCount === 1 ? "" : "s"}`
              : "Check Updates"}
          </button>
          <button
            onClick={refresh}
            disabled={!modPath || loading}
            className="game-control p-2 text-text-secondary hover:text-game hover:bg-surface-2 disabled:opacity-40 transition-colors"
            aria-label="Refresh mod list"
          >
            <RefreshCw size={16} className={loading ? "animate-spin" : ""} />
          </button>
        </div>
      </div>

      {restorePanelOpen && (
        <RestorePointsPanel
          onClose={() => setRestorePanelOpen(false)}
          onRestored={refresh}
        />
      )}

      {addModOpen && modPath && (
        <AddModDialog
          gameId={activeGame}
          existingCategories={categories}
          onInstalled={() => refresh()}
          onClose={() => setAddModOpen(false)}
        />
      )}

      {/* Toolbar */}
      <header className="flex items-center gap-3 px-5 py-3">
        <div className="game-control flex-1 flex items-center gap-2 px-3 py-2 bg-surface-1 border border-surface-3 focus-within:border-game/50 focus-within:shadow-glow-game transition-all">
          <Search size={16} className="text-text-muted" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search mods..."
            disabled={!modPath}
            className="flex-1 bg-transparent text-sm text-text-primary placeholder:text-text-muted focus:outline-none disabled:cursor-not-allowed"
          />
        </div>
        <button
          onClick={() => setFiltersOpen((v) => !v)}
          disabled={!modPath}
          className={`game-control p-2.5 border transition-colors disabled:opacity-40 ${
            filtersOpen
              ? "bg-game2/10 border-game2/40 text-game2"
              : "bg-surface-1 border-surface-3 hover:border-game2/40 text-text-secondary hover:text-game2"
          }`}
          aria-label="Filters"
          aria-expanded={filtersOpen}
        >
          <SlidersHorizontal size={16} />
        </button>
        {!detailPanelOpen && (
          <button
            onClick={toggleDetailPanel}
            className="game-control p-2.5 bg-surface-1 border border-surface-3 hover:border-game/40 text-text-secondary hover:text-game transition-colors"
            aria-label="Open detail panel"
          >
            <PanelRightOpen size={16} />
          </button>
        )}
      </header>

      {/* Presets */}
      {modPath && <PresetBar onApplied={refresh} />}

      {/* Conflict warnings — passive, dismissible, never blocks toggling */}
      {modPath && (
        <ConflictBanner
          conflicts={conflicts}
          onDisable={handleDisableConflicting}
          disablingId={togglingId}
        />
      )}

      {/* Phase 12: symlink layout migration prompt — shown when legacy mods
          are detected and the symlink layout is not yet active. Opt-in only. */}
      {modPath && layoutStatus && !layoutStatus.is_symlink_layout && layoutStatus.has_legacy_mods && (
        <SymlinkMigrationBanner
          layoutStatus={layoutStatus}
          migrating={migrating}
          migrationError={migrationError}
          onMigrate={migrateToSymlinkLayout}
        />
      )}

      {/* Filter row */}
      {filtersOpen && modPath && (
        <div className="flex flex-wrap items-center gap-2 px-5 pb-3">
          <div className="flex items-center gap-1 game-panel bg-surface-1 border border-surface-3 p-1">
            {STATUS_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                onClick={() => setStatusFilter(opt.value)}
                className={`game-control px-2.5 py-1 text-xs font-medium transition-colors ${
                  statusFilter === opt.value
                    ? "bg-game/15 text-game"
                    : "text-text-secondary hover:text-text-primary"
                }`}
              >
                {opt.label}
              </button>
            ))}
          </div>

          {categories.length > 0 && (
            <div className="flex items-center gap-1.5 flex-wrap">
              <button
                onClick={() => setCategoryFilter(null)}
                className={`game-control px-2.5 py-1 text-xs font-medium border transition-colors ${
                  categoryFilter === null
                    ? "bg-game2/10 border-game2/40 text-game2"
                    : "bg-surface-1 border-surface-3 text-text-secondary hover:text-text-primary"
                }`}
              >
                All categories
              </button>
              {categories.map((cat) => (
                <button
                  key={cat}
                  onClick={() => setCategoryFilter(cat)}
                  className={`game-control flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium border transition-colors ${
                    categoryFilter === cat
                      ? "bg-game2/10 border-game2/40 text-game2"
                      : "bg-surface-1 border-surface-3 text-text-secondary hover:text-text-primary"
                  }`}
                >
                  {cat}
                  {conflictingCategories.has(cat) && (
                    <span
                      className="w-1.5 h-1.5 rounded-full bg-game2 shadow-glow-game2"
                      aria-label="Has a possible conflict"
                    />
                  )}
                </button>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Batch action bar */}
      {checkedIds.size > 0 && (
        <div className="game-panel flex items-center justify-between mx-5 mb-3 px-3 py-2 bg-game/10 border border-game/30">
          <div className="flex items-center gap-2">
            <ListFilter size={14} className="text-game" />
            <span className="text-xs font-medium text-text-primary">
              {checkedIds.size} selected
            </span>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => handleBatch(true)}
              disabled={batchWorking}
              className="game-control flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium bg-game text-surface-0 hover:bg-game-hover disabled:opacity-50 transition-colors"
            >
              <Eye size={12} />
              Enable
            </button>
            <button
              onClick={() => handleBatch(false)}
              disabled={batchWorking}
              className="game-control flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium bg-surface-2 border border-surface-3 hover:border-game2/40 text-text-secondary hover:text-game2 disabled:opacity-50 transition-colors"
            >
              <EyeOff size={12} />
              Disable
            </button>
            {!confirmingDelete ? (
              <button
                onClick={() => setConfirmingDelete(true)}
                disabled={batchWorking}
                className="game-control flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium text-text-muted hover:text-game2 disabled:opacity-50 transition-colors"
              >
                <Trash2 size={12} />
                Delete
              </button>
            ) : (
              <button
                onClick={handleBatchDelete}
                disabled={batchWorking}
                className="game-control flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium bg-game2 text-surface-0 hover:bg-game2-hover disabled:opacity-50 transition-colors animate-pulse"
              >
                <Trash2 size={12} />
                {batchWorking ? "Deleting..." : `Confirm Delete ${checkedIds.size}`}
              </button>
            )}
            <button
              onClick={() => { clearSelection(); setConfirmingDelete(false); }}
              className="p-1 text-text-muted hover:text-text-primary transition-colors"
              aria-label="Clear selection"
            >
              <X size={14} />
            </button>
          </div>
        </div>
      )}

      {/* Toggle error banner */}
      {toggleError && (
        <div className="game-panel flex items-center gap-2 mx-5 mb-3 px-3 py-2 bg-game2/10 border border-game2/30 text-xs text-game2">
          <AlertTriangle size={13} className="shrink-0" />
          {toggleError}
        </div>
      )}

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-5 pb-5">
        {!modPath ? (
          <EmptyState
            icon={<FolderSearch size={28} className="text-game" />}
            title="No mod directory configured"
            description={
              <>
                Set a mod directory for{" "}
                <span className="text-game font-medium capitalize">{gameName}</span> in
                Settings to get started.
              </>
            }
          />
        ) : error ? (
          <EmptyState
            icon={<AlertTriangle size={28} className="text-game2" />}
            title="Couldn't read mod directory"
            description={error}
            tone="game2"
          />
        ) : loading && mods.length === 0 ? (
          <EmptyState
            icon={<RefreshCw size={28} className="text-game animate-spin" />}
            title="Scanning mods..."
            description="This should only take a moment."
          />
        ) : mods.length === 0 ? (
          <EmptyState
            icon={<FolderX size={28} className="text-game" />}
            title="No mods found"
            description="This directory doesn't contain any mod folders yet."
          />
        ) : filteredMods.length === 0 ? (
          <EmptyState
            icon={<Search size={28} className="text-game" />}
            title="No matches"
            description="Try a different search term or clear your filters."
          />
        ) : (
          <div className="grid gap-3 pt-2" style={{ gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))" }}>
            {filteredMods.map((mod) => (
              <ModCard
                key={mod.id}
                mod={mod}
                selected={mod.id === selectedId}
                onSelect={handleSelect}
                checked={checkedIds.has(mod.id)}
                onCheckChange={handleCheckChange}
                onToggleEnabled={handleToggleEnabled}
                toggling={togglingId === mod.id}
                conflicting={conflictingIds.has(mod.id)}
                hasUpdate={updates.has(mod.key)}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function EmptyState({
  icon,
  title,
  description,
  tone = "game",
}: {
  icon: React.ReactNode;
  title: string;
  description: React.ReactNode;
  tone?: "game" | "game2";
}) {
  const toneClasses =
    tone === "game2"
      ? "shadow-glow-game2 bg-game2/10 border-game2/30"
      : "shadow-glow-game bg-game/10 border-game/30";

  return (
    <div className="flex flex-1 h-full items-center justify-center p-8">
      <div className="text-center space-y-4 max-w-sm">
        <div
          className={`game-panel mx-auto w-16 h-16 flex items-center justify-center border ${toneClasses}`}
        >
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

export default LocalView;
