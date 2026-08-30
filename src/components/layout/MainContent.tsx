import type { ReactNode } from "react";
import { useAppStore } from "../../stores/appStore";
import LocalView from "../../pages/LocalView";
import OnlineView from "../../pages/OnlineView";
import DownloadsView from "../../pages/DownloadsView";
import SettingsView from "../../pages/SettingsView";
import type { View } from "../../types";

/**
 * All four views stay mounted for the lifetime of the app; only the active
 * one is visible. This preserves each page's in-memory state (scanned mods,
 * search text, filters, scroll position, GameBanana browse page) across
 * navigation, instead of tearing it down and rebuilding from scratch every
 * time the user switches tabs.
 *
 * The trade-off is that all page hooks stay active in the background, but
 * that's cheap here: the mod watcher is keyed to the active game and the
 * GameBanana browse only fetches on state change, not on a timer.
 */
function ViewSlot({ view, current, children }: { view: View; current: View; children: ReactNode }) {
  const active = view === current;
  // `hidden` removes the view from layout/paint without unmounting it, so all
  // of the page's state is retained across navigation. The tree structure is
  // identical in both states (same single wrapper div) so React never remounts
  // the child — only the className changes. The `page-enter` class is applied
  // only when active, replaying the entrance animation each time the view is
  // shown.
  return (
    <div
      className={`flex-1 flex-col h-full ${active ? "flex page-enter" : "hidden"}`}
      aria-hidden={!active}
    >
      {children}
    </div>
  );
}

function MainContent() {
  const { currentView } = useAppStore();

  return (
    <main className="relative flex-1 flex flex-col h-full overflow-hidden">
      <ViewSlot view="local" current={currentView}>
        <LocalView />
      </ViewSlot>
      <ViewSlot view="online" current={currentView}>
        <OnlineView />
      </ViewSlot>
      <ViewSlot view="downloads" current={currentView}>
        <DownloadsView />
      </ViewSlot>
      <ViewSlot view="settings" current={currentView}>
        <SettingsView />
      </ViewSlot>
    </main>
  );
}

export default MainContent;
