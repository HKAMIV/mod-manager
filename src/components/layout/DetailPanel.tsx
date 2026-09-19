import { useMemo, useState } from "react";
import { useAppStore } from "../../stores/appStore";
import { useModInis } from "../../hooks/useModInis";
import { X, ImageOff, Folder, FileStack, HardDrive, Clock, Keyboard, Hash } from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";
import KeybindsTab from "../ini/KeybindsTab";
import HashesTab from "../ini/HashesTab";

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatDate(unixSeconds: number | null): string {
  if (!unixSeconds) return "Unknown";
  return new Date(unixSeconds * 1000).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

type Tab = "details" | "keybinds" | "hashes";

function DetailPanel() {
  const { toggleDetailPanel, selectedMod } = useAppStore();
  const [tab, setTab] = useState<Tab>("details");

  const previewSrc = selectedMod?.preview_path
    ? convertFileSrc(selectedMod.preview_path)
    : null;

  // Load the mod's ini files (keybinds + hashes). Fetches on mod change,
  // cached per path. Passing null when no mod is selected clears it.
  const { files, loading, error, refresh, saveHashEdits } = useModInis(
    selectedMod?.path ?? null
  );

  const keybindCount = useMemo(
    () => files.reduce((n, f) => n + f.keybinds.length, 0),
    [files]
  );
  const hashCount = useMemo(
    () => files.reduce((n, f) => n + f.hashes.length, 0),
    [files]
  );

  return (
    <aside className="relative w-80 h-full bg-surface-1/90 backdrop-blur-xl flex flex-col">
      <div className="absolute left-0 top-0 bottom-0 w-px bg-gradient-to-b from-transparent via-game/60 to-transparent" />

      {/* Header */}
      <div className="flex flex-col gap-3 p-4 border-b border-surface-3">
        <div className="flex items-center justify-between">
          <span className="hud-label text-xs text-text-secondary">Mod Details</span>
          <button
            onClick={toggleDetailPanel}
            className="p-1 rounded hover:bg-surface-2 text-text-secondary hover:text-text-primary transition-colors"
            aria-label="Close detail panel"
          >
            <X size={16} />
          </button>
        </div>

        {/* Tabs — only shown when a mod is selected */}
        {selectedMod && (
          <div className="flex items-center gap-1">
            <TabButton
              active={tab === "details"}
              onClick={() => setTab("details")}
              label="Details"
            />
            <TabButton
              active={tab === "keybinds"}
              onClick={() => setTab("keybinds")}
              icon={<Keyboard size={12} />}
              label="Keybinds"
              count={keybindCount}
            />
            <TabButton
              active={tab === "hashes"}
              onClick={() => setTab("hashes")}
              icon={<Hash size={12} />}
              label="Hashes"
              count={hashCount}
            />
          </div>
        )}
      </div>

      {!selectedMod ? (
        <div className="flex-1 flex items-center justify-center p-6">
          <div className="text-center space-y-2">
            <div className="game-panel mx-auto w-10 h-10 bg-game/10 border border-game/30 flex items-center justify-center shadow-glow-game">
              <span className="w-1.5 h-1.5 rounded-full bg-game animate-pulse-glow" />
            </div>
            <p className="text-sm text-text-muted">Select a mod to view details</p>
          </div>
        </div>
      ) : tab === "details" ? (
        <div className="flex-1 overflow-y-auto">
          {/* Preview */}
          <div className="aspect-video bg-surface-2 flex items-center justify-center overflow-hidden">
            {previewSrc ? (
              <img
                src={previewSrc}
                alt={selectedMod.name}
                className="w-full h-full object-cover"
              />
            ) : (
              <ImageOff size={28} className="text-text-muted" />
            )}
          </div>

          <div className="p-4 space-y-4">
            <div className="space-y-1">
              <p className="font-display text-lg font-semibold text-text-primary leading-tight">
                {selectedMod.name}
              </p>
              <span
                className={`inline-flex items-center gap-1.5 mt-1 px-2 py-0.5 rounded-full text-[11px] font-medium ${
                  selectedMod.enabled
                    ? "bg-game/15 text-game border border-game/30"
                    : "bg-surface-2 text-text-muted border border-surface-3"
                }`}
              >
                <span
                  className={`w-1 h-1 rounded-full ${
                    selectedMod.enabled ? "bg-game shadow-glow-game" : "bg-text-muted"
                  }`}
                />
                {selectedMod.enabled ? "Enabled" : "Disabled"}
              </span>
            </div>

            <div className="hud-rule" />

            <dl className="space-y-2.5 text-sm">
              <DetailRow icon={<Folder size={13} />} label="Category" value={selectedMod.category} />
              <DetailRow icon={<FileStack size={13} />} label="Files" value={String(selectedMod.file_count)} />
              <DetailRow icon={<HardDrive size={13} />} label="Size" value={formatSize(selectedMod.size_bytes)} />
              <DetailRow
                icon={<Clock size={13} />}
                label="Modified"
                value={formatDate(selectedMod.modified_at)}
              />
            </dl>

            <div className="hud-rule" />

            <div>
              <p className="hud-label text-[10px] text-text-muted mb-1">Path</p>
              <p className="text-xs text-text-secondary font-mono break-all leading-relaxed">
                {selectedMod.path}
              </p>
            </div>
          </div>
        </div>
      ) : tab === "keybinds" ? (
        <KeybindsTab files={files} loading={loading} error={error} onRetry={refresh} />
      ) : (
        <HashesTab files={files} loading={loading} error={error} onSave={saveHashEdits} onRetry={refresh} />
      )}
    </aside>
  );
}

function TabButton({
  active,
  onClick,
  label,
  icon,
  count,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
  icon?: React.ReactNode;
  count?: number;
}) {
  return (
    <button
      onClick={onClick}
      className={`game-control flex items-center gap-1 px-2 py-1 text-[11px] font-medium border transition-colors ${
        active
          ? "bg-game/15 border-game/40 text-game"
          : "bg-surface-1 border-surface-3 text-text-secondary hover:text-text-primary"
      }`}
    >
      {icon}
      {label}
      {count !== undefined && count > 0 && (
        <span
          className={`ml-0.5 px-1 rounded-full text-[9px] leading-tight ${
            active ? "bg-game/25 text-game" : "bg-surface-2 text-text-muted"
          }`}
        >
          {count}
        </span>
      )}
    </button>
  );
}

function DetailRow({
  icon,
  label,
  value,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div className="flex items-center justify-between">
      <span className="flex items-center gap-1.5 text-text-muted">
        {icon}
        {label}
      </span>
      <span className="text-text-primary font-medium">{value}</span>
    </div>
  );
}

export default DetailPanel;
