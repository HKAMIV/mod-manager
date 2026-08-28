import { useState } from "react";
import {
  BookmarkPlus,
  Keyboard,
  MoreVertical,
  Pencil,
  Trash2,
  X,
  Play,
} from "lucide-react";
import { usePresets } from "../hooks/usePresets";
import { eventToAccelerator, formatAccelerator } from "../lib/hotkeyRecorder";
import type { Preset } from "../types";

interface Props {
  onApplied: () => void;
}

function PresetBar({ onApplied }: Props) {
  const {
    presets,
    savePreset,
    deletePreset,
    renamePreset,
    applyPreset,
    setHotkey,
    clearHotkey,
  } = usePresets(onApplied);

  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [menuOpenId, setMenuOpenId] = useState<string | null>(null);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [recordingId, setRecordingId] = useState<string | null>(null);
  const [applyingId, setApplyingId] = useState<string | null>(null);

  const handleCreate = async () => {
    const name = newName.trim();
    if (!name) return;
    await savePreset(name);
    setNewName("");
    setCreating(false);
  };

  const handleApply = async (preset: Preset) => {
    setApplyingId(preset.id);
    await applyPreset(preset.id);
    setApplyingId(null);
  };

  const handleRenameSubmit = async (preset: Preset) => {
    const name = renameValue.trim();
    if (name && name !== preset.name) {
      await renamePreset(preset.id, name);
    }
    setRenamingId(null);
  };

  const handleRecordKey = (preset: Preset) => (e: React.KeyboardEvent) => {
    e.preventDefault();
    if (e.key === "Escape") {
      setRecordingId(null);
      return;
    }
    const accelerator = eventToAccelerator(e.nativeEvent);
    if (accelerator) {
      setHotkey(preset.id, accelerator);
      setRecordingId(null);
    }
  };

  return (
    <div className="flex flex-wrap items-center gap-2 px-5 pb-3">
      {presets.map((preset) => (
        <div
          key={preset.id}
          className="game-panel group relative flex items-center gap-1.5 bg-surface-1 border border-surface-3 hover:border-game/40 pl-1 pr-1.5 py-1 transition-colors"
        >
          {renamingId === preset.id ? (
            <input
              autoFocus
              value={renameValue}
              onChange={(e) => setRenameValue(e.target.value)}
              onBlur={() => handleRenameSubmit(preset)}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleRenameSubmit(preset);
                if (e.key === "Escape") setRenamingId(null);
              }}
              className="game-control bg-surface-2 text-xs text-text-primary px-2 py-1 w-28 focus:outline-none"
            />
          ) : (
            <button
              onClick={() => handleApply(preset)}
              disabled={applyingId === preset.id}
              className="flex items-center gap-1.5 px-2 py-1 text-xs font-medium text-text-secondary hover:text-game transition-colors disabled:opacity-50"
              title={`Apply preset "${preset.name}"`}
            >
              <Play size={11} className={applyingId === preset.id ? "animate-pulse" : ""} />
              {preset.name}
              {preset.hotkey && (
                <span className="hud-label text-[9px] text-game2 bg-game2/10 px-1 py-0.5 rounded">
                  {formatAccelerator(preset.hotkey)}
                </span>
              )}
            </button>
          )}

          <div className="relative">
            <button
              onClick={() => setMenuOpenId(menuOpenId === preset.id ? null : preset.id)}
              className="p-1 rounded text-text-muted hover:text-text-primary hover:bg-surface-2 transition-colors"
              aria-label={`Manage preset ${preset.name}`}
            >
              <MoreVertical size={13} />
            </button>

            {menuOpenId === preset.id && (
              <div className="game-panel absolute z-50 top-full right-0 mt-1 w-44 bg-surface-2 border border-surface-3 shadow-2xl overflow-hidden">
                <button
                  onClick={() => {
                    setRenamingId(preset.id);
                    setRenameValue(preset.name);
                    setMenuOpenId(null);
                  }}
                  className="flex items-center gap-2 w-full text-left px-3 py-2 text-xs text-text-secondary hover:bg-surface-3 hover:text-text-primary transition-colors"
                >
                  <Pencil size={12} />
                  Rename
                </button>
                <button
                  onClick={() => {
                    setRecordingId(preset.id);
                    setMenuOpenId(null);
                  }}
                  className="flex items-center gap-2 w-full text-left px-3 py-2 text-xs text-text-secondary hover:bg-surface-3 hover:text-text-primary transition-colors"
                >
                  <Keyboard size={12} />
                  {preset.hotkey ? "Change hotkey" : "Set hotkey"}
                </button>
                {preset.hotkey && (
                  <button
                    onClick={() => {
                      clearHotkey(preset.id);
                      setMenuOpenId(null);
                    }}
                    className="flex items-center gap-2 w-full text-left px-3 py-2 text-xs text-text-secondary hover:bg-surface-3 hover:text-text-primary transition-colors"
                  >
                    <X size={12} />
                    Clear hotkey
                  </button>
                )}
                <button
                  onClick={() => {
                    deletePreset(preset.id);
                    setMenuOpenId(null);
                  }}
                  className="flex items-center gap-2 w-full text-left px-3 py-2 text-xs text-game2 hover:bg-game2/10 transition-colors"
                >
                  <Trash2 size={12} />
                  Delete
                </button>
              </div>
            )}
          </div>

          {recordingId === preset.id && (
            <div
              className="game-panel absolute z-50 top-full left-0 mt-1 px-3 py-2 bg-surface-2 border border-game shadow-glow-game text-xs text-game whitespace-nowrap"
              tabIndex={0}
              autoFocus
              onKeyDown={handleRecordKey(preset)}
              onBlur={() => setRecordingId(null)}
            >
              Press a key combo... (Esc to cancel)
            </div>
          )}
        </div>
      ))}

      {creating ? (
        <div className="game-control flex items-center gap-1 bg-surface-1 border border-game/40 px-1">
          <input
            autoFocus
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleCreate();
              if (e.key === "Escape") setCreating(false);
            }}
            placeholder="Preset name..."
            className="bg-transparent text-xs text-text-primary placeholder:text-text-muted px-2 py-1.5 w-32 focus:outline-none"
          />
          <button
            onClick={handleCreate}
            className="p-1 text-game hover:text-game/80"
            aria-label="Save preset"
          >
            <BookmarkPlus size={14} />
          </button>
          <button
            onClick={() => setCreating(false)}
            className="p-1 text-text-muted hover:text-text-primary"
            aria-label="Cancel"
          >
            <X size={14} />
          </button>
        </div>
      ) : (
        <button
          onClick={() => setCreating(true)}
          className="game-control flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-medium bg-surface-1 border border-surface-3 hover:border-game/40 text-text-secondary hover:text-game transition-colors"
        >
          <BookmarkPlus size={13} />
          Save preset
        </button>
      )}
    </div>
  );
}

export default PresetBar;
