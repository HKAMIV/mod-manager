import { useMemo, useState } from "react";
import { Keyboard, AlertTriangle, RefreshCw, Loader2 } from "lucide-react";
import type { IniKeybind, ModIniFile } from "../../types";
import IniFileSelector from "./IniFileSelector";

interface Props {
  files: ModIniFile[];
  loading: boolean;
  error: string | null;
  onRetry: () => void;
}

/** Files that actually contain keybinds. */
function withKeybinds(files: ModIniFile[]): ModIniFile[] {
  return files.filter((f) => f.keybinds.length > 0);
}

/**
 * Read-only display of a mod's keybindings, grouped by ini file. 3DMigoto
 * `[Key*]`/`[KeySwap*]` sections don't ship an in-game menu for most mods, so
 * this surfaces the key, its action type (cycle/toggle/hold), and any variant
 * variable it drives.
 */
export default function KeybindsTab({ files, loading, error, onRetry }: Props) {
  const [selected, setSelected] = useState<string | null>(null);

  const keybindFiles = useMemo(() => withKeybinds(files), [files]);
  const fileNames = useMemo(() => keybindFiles.map((f) => f.relative_path), [keybindFiles]);

  const visible = useMemo(
    () => (selected ? keybindFiles.filter((f) => f.relative_path === selected) : keybindFiles),
    [keybindFiles, selected]
  );

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
  if (keybindFiles.length === 0)
    return (
      <TabState
        icon={<Keyboard size={22} className="text-text-muted" />}
        title="No keybinds found"
        description="This mod's ini files don't declare any key bindings."
      />
    );

  return (
    <div className="flex-1 overflow-y-auto p-4 space-y-4">
      <IniFileSelector fileNames={fileNames} selected={selected} onSelect={setSelected} />

      {visible.map((file) => (
        <div key={file.path} className="space-y-2">
          {/* File header — shown when viewing all files, so groups are clear */}
          {visible.length > 1 && (
            <p className="hud-label text-[10px] text-game2 font-mono break-all">{file.relative_path}</p>
          )}
          <div className="space-y-2">
            {file.keybinds.map((kb, i) => (
              <KeybindRow key={`${file.path}-${i}`} kb={kb} />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

function typeVerb(bindType: string | null): string {
  switch ((bindType ?? "").toLowerCase()) {
    case "cycle":
      return "Cycle";
    case "toggle":
      return "Toggle";
    case "hold":
      return "Hold";
    default:
      return "Key";
  }
}

function KeybindRow({ kb }: { kb: IniKeybind }) {
  return (
    <div className="game-panel bg-surface-1 border border-surface-3 p-2.5 space-y-1.5">
      {/* Section label */}
      <p className="text-[10px] text-text-muted font-mono truncate">{kb.section}</p>

      {/* Primary key */}
      <div className="flex items-center gap-2 flex-wrap">
        <span className="text-[10px] uppercase tracking-wide text-game2">{typeVerb(kb.bind_type)}</span>
        {kb.key_label ? (
          <KeyCap label={kb.key_label} />
        ) : (
          <span className="text-xs text-text-muted italic">no key set</span>
        )}
        {kb.back_label && (
          <>
            <span className="text-[10px] text-text-muted">/ back</span>
            <KeyCap label={kb.back_label} muted />
          </>
        )}
      </div>

      {/* Driven variable + its value list, when present */}
      {kb.variable && (
        <p className="text-[11px] text-text-secondary">
          <span className="font-mono text-game">{kb.variable}</span>
          {kb.values && <span className="text-text-muted"> = {kb.values}</span>}
        </p>
      )}
    </div>
  );
}

function KeyCap({ label, muted = false }: { label: string; muted?: boolean }) {
  return (
    <kbd
      className={`game-control inline-flex items-center px-1.5 py-0.5 text-xs font-medium border ${
        muted
          ? "bg-surface-2 border-surface-3 text-text-secondary"
          : "bg-game/10 border-game/30 text-game"
      }`}
    >
      {label}
    </kbd>
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
