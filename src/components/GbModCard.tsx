import { useState } from "react";
import { ImageOff, Heart, Eye, Star, ShieldAlert } from "lucide-react";
import type { GbModSummary } from "../types";

interface Props {
  mod: GbModSummary;
  onSelect: (mod: GbModSummary) => void;
}

function formatCount(n: number): string {
  if (n < 1000) return String(n);
  if (n < 1_000_000) return `${(n / 1000).toFixed(1)}K`;
  return `${(n / 1_000_000).toFixed(1)}M`;
}

function GbModCard({ mod, onSelect }: Props) {
  const [imageError, setImageError] = useState(false);

  return (
    <button
      onClick={() => onSelect(mod)}
      className="game-panel group relative flex flex-col overflow-hidden border border-surface-3 bg-surface-1 hover:border-game/40 text-left transition-all"
    >
      {/* Thumbnail */}
      <div className="relative aspect-video bg-surface-2 flex items-center justify-center overflow-hidden">
        {mod.thumbnail_url && !imageError ? (
          <img
            src={mod.thumbnail_url}
            alt={mod.name}
            onError={() => setImageError(true)}
            loading="lazy"
            className="w-full h-full object-cover"
          />
        ) : (
          <ImageOff size={22} className="text-text-muted" />
        )}

        {mod.featured && (
          <span className="absolute top-2 left-2 flex items-center gap-1 px-1.5 py-0.5 rounded-full text-[10px] font-medium bg-gold/20 text-gold border border-gold/40 backdrop-blur-md">
            <Star size={9} fill="currentColor" />
            Featured
          </span>
        )}
        {mod.likely_nsfw && (
          <span className="absolute top-2 right-2 flex items-center gap-1 px-1.5 py-0.5 rounded-full text-[10px] font-medium bg-game2/20 text-game2 border border-game2/40 backdrop-blur-md">
            <ShieldAlert size={9} />
            NSFW
          </span>
        )}
      </div>

      {/* Info */}
      <div className="p-3 space-y-1.5">
        <p className="text-sm font-medium text-text-primary leading-snug line-clamp-2">
          {mod.name}
        </p>
        <div className="flex items-center justify-between text-xs text-text-muted">
          <span className="truncate">{mod.sub_category ?? mod.category ?? "Mod"}</span>
        </div>
        <div className="flex items-center justify-between">
          <span className="text-xs text-text-muted truncate">
            {mod.submitter_name ?? "Unknown"}
          </span>
          <div className="flex items-center gap-2 shrink-0 ml-2 text-text-muted">
            <span className="flex items-center gap-0.5 text-[11px]">
              <Heart size={10} />
              {formatCount(mod.like_count)}
            </span>
            <span className="flex items-center gap-0.5 text-[11px]">
              <Eye size={10} />
              {formatCount(mod.view_count)}
            </span>
          </div>
        </div>
      </div>
    </button>
  );
}

export default GbModCard;
