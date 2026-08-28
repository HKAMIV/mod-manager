import { useState } from "react";
import {
  X,
  ExternalLink,
  Download,
  ImageOff,
  ChevronLeft,
  ChevronRight,
  ShieldCheck,
  ShieldAlert,
  ShieldQuestion,
} from "lucide-react";
import { useGameBananaModDetail } from "../hooks/useGameBananaModDetail";
import { open as openExternal } from "@tauri-apps/plugin-shell";
import type { GbModFile } from "../types";

interface Props {
  modId: number;
  onClose: () => void;
  onDownload?: (
    file: GbModFile,
    modName: string,
    modId: number,
    categoryHint: string | null,
    previewImageUrl: string | null
  ) => void;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function AvBadge({ result }: { result: string | null }) {
  if (result === "clean") {
    return (
      <span className="flex items-center gap-1 text-[10px] text-game">
        <ShieldCheck size={11} />
        Clean
      </span>
    );
  }
  if (result === null) {
    return (
      <span className="flex items-center gap-1 text-[10px] text-text-muted">
        <ShieldQuestion size={11} />
        Unscanned
      </span>
    );
  }
  return (
    <span className="flex items-center gap-1 text-[10px] text-game2">
      <ShieldAlert size={11} />
      {result}
    </span>
  );
}

function GbModDetailPanel({ modId, onClose, onDownload }: Props) {
  const { detail, loading, error } = useGameBananaModDetail(modId);
  const [imageIndex, setImageIndex] = useState(0);
  const [imageError, setImageError] = useState(false);

  const images = detail?.images ?? [];
  const currentImage = images[imageIndex];

  const goToImage = (delta: number) => {
    setImageError(false);
    setImageIndex((i) => (i + delta + images.length) % images.length);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-surface-0/70 backdrop-blur-sm p-4">
      <div className="game-panel w-full max-w-2xl max-h-[85vh] flex flex-col bg-surface-1 border border-surface-3 shadow-glow-game2 overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-surface-3 shrink-0">
          <h2 className="font-display text-base font-semibold text-text-primary tracking-wide truncate pr-4">
            {detail?.name ?? "Loading..."}
          </h2>
          <button
            onClick={onClose}
            className="p-1 rounded text-text-secondary hover:text-text-primary hover:bg-surface-2 transition-colors shrink-0"
            aria-label="Close"
          >
            <X size={16} />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto">
          {loading ? (
            <p className="text-xs text-text-muted text-center py-10">Loading...</p>
          ) : error ? (
            <p className="text-xs text-game2 text-center py-10">{error}</p>
          ) : detail ? (
            <div className="space-y-4">
              {/* Image gallery */}
              {images.length > 0 && (
                <div className="relative aspect-video bg-surface-2 flex items-center justify-center overflow-hidden">
                  {!imageError ? (
                    <img
                      src={currentImage}
                      alt={detail.name}
                      onError={() => setImageError(true)}
                      className="w-full h-full object-contain"
                    />
                  ) : (
                    <ImageOff size={28} className="text-text-muted" />
                  )}
                  {images.length > 1 && (
                    <>
                      <button
                        onClick={() => goToImage(-1)}
                        className="absolute left-2 top-1/2 -translate-y-1/2 p-1.5 rounded-full bg-surface-0/60 text-text-primary hover:bg-surface-0/90 transition-colors"
                        aria-label="Previous image"
                      >
                        <ChevronLeft size={16} />
                      </button>
                      <button
                        onClick={() => goToImage(1)}
                        className="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 rounded-full bg-surface-0/60 text-text-primary hover:bg-surface-0/90 transition-colors"
                        aria-label="Next image"
                      >
                        <ChevronRight size={16} />
                      </button>
                      <span className="absolute bottom-2 right-2 px-1.5 py-0.5 rounded-full text-[10px] bg-surface-0/70 text-text-secondary">
                        {imageIndex + 1} / {images.length}
                      </span>
                    </>
                  )}
                </div>
              )}

              <div className="px-4 pb-4 space-y-4">
                {/* Submitter + external link */}
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    {detail.submitter_avatar_url && (
                      <img
                        src={detail.submitter_avatar_url}
                        alt=""
                        className="w-6 h-6 rounded-full object-cover"
                      />
                    )}
                    <span className="text-xs text-text-secondary">
                      by <span className="text-text-primary font-medium">{detail.submitter_name ?? "Unknown"}</span>
                    </span>
                  </div>
                  <button
                    onClick={() => openExternal(detail.profile_url)}
                    className="flex items-center gap-1 text-xs text-game hover:text-game-hover transition-colors"
                  >
                    View on GameBanana
                    <ExternalLink size={11} />
                  </button>
                </div>

                <div className="hud-rule" />

                {/* Description */}
                {detail.description_html && (
                  <div
                    className="text-sm text-text-secondary leading-relaxed [&_a]:text-game [&_a]:underline [&_img]:max-w-full [&_img]:rounded [&_h1]:font-display [&_h1]:text-text-primary [&_h1]:font-semibold [&_h2]:font-display [&_h2]:text-text-primary [&_h2]:font-semibold"
                    dangerouslySetInnerHTML={{ __html: detail.description_html }}
                  />
                )}

                <div className="hud-rule" />

                {/* Files */}
                <div className="space-y-2">
                  <p className="hud-label text-[10px] text-game2">
                    Files ({detail.files.length})
                  </p>
                  {detail.files.length === 0 ? (
                    <p className="text-xs text-text-muted">No downloadable files.</p>
                  ) : (
                    detail.files.map((file, idx) => (
                      <div
                        key={file.id}
                        className="game-panel flex items-center justify-between p-3 bg-surface-2 border border-surface-3"
                      >
                        <div className="min-w-0">
                          <div className="flex items-center gap-2">
                            <p className="text-xs font-medium text-text-primary truncate">
                              {file.file_name}
                            </p>
                            {idx === 0 && !file.is_archived && (
                              <span className="hud-label text-[9px] text-game bg-game/10 px-1 py-0.5 rounded shrink-0">
                                Latest
                              </span>
                            )}
                          </div>
                          <div className="flex items-center gap-2 mt-0.5">
                            <span className="text-[10px] text-text-muted">
                              {formatSize(file.filesize_bytes)}
                            </span>
                            {file.version && (
                              <span className="text-[10px] text-text-muted">v{file.version}</span>
                            )}
                            <AvBadge result={file.av_result} />
                          </div>
                        </div>
                        <button
                          onClick={() =>
                            onDownload?.(
                              file,
                              detail.name,
                              detail.id,
                              detail.category,
                              images[0] ?? null
                            )
                          }
                          className="game-control flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-medium bg-game text-surface-0 hover:bg-game-hover transition-colors shrink-0 ml-3"
                        >
                          <Download size={12} />
                          Download
                        </button>
                      </div>
                    ))
                  )}
                </div>
              </div>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}

export default GbModDetailPanel;
