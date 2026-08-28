import { useEffect } from "react";
import { useAppStore } from "./stores/appStore";
import Sidebar from "./components/layout/Sidebar";
import MainContent from "./components/layout/MainContent";
import DetailPanel from "./components/layout/DetailPanel";
import { invoke } from "./lib/invoke";
import type { Settings } from "./types";

function App() {
  const { detailPanelOpen, activeGame, setSettings } = useAppStore();

  useEffect(() => {
    invoke<Settings>("get_settings")
      .then(setSettings)
      .catch((err) => console.error("Failed to load settings:", err));
  }, [setSettings]);

  return (
    <div
      data-game={activeGame}
      className="app-shell flex h-screen w-screen"
    >
      <Sidebar />
      <MainContent />
      {detailPanelOpen && <DetailPanel />}
    </div>
  );
}

export default App;
