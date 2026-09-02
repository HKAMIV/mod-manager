import { useState } from "react";
import { GitMerge, CheckCircle2, AlertTriangle, X, Loader2 } from "lucide-react";
import type { LayoutStatus, MigrationResult } from "../types";

interface Props {
  layoutStatus: LayoutStatus;
  migrating: boolean;
  migrationError: string | null;
  onMigrate: (withRestorePoint: boolean) => Promise<MigrationResult | null>;
}

/**
 * Banner shown in LocalView when a game's mod directory is still using the
 * legacy DISABLED_-prefix rename layout.
 *
 * Explains what the symlink layout is and why it matters (3DMigoto/XXMI
 * persistent settings stop breaking on every toggle), then offers a one-click
 * migration with an opt-in restore point checkbox.
 *
 * After a successful migration the banner collapses into a success state and
 * the user can dismiss it. It never auto-dismisses so the outcome is visible.
 */
export default function SymlinkMigrationBanner({
  layoutStatus,
  migrating,
  migrationError,
  onMigrate,
}: Props) {
  const [dismissed, setDismissed] = useState(false);
  const [withRestorePoint, setWithRestorePoint] = useState(true);
  const [result, setResult] = useState<MigrationResult | null>(null);

  // If already on symlink layout or dismissed, render nothing.
  if (layoutStatus.is_symlink_layout || dismissed) return null;

  const handleMigrate = async () => {
    const r = await onMigrate(withRestorePoint);
    if (r) setResult(r);
  };

  if (result) {
    // Post-migration success state.
    return (
      <div className="game-panel mx-5 mb-3 border border-game/30 bg-game/8 overflow-hidden">
        <div className="flex items-start justify-between gap-3 px-4 py-3">
          <div className="flex items-start gap-3 min-w-0">
            <CheckCircle2 size={16} className="text-game shrink-0 mt-0.5" />
            <div className="min-w-0">
              <p className="text-xs font-medium text-text-primary">
                Migration complete —{" "}
                <span className="text-game">{result.migrated_count} mod{result.migrated_count === 1 ? "" : "s"}</span> moved
                to symlink layout
              </p>
              {result.restore_point_id && (
                <p className="text-xs text-text-muted mt-0.5">
                  Restore point saved (ID: <code className="font-mono">{result.restore_point_id}</code>)
                </p>
              )}
              {result.errors.length > 0 && (
                <p className="text-xs text-game2 mt-1">
                  {result.skipped_count} mod{result.skipped_count === 1 ? "" : "s"} skipped — check restore points if needed.
                </p>
              )}
            </div>
          </div>
          <button
            onClick={() => setDismissed(true)}
            className="shrink-0 p-1 text-text-muted hover:text-text-primary transition-colors"
            aria-label="Dismiss"
          >
            <X size={14} />
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="game-panel mx-5 mb-3 border border-game2/25 bg-surface-1/70 overflow-hidden">
      {/* Header row */}
      <div className="flex items-center justify-between gap-3 px-4 pt-3 pb-2">
        <div className="flex items-center gap-2">
          <GitMerge size={15} className="text-game2 shrink-0" />
          <p className="text-xs font-semibold text-text-primary uppercase tracking-wide font-display">
            Upgrade to Symlink Layout
          </p>
        </div>
        <button
          onClick={() => setDismissed(true)}
          className="p-1 text-text-muted hover:text-text-primary transition-colors"
          aria-label="Dismiss for this session"
        >
          <X size={13} />
        </button>
      </div>

      {/* Body */}
      <div className="px-4 pb-3 space-y-2">
        <p className="text-xs text-text-secondary leading-relaxed">
          Your{" "}
          <span className="text-text-primary font-medium">
            {layoutStatus.legacy_mod_count} mod{layoutStatus.legacy_mod_count === 1 ? "" : "s"}
          </span>{" "}
          are using the older rename-based layout. The new symlink layout keeps mod files in a stable
          location so <span className="text-game2 font-medium">3DMigoto / XXMI toggle settings</span> (key-swap
          variants, <code className="font-mono text-[10px]">$active</code> vars) are never lost when you enable
          or disable a mod.
        </p>

        {/* What changes callout */}
        <div className="game-control bg-surface-2/60 border border-surface-3 px-3 py-2 text-xs text-text-muted space-y-0.5">
          <p className="text-text-secondary font-medium mb-1">What changes:</p>
          <p>• Mod files move into <code className="font-mono text-[10px] text-game2">managed_src/</code> — paths stabilise forever</p>
          <p>• Enable/disable creates or removes a symlink in <code className="font-mono text-[10px] text-game2">managed_tgt/</code></p>
          <p>• <code className="font-mono text-[10px] text-game2">d3dx_user.ini</code> keys are rewritten to the new paths automatically</p>
          <p>• Existing mod enabled/disabled states are preserved exactly</p>
        </div>

        {/* Restore point checkbox */}
        <label className="flex items-center gap-2 cursor-pointer select-none group">
          <input
            type="checkbox"
            checked={withRestorePoint}
            onChange={(e) => setWithRestorePoint(e.target.checked)}
            className="game-control accent-game w-3.5 h-3.5"
          />
          <span className="text-xs text-text-secondary group-hover:text-text-primary transition-colors">
            Create a restore point before migrating{" "}
            <span className="text-text-muted">(recommended)</span>
          </span>
        </label>

        {/* Error */}
        {migrationError && (
          <div className="flex items-start gap-2 game-control px-2.5 py-2 bg-game2/10 border border-game2/30 text-xs text-game2">
            <AlertTriangle size={13} className="shrink-0 mt-0.5" />
            {migrationError}
          </div>
        )}

        {/* CTA */}
        <div className="flex items-center gap-3 pt-0.5">
          <button
            onClick={handleMigrate}
            disabled={migrating}
            className="game-control flex items-center gap-1.5 px-3 py-1.5 text-xs font-semibold bg-game2/15 border border-game2/40 text-game2 hover:bg-game2/25 disabled:opacity-50 transition-colors shadow-glow-game2"
          >
            {migrating ? (
              <>
                <Loader2 size={12} className="animate-spin" />
                Migrating…
              </>
            ) : (
              <>
                <GitMerge size={12} />
                Migrate {layoutStatus.legacy_mod_count} mod{layoutStatus.legacy_mod_count === 1 ? "" : "s"}
              </>
            )}
          </button>
          <span className="text-xs text-text-muted">
            You can keep using the app without migrating — this is opt-in.
          </span>
        </div>
      </div>
    </div>
  );
}
