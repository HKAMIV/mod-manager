import { useMemo, useState } from "react";
import { Hash, AlertTriangle, RefreshCw, Loader2, Save, Undo2, Info, CheckCircle2 } from "lucide-react";
import type { HashEdit, HashUpdateResult, ModIniFile } from "../../types";
import IniFileSelector from "./IniFileSelector";

interface Props {
  files: ModIniFile[];
  loading: boolean;
  error: string | null;
  onSave: (edits: HashEdit[]) => Promise<HashUpdateResult>;
  onRetry: () => void;
}

/** Stable key for one hash line across files. */
function hashKey(path: string, lineIndex: number): string {
  return `${path}::${lineIndex}`;
}

function withHashes(files: ModIniFile[]): ModIniFile[] {
  return files.filter((f) => f.hashes.length > 0);
}

/**
 * Manual hash updater. When a game updates, asset hashes shift and some mods
 * stop matching. We can't know the correct new hash — but if the user finds it
 * (from an updated mod, a forum, etc.) this lets them paste it in. Edits are
 * staged locally and flushed in one batch on Save, each touched file backed up
 * to `<file>.ini.bak` first.
 */
export default function HashesTab({ files, loading, error, onSave, onRetry }: Props) {
  const [selected, setSelected] = useState<string | null>(null);
  // Staged edits: key -> new value. Only entries that differ from the current
  // on-disk value are meaningful; empty/equal entries are pruned before save.
  const [pending, setPending] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [lastResult, setLastResult] = useState<HashUpdateResult | null>(null);

  const hashFiles = useMemo(() => withHashes(files), [files]);
  const fileNames = useMemo(() => hashFiles.map((f) => f.relative_path), [hashFiles]);

  const visible = useMemo(
    () => (selected ? hashFiles.filter((f) => f.relative_path === selected) : hashFiles),
    [hashFiles, selected]
  );

  // Build the concrete edit list: staged values that actually differ from the
  // current on-disk value and aren't blank.
  const edits = useMemo<HashEdit[]>(() => {
    const out: HashEdit[] = [];
    for (const file of hashFiles) {
      for (const h of file.hashes) {
        const key = hashKey(file.path, h.line_index);
        const staged = pending[key];
        if (staged !== undefined && staged.trim() !== "" && staged.trim() !== h.value) {
          out.push({ ini_path: file.path, line_index: h.line_index, new_value: staged.trim() });
        }
      }
    }
    return out;
  }, [hashFiles, pending]);

  const pendingCount = edits.length;

  const setStaged = (key: string, value: string) => {
    setLastResult(null);
    setSaveError(null);
    setPending((prev) => ({ ...prev, [key]: value }));
  };

  const discard = () => {
    setPending({});
    setSaveError(null);
    setLastResult(null);
  };

  const save = async () => {
    if (pendingCount === 0) return;
    setSaving(true);
    setSaveError(null);
    setLastResult(null);
    try {
      const result = await onSave(edits);
      setLastResult(result);
      if (result.errors.length > 0) {
        setSaveError(result.errors.join("; "));
      }
      // Clear staging — a re-read has happened in the hook, so the displayed
      // current values now reflect what was written.
      setPending({});
    } catch (err) {
      setSaveError(typeof err === "string" ? err : "Failed to save hash edits");
    } finally {
      setSaving(false);
    }
  };

  if (loading) return <TabState icon={<Loader2 size={22} className="text-game animate-spin" />} title="Reading ini files…" />;
  if (error)
    return (
      <TabState
        icon={<AlertTriangle size={22} className="text-game2" />}
        title="Couldn't read ini files"
        description={error}
        action={
          <button
            onClick={onRetry}
            className="game-control flex items-center gap-1.5 px-3 py-1.5 text-xs bg-surface-1 border border-surface-3 hover:border-game/40 text-text-secondary hover:text-game transition-colors"
          >
            <RefreshCw size={13} /> Retry
          </button>
        }
      />
    );
  if (hashFiles.length === 0)
    return (
      <TabState
        icon={<Hash size={22} className="text-text-muted" />}
        title="No hashes found"
        description="This mod's ini files don't contain any hash overrides."
      />
    );

  return (
    <div className="flex flex-col flex-1 min-h-0">
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {/* Honest framing — we can't find the right hash for you */}
        <div className="game-panel flex items-start gap-2 px-3 py-2 bg-surface-2/60 border border-surface-3 text-[11px] text-text-muted leading-relaxed">
          <Info size={13} className="shrink-0 mt-0.5 text-game2" />
          <span>
            Paste a corrected hash you found (from an updated mod or a forum).
            The app can't tell you the right value — a wrong hash simply won't
            match. A <span className="font-mono">.ini.bak</span> backup is saved
            before any change.
          </span>
        </div>

        <IniFileSelector fileNames={fileNames} selected={selected} onSelect={setSelected} />

        {visible.map((file) => (
          <div key={file.path} className="space-y-2">
            {visible.length > 1 && (
              <p className="hud-label text-[10px] text-game2 font-mono break-all">{file.relative_path}</p>
            )}
            <div className="space-y-2">
              {file.hashes.map((h) => {
                const key = hashKey(file.path, h.line_index);
                const staged = pending[key];
                const isDirty =
                  staged !== undefined && staged.trim() !== "" && staged.trim() !== h.value;
                return (
                  <div key={key} className="game-panel bg-surface-1 border border-surface-3 p-2.5 space-y-1.5">
                    <p className="text-[10px] text-text-muted font-mono truncate">{h.section}</p>
                    <div className="flex items-center gap-2">
                      <span className="text-[10px] uppercase tracking-wide text-text-muted shrink-0">hash</span>
                      <input
                        type="text"
                        value={staged ?? h.value}
                        onChange={(e) => setStaged(key, e.target.value)}
                        spellCheck={false}
                        className={`game-control flex-1 min-w-0 bg-surface-2 border px-2 py-1 text-xs font-mono text-text-primary focus:outline-none transition-colors ${
                          isDirty
                            ? "border-game/50 shadow-glow-game"
                            : "border-surface-3 focus:border-game/40"
                        }`}
                      />
                    </div>
                    {isDirty && (
                      <p className="text-[10px] text-game">
                        was <span className="font-mono">{h.value}</span>
                      </p>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        ))}

        {/* Save result feedback */}
        {lastResult && lastResult.errors.length === 0 && (
          <div className="flex items-center gap-2 game-control px-3 py-2 bg-game/10 border border-game/30 text-xs text-game">
            <CheckCircle2 size={13} className="shrink-0" />
            Saved {lastResult.hashes_changed} hash{lastResult.hashes_changed === 1 ? "" : "es"} across{" "}
            {lastResult.files_written} file{lastResult.files_written === 1 ? "" : "s"}
          </div>
        )}
        {saveError && (
          <div className="flex items-start gap-2 game-control px-3 py-2 bg-game2/10 border border-game2/30 text-xs text-game2">
            <AlertTriangle size={13} className="shrink-0 mt-0.5" />
            <span>{saveError}</span>
          </div>
        )}
      </div>

      {/* Sticky action bar — only when there are staged edits */}
      {pendingCount > 0 && (
        <div className="flex items-center justify-between gap-2 p-3 border-t border-surface-3 bg-surface-1/95">
          <span className="text-xs text-text-secondary">
            {pendingCount} pending edit{pendingCount === 1 ? "" : "s"}
          </span>
          <div className="flex items-center gap-2">
            <button
              onClick={discard}
              disabled={saving}
              className="game-control flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-text-secondary hover:text-text-primary bg-surface-2 border border-surface-3 disabled:opacity-50 transition-colors"
            >
              <Undo2 size={12} /> Discard
            </button>
            <button
              onClick={save}
              disabled={saving}
              className="game-control flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium bg-game text-surface-0 hover:bg-game-hover disabled:opacity-50 shadow-glow-game transition-colors"
            >
              {saving ? <Loader2 size={12} className="animate-spin" /> : <Save size={12} />}
              {saving ? "Saving…" : `Save ${pendingCount}`}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

function TabState({
  icon,
  title,
  description,
  action,
}: {
  icon: React.ReactNode;
  title: string;
  description?: string;
  action?: React.ReactNode;
}) {
  return (
    <div className="flex-1 flex items-center justify-center p-6">
      <div className="text-center space-y-2.5 max-w-[15rem]">
        <div className="mx-auto w-11 h-11 flex items-center justify-center">{icon}</div>
        <p className="text-sm font-medium text-text-secondary">{title}</p>
        {description && <p className="text-xs text-text-muted leading-relaxed">{description}</p>}
        {action && <div className="pt-1 flex justify-center">{action}</div>}
      </div>
    </div>
  );
}
