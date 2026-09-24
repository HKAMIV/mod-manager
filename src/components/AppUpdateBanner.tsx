import { open as openUrl } from "@tauri-apps/plugin-shell";
import {
  Download,
  X,
  Loader2,
  AlertTriangle,
  ExternalLink,
  Sparkles,
} from "lucide-react";
import { useAppUpdate, RELEASES_URL } from "../hooks/useAppUpdate";

/**
 * Floating banner shown when a newer signed release is available. Offers the
 * changelog and a one-click update (AppImage/macOS). If the install fails — or
 * for package formats the updater can't replace (e.g. .deb) — it points the
 * user at the GitHub release to download manually, so a failed update never
 * leaves them stuck.
 *
 * Startup checks are silent on failure (see useAppUpdate), so this renders
 * nothing unless an update is genuinely available or an install errored.
 */
export default function AppUpdateBanner() {
  const { phase, version, notes, progress, error, visible, installUpdate, dismiss } =
    useAppUpdate();

  if (!visible) return null;

  const busy = phase === "downloading" || phase === "installing";
  const failed = phase === "error";

  const progressPct =
    progress !== null ? Math.round(progress * 100) : null;

  return (
    <div className="fixed bottom-4 right-4 z-50 w-[22rem] max-w-[calc(100vw-2rem)]">
      <div className="game-panel glass-panel border border-game/40 shadow-glow-game overflow-hidden">
        {/* Header */}
        <div className="flex items-start justify-between gap-3 px-4 pt-3.5 pb-2">
          <div className="flex items-center gap-2">
            <Sparkles size={15} className="text-game shrink-0" />
            <div>
              <p className="hud-label text-[10px] text-game">Update Available</p>
              <p className="text-sm font-semibold text-text-primary font-display leading-tight">
                Version {version}
              </p>
            </div>
          </div>
          {!busy && (
            <button
              onClick={dismiss}
              className="p-1 text-text-muted hover:text-text-primary transition-colors"
              aria-label="Dismiss"
            >
              <X size={15} />
            </button>
          )}
        </div>

        {/* Changelog */}
        {notes && !failed && (
          <div className="px-4 pb-2">
            <div className="hud-rule mb-2" />
            <p className="hud-label text-[9px] text-text-muted mb-1">Changelog</p>
            <div className="max-h-40 overflow-y-auto text-xs text-text-secondary leading-relaxed whitespace-pre-line pr-1">
              {notes.trim()}
            </div>
          </div>
        )}

        {/* Progress */}
        {busy && (
          <div className="px-4 pb-2 space-y-1.5">
            <div className="flex items-center justify-between text-[11px] text-text-secondary">
              <span>{phase === "installing" ? "Installing…" : "Downloading…"}</span>
              {progressPct !== null && <span className="font-mono">{progressPct}%</span>}
            </div>
            <div className="h-1.5 bg-surface-2 rounded-full overflow-hidden">
              <div
                className="h-full bg-game shadow-glow-game transition-all duration-200"
                style={{
                  width: progressPct !== null ? `${progressPct}%` : "40%",
                }}
              />
            </div>
          </div>
        )}

        {/* Error + manual fallback */}
        {failed && (
          <div className="px-4 pb-2 space-y-2">
            <div className="flex items-start gap-2 game-control px-2.5 py-2 bg-game2/10 border border-game2/30 text-xs text-game2">
              <AlertTriangle size={13} className="shrink-0 mt-0.5" />
              <span>{error ?? "The update couldn't be installed."}</span>
            </div>
            <p className="text-[11px] text-text-muted">
              You can download the new version manually instead.
            </p>
          </div>
        )}

        {/* Actions */}
        <div className="flex items-center justify-end gap-2 px-4 pb-3.5 pt-1">
          {failed ? (
            <>
              <button
                onClick={dismiss}
                className="game-control px-3 py-1.5 text-xs text-text-secondary hover:text-text-primary bg-surface-1 border border-surface-3 transition-colors"
              >
                Dismiss
              </button>
              <button
                onClick={() => openUrl(RELEASES_URL)}
                className="game-control flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium bg-game text-surface-0 hover:bg-game-hover shadow-glow-game transition-colors"
              >
                <ExternalLink size={12} />
                Open GitHub
              </button>
            </>
          ) : (
            <>
              <button
                onClick={dismiss}
                disabled={busy}
                className="game-control px-3 py-1.5 text-xs text-text-secondary hover:text-text-primary bg-surface-1 border border-surface-3 disabled:opacity-40 transition-colors"
              >
                Later
              </button>
              <button
                onClick={installUpdate}
                disabled={busy}
                className="game-control flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium bg-game text-surface-0 hover:bg-game-hover disabled:opacity-60 shadow-glow-game transition-colors"
              >
                {busy ? <Loader2 size={12} className="animate-spin" /> : <Download size={12} />}
                {busy ? "Updating…" : "Update now"}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
