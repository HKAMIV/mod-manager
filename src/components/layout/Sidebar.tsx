import { useAppStore } from "../../stores/appStore";
import GameSelector from "../GameSelector";
import { useDownloads } from "../../hooks/useDownloads";
import { Folder, Globe, Download, Settings, PanelLeftClose, PanelLeftOpen, Zap } from "lucide-react";
import type { View } from "../../types";

interface NavItem {
  id: View;
  label: string;
  icon: React.ReactNode;
}

const navItems: NavItem[] = [
  { id: "local", label: "Local Mods", icon: <Folder size={20} /> },
  { id: "online", label: "Online", icon: <Globe size={20} /> },
  { id: "downloads", label: "Downloads", icon: <Download size={20} /> },
  { id: "settings", label: "Settings", icon: <Settings size={20} /> },
];

function Sidebar() {
  const { currentView, setCurrentView, sidebarCollapsed, toggleSidebar } =
    useAppStore();
  const { downloads } = useDownloads();

  // Downloads that need the user to do something — either it's stuck
  // waiting on a conflict decision, or it failed outright. Both are easy to
  // miss entirely if the only place they show up is the Downloads page
  // itself, so surface a count badge on the nav item from anywhere in the app.
  const needsAttentionCount = downloads.filter(
    (d) => d.status.state === "waiting_for_conflict" || d.status.state === "failed"
  ).length;

  return (
    <aside
      className={`relative flex flex-col h-full bg-surface-1/90 backdrop-blur-xl border-r border-surface-3 transition-all duration-200 ${
        sidebarCollapsed ? "w-16" : "w-60"
      }`}
    >
      {/* Right-edge accent rule — glows with the current game color */}
      <div className="absolute right-0 top-0 bottom-0 w-px bg-gradient-to-b from-transparent via-game/60 to-transparent" />

      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b border-surface-3">
        {!sidebarCollapsed ? (
          <div className="flex items-center gap-2 min-w-0">
            <span className="game-control flex items-center justify-center w-7 h-7 bg-game/10 text-game shadow-glow-game shrink-0">
              <Zap size={15} strokeWidth={2.5} />
            </span>
            <span className="font-display font-semibold text-base text-text-primary tracking-wide truncate">
              MOD<span className="text-game">MGR</span>
            </span>
          </div>
        ) : (
          <span className="game-control flex items-center justify-center w-7 h-7 bg-game/10 text-game shadow-glow-game mx-auto">
            <Zap size={15} strokeWidth={2.5} />
          </span>
        )}
        {!sidebarCollapsed && (
          <button
            onClick={toggleSidebar}
            className="p-1.5 rounded hover:bg-surface-2 text-text-secondary hover:text-text-primary transition-colors"
            aria-label="Collapse sidebar"
          >
            <PanelLeftClose size={18} />
          </button>
        )}
      </div>
      {sidebarCollapsed && (
        <button
          onClick={toggleSidebar}
          className="mx-auto mt-2 p-1.5 rounded hover:bg-surface-2 text-text-secondary hover:text-text-primary transition-colors"
          aria-label="Expand sidebar"
        >
          <PanelLeftOpen size={18} />
        </button>
      )}

      {/* Game Selector */}
      <div className="p-2 border-b border-surface-3">
        <GameSelector collapsed={sidebarCollapsed} />
      </div>

      {/* Navigation */}
      <nav className="flex-1 p-2 space-y-1">
        {!sidebarCollapsed && (
          <p className="hud-label text-[10px] text-text-muted px-3 pt-1 pb-2">
            Navigation
          </p>
        )}
        {navItems.map((item) => {
          const active = currentView === item.id;
          const showBadge = item.id === "downloads" && needsAttentionCount > 0;
          return (
            <button
              key={item.id}
              onClick={() => setCurrentView(item.id)}
              className={`game-control group relative flex items-center gap-3 w-full px-3 py-2 text-sm font-medium transition-all duration-150 ${
                active
                  ? "bg-game/10 text-game"
                  : "text-text-secondary hover:bg-surface-2 hover:text-text-primary"
              }`}
              title={
                sidebarCollapsed
                  ? showBadge
                    ? `${item.label} — ${needsAttentionCount} need${needsAttentionCount === 1 ? "s" : ""} attention`
                    : item.label
                  : undefined
              }
            >
              {active && (
                <span className="absolute left-0 top-1.5 bottom-1.5 w-0.5 rounded-full bg-game shadow-glow-game" />
              )}
              <span className={`relative ${active ? "drop-shadow-[0_0_6px_var(--game-accent)]" : ""}`}>
                {item.icon}
                {showBadge && (
                  <span className="absolute -top-1.5 -right-1.5 flex items-center justify-center min-w-[16px] h-4 px-1 rounded-full bg-game2 text-surface-0 text-[9px] font-bold shadow-glow-game2 leading-none">
                    {needsAttentionCount > 9 ? "9+" : needsAttentionCount}
                  </span>
                )}
              </span>
              {!sidebarCollapsed && <span>{item.label}</span>}
              {!sidebarCollapsed && showBadge && (
                <span className="ml-auto flex items-center justify-center min-w-[18px] h-[18px] px-1 rounded-full bg-game2/15 text-game2 text-[10px] font-semibold">
                  {needsAttentionCount}
                </span>
              )}
            </button>
          );
        })}
      </nav>

      {/* Footer */}
      <div className="p-3 border-t border-surface-3">
        {!sidebarCollapsed ? (
          <div className="flex items-center justify-between">
            <p className="text-[11px] text-text-muted font-mono">v{__APP_VERSION__}</p>
            <span className="flex items-center gap-1.5 text-[11px] text-text-muted">
              <span className="w-1.5 h-1.5 rounded-full bg-accent shadow-glow-accent animate-pulse-glow" />
              Ready
            </span>
          </div>
        ) : (
          <span className="mx-auto block w-1.5 h-1.5 rounded-full bg-accent shadow-glow-accent animate-pulse-glow" />
        )}
      </div>
    </aside>
  );
}

export default Sidebar;
