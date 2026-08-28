import { create } from "zustand";
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

export const useAppStore = create<AppState>((set) => ({
  currentView: "local",
  setCurrentView: (view) => set({ currentView: view }),

  detailPanelOpen: false,
  toggleDetailPanel: () =>
    set((state) => ({ detailPanelOpen: !state.detailPanelOpen })),

  settings: null,
  setSettings: (settings) => set({ settings }),

  activeGame: "wuthering-waves",
  setActiveGame: (game) => set({ activeGame: game, selectedMod: null }),

  sidebarCollapsed: false,
  toggleSidebar: () =>
    set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),

  selectedMod: null,
  setSelectedMod: (mod) => set({ selectedMod: mod }),
}));
