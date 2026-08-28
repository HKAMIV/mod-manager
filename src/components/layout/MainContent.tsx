import { useAppStore } from "../../stores/appStore";
import LocalView from "../../pages/LocalView";
import OnlineView from "../../pages/OnlineView";
import DownloadsView from "../../pages/DownloadsView";
import SettingsView from "../../pages/SettingsView";

function MainContent() {
  const { currentView } = useAppStore();

  const renderView = () => {
    switch (currentView) {
      case "local":
        return <LocalView />;
      case "online":
        return <OnlineView />;
      case "downloads":
        return <DownloadsView />;
      case "settings":
        return <SettingsView />;
      default:
        return <LocalView />;
    }
  };

  return (
    <main className="relative flex-1 flex flex-col h-full overflow-hidden">
      <div key={currentView} className="flex-1 flex flex-col h-full page-enter">
        {renderView()}
      </div>
    </main>
  );
}

export default MainContent;
