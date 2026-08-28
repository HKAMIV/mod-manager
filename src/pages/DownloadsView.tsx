import { useState } from "react";
import {
  Download,
  Loader2,
  Archive,
  FolderCheck,
  XCircle,
  Ban,
  X,
  RotateCcw,
  Trash2,
  AlertTriangle,
  FolderOpen,
  Copy,
} from "lucide-react";
import { useDownloads } from "../hooks/useDownloads";
import type { DownloadItem } from "../types";

function formatSize(bytes: number): string {
  if (bytes <= 0) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatRate(bytesPerSecond: number): string {
  return `${formatSize(bytesPerSecond)}/s`;
}

function formatEta(bytesPerSecond: number, remainingBytes: number): string | null {
  if (bytesPerSecond <= 0 || remainingBytes <= 0) return null;
  const seconds = Math.ceil(remainingBytes / bytesPerSecond);
  if (seconds < 60) return `${seconds}s left`;
  const minutes = Math.ceil(seconds / 60);
  if (minutes < 60) return `${minutes}m left`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ${minutes % 60}m left`;
}

/** Visual treatment for each status: icon, label, and whether it counts as
 * "active" for progress-bar purposes. */
function statusMeta(item: DownloadItem) {
  switch (item.status.state) {
    case "queued":
      return { label: "Queued", icon: Loader2, spin: false, tone: "muted" as const };
    case "downloading":
      return { label: "Downloading", icon: Download, spin: false, tone: "game" as const };
    case "extracting":
      return { label: "Extracting", icon: Archive, spin: true, tone: "game" as const };
    case "installing":
      return { label: "Installing", icon: Archive, spin: true, tone: "game" as const };
    case "waiting_for_conflict":
      return { label: "Needs your input", icon: AlertTriangle, spin: false, tone: "game2" as const };
    case "completed":
      return { label: "Installed", icon: FolderCheck, spin: false, tone: "game" as const };
    case "failed":
      return { label: "Failed", icon: XCircle, spin: false, tone: "game2" as const };
    case "cancelled":
      return { label: "Cancelled", icon: Ban, spin: false, tone: "muted" as const };
  }
}

const TONE_TEXT: Record<string, string> = {
  game: "text-game",
  game2: "text-game2",
  muted: "text-text-muted",
};

interface RowProps {
  item: DownloadItem;
  rateFor: (id: string) => number | null;
  onCancel: (id: string) => void;
  onRetry: (id: string) => void;
  onRemove: (id: string) => void;
  onResolve: (id: string, action: "overwrite" | "rename" | "cancel") => void;
}

function DownloadRow({ item, rateFor, onCancel, onRetry, onRemove, onResolve }: RowProps) {
  const meta = statusMeta(item);
  const Icon = meta.icon;
  const isActive = item.status.state === "downloading" || item.status.state === "extracting" || item.status.state === "installing";
  const progressPct =
    item.total_bytes > 0 ? Math.min(100, (item.downloaded_bytes / item.total_bytes) * 100) : 0;
  const rate = item.status.state === "downloading" ? rateFor(item.id) : null;
  const eta = rate ? formatEta(rate, item.total_bytes - item.downloaded_bytes) : null;

  return (
    <div className="game-panel bg-surface-1 border border-surface-3 p-3.5 space-y-2.5">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <p className="text-sm font-medium text-text-primary truncate">{item.mod_name}</p>
          <p className="text-xs text-text-muted truncate mt-0.5">
            {item.file_name}
            {item.category_hint && ` · ${item.category_hint}`}
          </p>
        </div>
        <div className={`flex items-center gap-1.5 shrink-0 text-xs font-medium ${TONE_TEXT[meta.tone]}`}>
          <Icon size={13} className={meta.spin ? "animate-spin" : ""} />
          {meta.label}
        </div>
      </div>

      {/* Progress bar — shown for anything actively transferring/installing,
          or that's queued (idle, 0%) so its row doesn't jump around once it
          starts. */}
      {(isActive || item.status.state === "queued") && (
        <div className="space-y-1">
          <div className="h-1.5 w-full bg-surface-3 rounded-full overflow-hidden">
            <div
              className="h-full bg-game transition-[width] duration-300 rounded-full"
              style={{ width: `${item.status.state === "queued" ? 0 : progressPct}%` }}
            />
          </div>
          <div className="flex items-center justify-between text-[10px] text-text-muted">
            <span>
              {item.status.state === "downloading"
                ? `${formatSize(item.downloaded_bytes)} / ${formatSize(item.total_bytes)}`
                : meta.label}
            </span>
            {rate && (
              <span>
                {formatRate(rate)}
                {eta && ` · ${eta}`}
              </span>
            )}
          </div>
        </div>
      )}

      {/* Conflict resolution prompt */}
      {item.status.state === "waiting_for_conflict" && (
        <div className="game-panel p-2.5 bg-game2/5 border border-game2/30 space-y-2">
          <p className="text-xs text-text-secondary leading-relaxed">
            A folder already exists at this mod's install location. Choose how to proceed:
          </p>
          <p className="text-[10px] text-text-muted font-mono truncate">
            {item.status.existing_path}
          </p>
          <div className="flex items-center gap-2 flex-wrap">
            <button
              onClick={() => onResolve(item.id, "overwrite")}
              className="game-control flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-medium bg-game2 text-surface-0 hover:bg-game2-hover transition-colors"
            >
              <FolderOpen size={12} />
              Overwrite
            </button>
            <button
              onClick={() => onResolve(item.id, "rename")}
              className="game-control flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-medium bg-surface-2 border border-surface-3 hover:border-game/40 text-text-secondary hover:text-game transition-colors"
            >
              <Copy size={12} />
              Install Alongside
            </button>
            <button
              onClick={() => onResolve(item.id, "cancel")}
              className="game-control px-2.5 py-1.5 text-xs font-medium text-text-muted hover:text-text-primary transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Failure message */}
      {item.status.state === "failed" && (
        <p className="text-xs text-game2 leading-relaxed">{item.status.message}</p>
      )}

      {/* Completed path */}
      {item.status.state === "completed" && (
        <p className="text-[10px] text-text-muted font-mono truncate">
          {item.status.installed_path}
        </p>
      )}

      {/* Row actions */}
      <div className="flex items-center gap-2 justify-end">
        {isActive && (
          <button
            onClick={() => onCancel(item.id)}
            className="game-control flex items-center gap-1 px-2.5 py-1 text-xs font-medium text-text-secondary hover:text-game2 transition-colors"
          >
            <X size={12} />
            Cancel
          </button>
        )}
        {(item.status.state === "failed" || item.status.state === "cancelled") && (
          <button
            onClick={() => onRetry(item.id)}
            className="game-control flex items-center gap-1 px-2.5 py-1 text-xs font-medium text-text-secondary hover:text-game transition-colors"
          >
            <RotateCcw size={12} />
            Retry
          </button>
        )}
        {!isActive && item.status.state !== "waiting_for_conflict" && (
          <button
            onClick={() => onRemove(item.id)}
            className="game-control flex items-center gap-1 px-2.5 py-1 text-xs font-medium text-text-muted hover:text-game2 transition-colors"
          >
            <Trash2 size={12} />
            Remove
          </button>
        )}
      </div>
    </div>
  );
}

function DownloadsView() {
  const { downloads, loading, error, cancel, retry, remove, resolveConflict, rateFor } =
    useDownloads();
  const [clearingHistory, setClearingHistory] = useState(false);

  const active = downloads.filter((d) =>
    ["queued", "downloading", "extracting", "installing", "waiting_for_conflict"].includes(
      d.status.state
    )
  );
  const history = downloads.filter((d) => !active.includes(d));

  const handleClearHistory = async () => {
    setClearingHistory(true);
    await Promise.all(history.map((d) => remove(d.id)));
    setClearingHistory(false);
  };

  if (!loading && downloads.length === 0) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center p-8">
        <div className="text-center space-y-4">
          <div className="game-panel mx-auto w-16 h-16 bg-game/10 border border-game/30 shadow-glow-game flex items-center justify-center">
            <Download size={28} className="text-game" />
          </div>
          <div className="space-y-1.5">
            <p className="font-display text-lg font-medium text-text-secondary">
              Download Manager
            </p>
            <p className="text-sm text-text-muted max-w-xs">
              Downloads you start from Online Mods will show up here, with automatic
              extraction and install.
            </p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="px-5 pt-5 pb-1">
        <h1 className="font-display text-2xl font-semibold text-text-primary tracking-wide">
          Downloads
        </h1>
        <p className="text-xs text-text-muted mt-0.5">
          {active.length > 0
            ? `${active.length} active · ${history.length} in history`
            : `${history.length} in history`}
        </p>
      </div>

      {error && (
        <div className="game-panel mx-5 mt-3 p-3 bg-game2/10 border border-game2/30 text-xs text-game2">
          {error}
        </div>
      )}

      <div className="flex-1 overflow-y-auto px-5 py-4 space-y-5">
        {active.length > 0 && (
          <section className="space-y-2">
            <p className="hud-label text-[10px] text-game2">In Progress</p>
            <div className="hud-rule" />
            <div className="space-y-2 pt-1">
              {active.map((item) => (
                <DownloadRow
                  key={item.id}
                  item={item}
                  rateFor={rateFor}
                  onCancel={cancel}
                  onRetry={retry}
                  onRemove={remove}
                  onResolve={resolveConflict}
                />
              ))}
            </div>
          </section>
        )}

        {history.length > 0 && (
          <section className="space-y-2">
            <div className="flex items-center justify-between">
              <p className="hud-label text-[10px] text-game2">History</p>
              <button
                onClick={handleClearHistory}
                disabled={clearingHistory}
                className="text-[10px] text-text-muted hover:text-game2 disabled:opacity-40 transition-colors"
              >
                Clear history
              </button>
            </div>
            <div className="hud-rule" />
            <div className="space-y-2 pt-1">
              {history.map((item) => (
                <DownloadRow
                  key={item.id}
                  item={item}
                  rateFor={rateFor}
                  onCancel={cancel}
                  onRetry={retry}
                  onRemove={remove}
                  onResolve={resolveConflict}
                />
              ))}
            </div>
          </section>
        )}
      </div>
    </div>
  );
}

export default DownloadsView;
