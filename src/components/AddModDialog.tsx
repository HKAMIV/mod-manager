import { useCallback, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "../lib/invoke";
import {
  FolderOpen,
  FileArchive,
  X,
  Plus,
  Loader2,
  CheckCircle2,
  AlertTriangle,
  ChevronDown,
} from "lucide-react";
import type { InstallResult, ModInfo } from "../types";

interface Props {
  gameId: string;
  /** Existing category names from the current mod list, for the picker. */
  existingCategories: string[];
  onInstalled: (mod: ModInfo) => void;
  onClose: () => void;
}

type Tab = "folder" | "archive";

/** New-category sentinel value in the category <select>. */
const NEW_CATEGORY = "__new__";

/**
 * Modal dialog for manually adding a mod from a local folder or archive.
 * Folder mode copies the selected directory into the game's mod folder.
 * Archive mode extracts .zip / .7z / .rar and places the content.
 * Both respect the active layout (legacy rename vs. Phase 12 symlink).
 */
export default function AddModDialog({
  gameId,
  existingCategories,
  onInstalled,
  onClose,
}: Props) {
  const [tab, setTab] = useState<Tab>("folder");

  // Picked path
  const [pickedPath, setPickedPath] = useState<string | null>(null);

  // Category
  const [categorySelect, setCategorySelect] = useState<string>("");
  const [newCategory, setNewCategory] = useState("");

  // Mod name override (blank = derive from source)
  const [modName, setModName] = useState("");

  // State
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  // Sorted unique categories with "Uncategorized" last
  const categories = [
    ...existingCategories.filter((c) => c !== "Uncategorized").sort(),
    ...(existingCategories.includes("Uncategorized") ? ["Uncategorized"] : []),
  ];

  const resolvedCategory =
    categorySelect === NEW_CATEGORY
      ? newCategory.trim()
      : categorySelect === "Uncategorized"
      ? ""
      : categorySelect;

  const canInstall =
    pickedPath !== null &&
    !installing &&
    // If "New category" is selected, the text field must not be empty
    !(categorySelect === NEW_CATEGORY && newCategory.trim() === "");

  // -------------------------------------------------------------------------
  // File picker
  // -------------------------------------------------------------------------

  const pickFolder = useCallback(async () => {
    const result = await open({ directory: true, multiple: false, title: "Select mod folder" });
    if (typeof result === "string") {
      setPickedPath(result);
      setError(null);
      setSuccess(false);
      // Pre-fill mod name from folder name if blank
      if (modName === "") {
        const parts = result.replace(/\\/g, "/").split("/");
        setModName(parts[parts.length - 1] ?? "");
      }
    }
  }, [modName]);

  const pickArchive = useCallback(async () => {
    const result = await open({
      multiple: false,
      title: "Select mod archive",
      filters: [{ name: "Mod archives", extensions: ["zip", "7z", "rar"] }],
    });
    if (typeof result === "string") {
      setPickedPath(result);
      setError(null);
      setSuccess(false);
      // Pre-fill mod name from archive stem if blank
      if (modName === "") {
        const base = result.replace(/\\/g, "/").split("/").pop() ?? "";
        const stem = base.replace(/\.(zip|7z|rar)$/i, "");
        setModName(stem);
      }
    }
  }, [modName]);

  // -------------------------------------------------------------------------
  // Install
  // -------------------------------------------------------------------------

  const handleInstall = useCallback(async () => {
    if (!pickedPath) return;
    setInstalling(true);
    setError(null);
    try {
      const command =
        tab === "folder" ? "install_mod_from_folder" : "install_mod_from_archive";
      const pathKey = tab === "folder" ? "sourcePath" : "archivePath";

      const result = await invoke<InstallResult>(command, {
        gameId,
        [pathKey]: pickedPath,
        category: resolvedCategory,
        modName: modName.trim(),
      });

      setSuccess(true);
      onInstalled(result.mod_info);

      // Auto-close after a brief success flash
      setTimeout(onClose, 900);
    } catch (err) {
      setError(typeof err === "string" ? err : "Installation failed");
    } finally {
      setInstalling(false);
    }
  }, [pickedPath, tab, gameId, resolvedCategory, modName, onInstalled, onClose]);

  // -------------------------------------------------------------------------
  // Render
  // -------------------------------------------------------------------------

  const pathLabel = pickedPath
    ? pickedPath.replace(/\\/g, "/").split("/").pop()!
    : null;

  return (
    /* Backdrop */
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-surface-0/80 backdrop-blur-sm"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div
        className="game-panel glass-panel border border-surface-3 shadow-glow-game w-full max-w-md mx-4"
        role="dialog"
        aria-modal="true"
        aria-label="Add mod"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 pt-5 pb-4 border-b border-surface-3">
          <div>
            <p className="hud-label text-game2 text-xs">ADD MOD</p>
            <h2 className="font-display text-lg font-semibold text-text-primary mt-0.5">
              Install from disk
            </h2>
            <p className="text-xs text-text-muted mt-0.5">
              Installs into the managed store with a symlink — enabled by default.
            </p>
          </div>
          <button
            onClick={onClose}
            className="game-control p-1.5 text-text-muted hover:text-text-primary hover:bg-surface-2 transition-colors"
            aria-label="Close"
          >
            <X size={16} />
          </button>
        </div>

        {/* Tab switcher */}
        <div className="flex gap-1 px-5 pt-4">
          {(["folder", "archive"] as Tab[]).map((t) => (
            <button
              key={t}
              onClick={() => {
                setTab(t);
                setPickedPath(null);
                setError(null);
                setSuccess(false);
              }}
              className={`game-control flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium border transition-colors ${
                tab === t
                  ? "bg-game/15 border-game/40 text-game"
                  : "bg-surface-1 border-surface-3 text-text-secondary hover:text-text-primary"
              }`}
            >
              {t === "folder" ? <FolderOpen size={13} /> : <FileArchive size={13} />}
              {t === "folder" ? "Folder" : "Archive"}
            </button>
          ))}
        </div>

        {/* Body */}
        <div className="px-5 py-4 space-y-4">
          {/* Path picker */}
          <div>
            <label className="block text-xs text-text-secondary mb-1.5">
              {tab === "folder" ? "Mod folder" : "Archive file (.zip / .7z / .rar)"}
            </label>
            <button
              onClick={tab === "folder" ? pickFolder : pickArchive}
              className="game-control w-full flex items-center gap-2.5 px-3 py-2.5 bg-surface-1 border border-surface-3 hover:border-game/50 hover:shadow-glow-game text-left transition-all"
            >
              {tab === "folder"
                ? <FolderOpen size={15} className="shrink-0 text-game2" />
                : <FileArchive size={15} className="shrink-0 text-game2" />}
              <span className={`text-sm truncate ${pathLabel ? "text-text-primary" : "text-text-muted"}`}>
                {pathLabel ?? (tab === "folder" ? "Click to choose a folder…" : "Click to choose an archive…")}
              </span>
            </button>
          </div>

          {/* Category */}
          <div>
            <label className="block text-xs text-text-secondary mb-1.5">Category</label>
            <div className="relative">
              <select
                value={categorySelect}
                onChange={(e) => { setCategorySelect(e.target.value); setError(null); }}
                className="game-control w-full appearance-none bg-surface-1 border border-surface-3 focus:border-game/50 focus:shadow-glow-game px-3 py-2 text-sm text-text-primary pr-8 transition-all cursor-pointer outline-none"
              >
                <option value="">Uncategorized</option>
                {categories.filter((c) => c !== "Uncategorized").map((cat) => (
                  <option key={cat} value={cat}>{cat}</option>
                ))}
                <option value={NEW_CATEGORY}>+ New category…</option>
              </select>
              <ChevronDown
                size={14}
                className="absolute right-2.5 top-1/2 -translate-y-1/2 text-text-muted pointer-events-none"
              />
            </div>

            {/* New category text input */}
            {categorySelect === NEW_CATEGORY && (
              <div className="mt-2 flex items-center gap-2">
                <Plus size={13} className="shrink-0 text-game2" />
                <input
                  type="text"
                  value={newCategory}
                  onChange={(e) => setNewCategory(e.target.value)}
                  placeholder="e.g. Characters"
                  autoFocus
                  className="game-control flex-1 bg-surface-1 border border-game/40 px-3 py-1.5 text-sm text-text-primary placeholder:text-text-muted focus:outline-none focus:border-game focus:shadow-glow-game transition-all"
                />
              </div>
            )}
          </div>

          {/* Mod name override */}
          <div>
            <label className="block text-xs text-text-secondary mb-1.5">
              Mod name{" "}
              <span className="text-text-muted">(optional — leave blank to use source name)</span>
            </label>
            <input
              type="text"
              value={modName}
              onChange={(e) => setModName(e.target.value)}
              placeholder="e.g. Furina Dreamscape v2"
              className="game-control w-full bg-surface-1 border border-surface-3 focus:border-game/50 focus:shadow-glow-game px-3 py-2 text-sm text-text-primary placeholder:text-text-muted focus:outline-none transition-all"
            />
          </div>

          {/* Error */}
          {error && (
            <div className="flex items-start gap-2 game-control px-3 py-2 bg-game2/10 border border-game2/30 text-xs text-game2">
              <AlertTriangle size={13} className="shrink-0 mt-0.5" />
              <span>{error}</span>
            </div>
          )}

          {/* Success flash */}
          {success && (
            <div className="flex items-center gap-2 game-control px-3 py-2 bg-game/10 border border-game/30 text-xs text-game">
              <CheckCircle2 size={13} className="shrink-0" />
              Installed successfully
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-2 px-5 pb-5">
          <button
            onClick={onClose}
            className="game-control px-4 py-2 text-sm text-text-secondary hover:text-text-primary bg-surface-1 border border-surface-3 hover:border-surface-3/80 transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={handleInstall}
            disabled={!canInstall}
            className="game-control flex items-center gap-2 px-4 py-2 text-sm font-medium bg-game text-surface-0 hover:bg-game-hover disabled:opacity-40 disabled:cursor-not-allowed shadow-glow-game transition-all"
          >
            {installing && <Loader2 size={14} className="animate-spin" />}
            {installing ? "Installing…" : "Install mod"}
          </button>
        </div>
      </div>
    </div>
  );
}
