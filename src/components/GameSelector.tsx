import { useAppStore } from "../stores/appStore";
import { ChevronDown } from "lucide-react";
import { useState } from "react";
import type { GameId } from "../types";

interface GameInfo {
  id: GameId;
  name: string;
  shortName: string;
  /** Static swatch color for this game — independent of the currently active
   * --game-accent cascade, so every entry in the list shows its own identity
   * color rather than all matching whichever game happens to be active. */
  swatch: string;
}

const games: GameInfo[] = [
  { id: "wuthering-waves", name: "Wuthering Waves", shortName: "WuWa", swatch: "#00e5ff" },
  { id: "genshin-impact", name: "Genshin Impact", shortName: "GI", swatch: "#48d1cc" },
  { id: "zenless-zone-zero", name: "Zenless Zone Zero", shortName: "ZZZ", swatch: "#fafd00" },
  { id: "honkai-star-rail", name: "Honkai Star Rail", shortName: "HSR", swatch: "#8b5cf6" },
  { id: "arknights-endfield", name: "Arknights Endfield", shortName: "AE", swatch: "#ff5500" },
];

interface Props {
  collapsed: boolean;
}

function GameSelector({ collapsed }: Props) {
  const { activeGame, setActiveGame } = useAppStore();
  const [isOpen, setIsOpen] = useState(false);

  const currentGame = games.find((g) => g.id === activeGame) ?? games[0];

  const handleSelect = (id: GameId) => {
    setActiveGame(id);
    setIsOpen(false);
  };

  return (
    <div className="relative">
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="game-control flex items-center justify-between w-full px-2.5 py-2 bg-surface-2 border border-surface-3 hover:border-game/40 text-text-primary text-sm font-medium transition-colors"
        aria-label="Select game"
        aria-expanded={isOpen}
      >
        <span className="flex items-center gap-2 min-w-0">
          <span className="w-2 h-2 rounded-full bg-game shadow-glow-game shrink-0" />
          <span className="truncate">
            {collapsed ? currentGame.shortName : currentGame.name}
          </span>
        </span>
        {!collapsed && (
          <ChevronDown
            size={14}
            className={`text-text-muted transition-transform ${isOpen ? "rotate-180" : ""}`}
          />
        )}
      </button>

      {isOpen && (
        <div className="game-panel absolute z-50 top-full left-0 right-0 mt-1.5 bg-surface-2 border border-surface-3 shadow-glow-accent2 shadow-2xl overflow-hidden">
          {games.map((game) => {
            const isActive = game.id === activeGame;
            return (
              <button
                key={game.id}
                onClick={() => handleSelect(game.id)}
                className={`flex items-center gap-2 w-full text-left px-3 py-2 text-sm transition-colors ${
                  isActive
                    ? "bg-game/10 text-game"
                    : "text-text-secondary hover:bg-surface-2 hover:text-text-primary"
                }`}
              >
                <span
                  className="w-1.5 h-1.5 rounded-full shrink-0"
                  style={{
                    backgroundColor: isActive ? "var(--game-accent)" : game.swatch,
                    boxShadow: isActive
                      ? "0 0 8px -1px var(--game-accent)"
                      : `0 0 6px -1px ${game.swatch}`,
                  }}
                />
                {collapsed ? game.shortName : game.name}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

export default GameSelector;
