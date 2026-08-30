import { create } from "zustand";
import { invoke } from "../lib/invoke";
import type { GameId, ModInfo, Settings, View } from "../types";

interface AppState {
  // Navigation
  currentView: View;
  setCurrentView: (view: View) => void;

  // Detail panel
  detailPanelOpen: boolean;
  toggleDetailPanel: () => void;

  // Settings
  settings: Settings | null;
  setSettings: (settings: Settings) => void;

  // Active game
  activeGame: GameId;
  setActiveGame: (game: GameId) => void;

  // Sidebar collapsed
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;

  // Currently selected mod, shown in the detail panel
  selectedMod: ModInfo | null;
  setSelectedMod: (mod: ModInfo | null) => void;
}

export const useAppStore = create<AppState>((set, get) => ({
  currentView: "local",
  setCurrentView: (view) => set({ currentView: view }),

  detailPanelOpen: false,
  toggleDetailPanel: () =>
    set((state) => ({ detailPanelOpen: !state.detailPanelOpen })),

  settings: null,
  setSettings: (settings) =>
    // Adopt the persisted active game whenever settings load, so the app
    // reopens on the game the user was last using rather than the hardcoded
    // default. Guarded so a settings refresh mid-session (e.g. after setting
    // a mod path) doesn't clobber an active-game switch the user just made.
    set((state) => ({
      settings,
      activeGame: state.settings ? state.activeGame : (settings.active_game as GameId),
    })),

  activeGame: "wuthering-waves",
  setActiveGame: (game) => {
    set({ activeGame: game, selectedMod: null });
    // Persist the choice so it survives a restart. Fire-and-forget — a failed
    // write just means the preference isn't remembered next session, which is
    // non-critical and shouldn't block the UI switch that already happened.
    const { settings } = get();
    if (settings && settings.active_game !== game) {
      const updated = { ...settings, active_game: game };
      set({ settings: updated });
      invoke("save_settings", { settings: updated }).catch((err) =>
        console.error("Failed to persist active game:", err)
      );
    }
  },

  sidebarCollapsed: false,
  toggleSidebar: () =>
    set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),

  selectedMod: null,
  setSelectedMod: (mod) => set({ selectedMod: mod }),
}));
