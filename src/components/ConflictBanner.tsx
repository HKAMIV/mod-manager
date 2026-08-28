import { useState } from "react";
import { AlertTriangle, ChevronDown, EyeOff, X } from "lucide-react";
import type { CategoryConflict, ModInfo } from "../types";

interface Props {
  conflicts: CategoryConflict[];
  onDisable: (mod: ModInfo) => void;
  disablingId: string | null;
}

/**
 * Passive conflict warning: lists categories with more than one enabled mod.
 * This is a heuristic, not a rule — deliberately doesn't block anything or
 * require the user to "resolve" it, since split mods (e.g. a hair-only mod
 * and an outfit-only mod for the same character) can legitimately coexist.
 * The user can dismiss the banner for the current session; it reappears next
 * time the app opens or the conflict set changes, since we don't track which
 * specific combinations have been reviewed and cleared.
 */
function ConflictBanner({ conflicts, onDisable, disablingId }: Props) {
  const [collapsed, setCollapsed] = useState(false);
  // Dismissal is keyed to the exact set of conflicting categories, not a
  // plain boolean — so if the user dismisses "Furina has 2 enabled mods" and
  // later "Nahida" also ends up with 2 enabled, the banner reappears for the
  // new situation instead of staying hidden forever. We deliberately don't
  // persist this anywhere; it resets on every app launch too, since we're
  // not tracking which specific mod combinations a user has reviewed.
  const [dismissedKey, setDismissedKey] = useState<string | null>(null);

  const conflictKey = conflicts.map((c) => c.category).sort().join("|");
  if (conflicts.length === 0 || dismissedKey === conflictKey) return null;

  const totalMods = conflicts.reduce((sum, c) => sum + c.mods.length, 0);

  return (
    <div className="game-panel mx-5 mb-3 bg-game2/5 border border-game2/30 overflow-hidden">
      <button
        onClick={() => setCollapsed((v) => !v)}
        className="flex items-center justify-between w-full px-3 py-2 text-left"
      >
        <span className="flex items-center gap-2">
          <AlertTriangle size={14} className="text-game2 shrink-0" />
          <span className="text-xs font-medium text-text-primary">
            {conflicts.length} possible conflict{conflicts.length === 1 ? "" : "s"}
          </span>
          <span className="text-xs text-text-muted">
            · {totalMods} mods enabled in the same category
          </span>
        </span>
        <span className="flex items-center gap-1 shrink-0">
          <span
            role="button"
            tabIndex={0}
            onClick={(e) => {
              e.stopPropagation();
              setDismissedKey(conflictKey);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                e.stopPropagation();
                setDismissedKey(conflictKey);
              }
            }}
            className="p-1 rounded text-text-muted hover:text-text-primary hover:bg-surface-2 transition-colors"
            aria-label="Dismiss for this session"
          >
            <X size={13} />
          </span>
          <ChevronDown
            size={14}
            className={`text-text-muted transition-transform ${collapsed ? "" : "rotate-180"}`}
          />
        </span>
      </button>

      {!collapsed && (
        <div className="px-3 pb-3 space-y-3">
          <p className="text-xs text-text-muted leading-relaxed">
            Most character mods fully replace the same model, so having more than one enabled
            at once usually means only one will actually show in-game. This is a heuristic —
            mods that alter different parts of a character (e.g. hair vs. outfit) can coexist
            safely. Nothing is blocked; disable extras only if this doesn't match your setup.
          </p>
          <div className="space-y-2">
            {conflicts.map((conflict) => (
              <div key={conflict.category} className="space-y-1">
                <p className="hud-label text-[10px] text-game2">{conflict.category}</p>
                <div className="space-y-1">
                  {conflict.mods.map((mod) => (
                    <div
                      key={mod.id}
                      className="flex items-center justify-between px-2.5 py-1.5 bg-surface-1 border border-surface-3 rounded-md"
                    >
                      <span className="text-xs text-text-primary truncate">{mod.name}</span>
                      <button
                        onClick={() => onDisable(mod)}
                        disabled={disablingId === mod.id}
                        className="game-control flex items-center gap-1 px-2 py-1 text-[11px] font-medium bg-surface-2 border border-surface-3 hover:border-game2/40 text-text-secondary hover:text-game2 disabled:opacity-50 transition-colors shrink-0 ml-2"
                      >
                        <EyeOff size={10} />
                        Disable
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export default ConflictBanner;
