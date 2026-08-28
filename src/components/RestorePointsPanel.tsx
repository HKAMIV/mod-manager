import { useState } from "react";
import {
  X,
  History,
  Plus,
  Trash2,
  RotateCcw,
  HardDriveDownload,
  Layers,
  AlertTriangle,
  CheckCircle2,
} from "lucide-react";
import { useRestorePoints } from "../hooks/useRestorePoints";
import type { RestorePoint, RestoreResult } from "../types";

interface Props {
  onClose: () => void;
  onRestored: () => void;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatRelativeTime(unixSeconds: number): string {
  const diffMs = Date.now() - unixSeconds * 1000;
  const minutes = Math.floor(diffMs / 60000);
  if (minutes < 1) return "Just now";
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  return new Date(unixSeconds * 1000).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

function RestorePointsPanel({ onClose, onRestored }: Props) {
  const { restorePoints, loading, error, createRestorePoint, deleteRestorePoint, restoreFromPoint } =
    useRestorePoints(onRestored);

  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [withFileBackup, setWithFileBackup] = useState(false);
  const [saving, setSaving] = useState(false);
  const [confirmingId, setConfirmingId] = useState<string | null>(null);
  const [workingId, setWorkingId] = useState<string | null>(null);
  const [lastResult, setLastResult] = useState<{ point: RestorePoint; result: RestoreResult } | null>(
    null
  );

  const handleCreate = async () => {
    const name = newName.trim() || `Restore point`;
    setSaving(true);
    await createRestorePoint(name, withFileBackup);
    setSaving(false);
    setNewName("");
    setWithFileBackup(false);
    setCreating(false);
  };

  const handleRestore = async (point: RestorePoint) => {
    setWorkingId(point.id);
    const result = await restoreFromPoint(point.id);
    setWorkingId(null);
    setConfirmingId(null);
    if (result) {
      setLastResult({ point, result });
    }
  };

  const handleDelete = async (point: RestorePoint) => {
    setWorkingId(point.id);
    await deleteRestorePoint(point.id);
    setWorkingId(null);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-surface-0/70 backdrop-blur-sm p-4">
      <div className="game-panel w-full max-w-lg max-h-[85vh] flex flex-col bg-surface-1 border border-surface-3 shadow-glow-game2 overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-surface-3">
          <div className="flex items-center gap-2">
            <History size={16} className="text-game2" />
            <h2 className="font-display text-base font-semibold text-text-primary tracking-wide">
              Restore Points
            </h2>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded text-text-secondary hover:text-text-primary hover:bg-surface-2 transition-colors"
            aria-label="Close"
          >
            <X size={16} />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-4 space-y-3">
          {/* Last restore result */}
          {lastResult && (
            <div className="game-panel p-3 bg-game/5 border border-game/30 space-y-1.5">
              <div className="flex items-center gap-2">
                <CheckCircle2 size={14} className="text-game" />
                <p className="text-xs font-medium text-text-primary">
                  Restored "{lastResult.point.name}"
                </p>
              </div>
              <p className="text-xs text-text-muted">
                {lastResult.result.enabled_count} enabled · {lastResult.result.disabled_count}{" "}
                disabled
                {lastResult.result.recreated_count > 0 &&
                  ` · ${lastResult.result.recreated_count} recreated from backup`}
              </p>
              {lastResult.result.unrecoverable.length > 0 && (
                <p className="text-xs text-game2 flex items-start gap-1">
                  <AlertTriangle size={12} className="shrink-0 mt-0.5" />
                  <span>
                    {lastResult.result.unrecoverable.length} mod
                    {lastResult.result.unrecoverable.length === 1 ? "" : "s"} couldn't be
                    recovered (no file backup, and no longer on disk):{" "}
                    {lastResult.result.unrecoverable.join(", ")}
                  </span>
                </p>
              )}
            </div>
          )}

          {error && (
            <div className="game-panel p-3 bg-game2/10 border border-game2/30 text-xs text-game2">
              {error}
            </div>
          )}

          {/* Create new */}
          {creating ? (
            <div className="game-panel p-3 bg-surface-2 border border-game/40 space-y-3">
              <input
                autoFocus
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleCreate()}
                placeholder="Name this restore point..."
                className="game-control w-full bg-surface-1 border border-surface-3 px-3 py-2 text-sm text-text-primary placeholder:text-text-muted focus:outline-none focus:border-game/50"
              />
              <label className="flex items-start gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={withFileBackup}
                  onChange={(e) => setWithFileBackup(e.target.checked)}
                  className="w-4 h-4 mt-0.5"
                  style={{ accentColor: "var(--game-accent)" }}
                />
                <span className="text-xs text-text-secondary leading-relaxed">
                  Include full file backup — copies every mod's files so a deleted mod can
                  be recreated on restore, not just re-enabled. Uses more disk space and
                  takes longer.
                </span>
              </label>
              <div className="flex items-center gap-2 justify-end">
                <button
                  onClick={() => setCreating(false)}
                  className="game-control px-3 py-1.5 text-xs font-medium text-text-secondary hover:text-text-primary transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={handleCreate}
                  disabled={saving}
                  className="game-control px-3 py-1.5 text-xs font-medium bg-game text-surface-0 hover:bg-game-hover disabled:opacity-50 transition-colors"
                >
                  {saving ? "Saving..." : "Save Restore Point"}
                </button>
              </div>
            </div>
          ) : (
            <button
              onClick={() => setCreating(true)}
              className="game-control flex items-center justify-center gap-2 w-full py-2.5 text-sm font-medium bg-surface-2 border border-surface-3 hover:border-game/40 text-text-secondary hover:text-game transition-colors"
            >
              <Plus size={14} />
              New Restore Point
            </button>
          )}

          {/* List */}
          {loading && restorePoints.length === 0 ? (
            <p className="text-xs text-text-muted text-center py-6">Loading...</p>
          ) : restorePoints.length === 0 ? (
            <div className="text-center py-8 space-y-2">
              <Layers size={24} className="mx-auto text-text-muted" />
              <p className="text-xs text-text-muted">
                No restore points yet. Create one before making big changes to your mods.
              </p>
            </div>
          ) : (
            <div className="space-y-2">
              {restorePoints.map((point) => (
                <div
                  key={point.id}
                  className="game-panel p-3 bg-surface-1 border border-surface-3 space-y-2"
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                      <p className="text-sm font-medium text-text-primary truncate">
                        {point.name}
                      </p>
                      <p className="text-xs text-text-muted mt-0.5">
                        {formatRelativeTime(point.created_at)} · {point.mods.length} mod
                        {point.mods.length === 1 ? "" : "s"}
                        {point.has_file_backup && (
                          <>
                            {" "}
                            ·{" "}
                            <span className="text-game2 inline-flex items-center gap-0.5">
                              <HardDriveDownload size={10} />
                              {formatSize(point.backup_size_bytes)} backup
                            </span>
                          </>
                        )}
                      </p>
                    </div>
                    <button
                      onClick={() => handleDelete(point)}
                      disabled={workingId === point.id}
                      className="p-1.5 rounded text-text-muted hover:text-game2 hover:bg-game2/10 disabled:opacity-40 transition-colors shrink-0"
                      aria-label={`Delete restore point ${point.name}`}
                    >
                      <Trash2 size={13} />
                    </button>
                  </div>

                  {confirmingId === point.id ? (
                    <div className="game-panel p-2.5 bg-game2/5 border border-game2/30 space-y-2">
                      <p className="text-xs text-text-secondary leading-relaxed">
                        This re-enables/disables mods to match this snapshot
                        {point.has_file_backup ? " and recreates any that were deleted" : ""}.
                        Mods added since won't be touched or removed.
                      </p>
                      <div className="flex items-center gap-2 justify-end">
                        <button
                          onClick={() => setConfirmingId(null)}
                          className="game-control px-2.5 py-1 text-xs font-medium text-text-secondary hover:text-text-primary transition-colors"
                        >
                          Cancel
                        </button>
                        <button
                          onClick={() => handleRestore(point)}
                          disabled={workingId === point.id}
                          className="game-control px-2.5 py-1 text-xs font-medium bg-game2 text-surface-0 hover:bg-game2-hover disabled:opacity-50 transition-colors"
                        >
                          {workingId === point.id ? "Restoring..." : "Confirm Restore"}
                        </button>
                      </div>
                    </div>
                  ) : (
                    <button
                      onClick={() => setConfirmingId(point.id)}
                      className="game-control flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-medium bg-surface-2 border border-surface-3 hover:border-game/40 text-text-secondary hover:text-game transition-colors"
                    >
                      <RotateCcw size={12} />
                      Restore
                    </button>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default RestorePointsPanel;
