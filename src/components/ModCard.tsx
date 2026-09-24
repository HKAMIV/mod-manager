import { memo, useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { ImageOff, FileStack, Check, AlertTriangle, ArrowUpCircle } from "lucide-react";
import type { ModInfo } from "../types";

interface Props {
  mod: ModInfo;
  selected: boolean;
  onSelect: (mod: ModInfo) => void;
  checked: boolean;
  onCheckChange: (mod: ModInfo, checked: boolean) => void;
  onToggleEnabled: (mod: ModInfo) => void;
  toggling?: boolean;
  /** True if this mod shares its category with another enabled mod — a
   * passive heuristic warning, not a hard block (see useMods' `conflicts`). */
  conflicting?: boolean;
  /** True if a newer version exists on GameBanana (Phase 8 update tracking). */
  hasUpdate?: boolean;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function ModCard({
  mod,
  selected,
  onSelect,
  checked,
  onCheckChange,
  onToggleEnabled,
  toggling = false,
  conflicting = false,
  hasUpdate = false,
}: Props) {
  const [imageError, setImageError] = useState(false);
  const previewSrc = mod.preview_path ? convertFileSrc(mod.preview_path) : null;

  // Reset the broken-image fallback whenever the mod (and thus its preview
  // path) changes, so switching games/mods doesn't get stuck on a stale error.
  useEffect(() => {
    setImageError(false);
  }, [mod.preview_path]);

  return (
    <div
      className={`game-panel card-lift card-enter content-auto group relative flex flex-col overflow-hidden border ${
        selected
          ? "border-game shadow-glow-game bg-game/5"
          : conflicting
          ? "border-game2/50 hover:border-game2"
          : "border-surface-3 bg-surface-1 hover:border-game/40 hover:shadow-glow-game"
      } ${!mod.enabled ? "opacity-60" : ""}`}
    >
      {/* Conflict accent bar — passive warning, doesn't block anything */}
      {conflicting && (
        <div className="absolute top-0 left-0 right-0 h-0.5 bg-game2 shadow-glow-game2 z-10" />
      )}

      <button
        onClick={() => onSelect(mod)}
        className="flex flex-col text-left w-full"
        aria-label={`View details for ${mod.name}`}
      >
        {/* Preview image */}
        <div className="relative aspect-video bg-surface-2 flex items-center justify-center overflow-hidden">
          {previewSrc && !imageError ? (
            <img
              src={previewSrc}
              alt={mod.name}
              onError={() => setImageError(true)}
              loading="lazy"
              decoding="async"
              width={320}
              height={180}
              className="w-full h-full object-cover"
            />
          ) : (
            <ImageOff size={22} className="text-text-muted" />
          )}

          {/* Selection checkbox */}
          <span
            role="checkbox"
            aria-checked={checked}
            tabIndex={0}
            onClick={(e) => {
              e.stopPropagation();
              onCheckChange(mod, !checked);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                e.stopPropagation();
                onCheckChange(mod, !checked);
              }
            }}
            className={`absolute top-2 left-2 w-5 h-5 rounded-md border flex items-center justify-center transition-colors cursor-pointer ${
              checked
                ? "bg-game border-game text-surface-0"
                : "bg-surface-0/60 border-surface-3 text-transparent opacity-0 group-hover:opacity-100 group-hover:backdrop-blur-md"
            }`}
          >
            <Check size={13} strokeWidth={3} />
          </span>

          {/* Update available badge — gold per design system */}
          {hasUpdate && (
            <span
              className="absolute top-2 right-2 flex items-center gap-1 px-1.5 py-0.5 rounded-full bg-gold/90 text-surface-0 text-[9px] font-bold shadow-glow-gold backdrop-blur-sm"
              aria-label="Update available on GameBanana"
            >
              <ArrowUpCircle size={10} />
              Update
            </span>
          )}
        </div>

        {/* Info */}
        <div className="p-3 space-y-1">
          <p className="text-sm font-medium text-text-primary truncate">{mod.name}</p>
          <div className="flex items-center justify-between text-xs text-text-muted">
            <span className="flex items-center gap-1 min-w-0">
              {conflicting && (
                <AlertTriangle
                  size={11}
                  className="text-game2 shrink-0"
                  aria-label="Possible conflict with another enabled mod in this category"
                />
              )}
              <span className="truncate">{mod.category}</span>
            </span>
            <span className="flex items-center gap-1 shrink-0 ml-2">
              <FileStack size={11} />
              {formatSize(mod.size_bytes)}
            </span>
          </div>
        </div>
      </button>

      {/* Enable/disable toggle */}
      <div className="flex items-center justify-between px-3 pb-3">
        <span
          className={`text-[10px] font-medium ${mod.enabled ? "text-game" : "text-text-muted"}`}
        >
          {mod.enabled ? "Enabled" : "Disabled"}
        </span>
        <button
          onClick={(e) => {
            e.stopPropagation();
            onToggleEnabled(mod);
          }}
          disabled={toggling}
          role="switch"
          aria-checked={mod.enabled}
          aria-label={mod.enabled ? `Disable ${mod.name}` : `Enable ${mod.name}`}
          className={`relative w-8 h-[18px] rounded-full transition-colors disabled:opacity-50 ${
            mod.enabled ? "bg-game shadow-glow-game" : "bg-surface-3"
          }`}
        >
          <span
            key={mod.enabled ? "on" : "off"}
            className={`toggle-pop absolute top-0.5 left-0.5 w-3.5 h-3.5 rounded-full bg-surface-0 transition-transform ${
              mod.enabled ? "translate-x-3.5" : "translate-x-0"
            }`}
          />
        </button>
      </div>
    </div>
  );
}

// Memoized: the LocalView grid can hold hundreds of these, and a single
// interaction (search keystroke, one toggle, hover) re-renders LocalView.
// Without memo every card re-renders and re-composites its glow/blur; with it
// (plus useCallback'd handlers in LocalView) only the cards whose props
// actually changed re-render.
export default memo(ModCard);
