import { useCallback, useEffect, useRef, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

/** GitHub releases page — the manual-download fallback when auto-update
 *  isn't possible (e.g. .deb) or fails. */
export const RELEASES_URL = "https://github.com/HKAMIV/mod-manager/releases/latest";

type Phase = "idle" | "checking" | "available" | "downloading" | "installing" | "error" | "done";

interface UseAppUpdateResult {
  /** Current lifecycle phase of the updater. */
  phase: Phase;
  /** The new version string, once an update is found. */
  version: string | null;
  /** Release notes / changelog for the new version, if provided. */
  notes: string | null;
  /** Download progress 0–1 while downloading (null when total is unknown). */
  progress: number | null;
  /** Error message from a failed check or install. */
  error: string | null;
  /** Whether the update banner should be visible (available + not dismissed). */
  visible: boolean;
  /** Begin download + install, then relaunch into the new version. */
  installUpdate: () => Promise<void>;
  /** Dismiss the banner for this session ("Later"). */
  dismiss: () => void;
  /** Manually run a check (e.g. from a Settings button). */
  checkNow: () => Promise<void>;
}

/**
 * App self-update via `tauri-plugin-updater`.
 *
 * Checks once on startup — non-blocking, and silent on failure so a missing
 * network connection or an unreachable manifest never nags the user or delays
 * the UI. When a newer signed release exists, exposes the version + changelog
 * and an install action that downloads, verifies, replaces, and relaunches.
 *
 * Auto-install works for the AppImage and macOS app. For the `.deb` (owned by
 * the system package manager) the updater simply reports no update / can't
 * install; the banner offers the GitHub release link as the fallback.
 */
export function useAppUpdate(): UseAppUpdateResult {
  const [phase, setPhase] = useState<Phase>("idle");
  const [version, setVersion] = useState<string | null>(null);
  const [notes, setNotes] = useState<string | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [dismissed, setDismissed] = useState(false);

  // Hold the resolved Update handle between "found" and "install".
  const updateRef = useRef<Update | null>(null);
  // Guard so the startup check only ever runs once.
  const checkedRef = useRef(false);

  const runCheck = useCallback(async (manual: boolean) => {
    setError(null);
    setPhase("checking");
    try {
      const update = await check();
      if (update) {
        updateRef.current = update;
        setVersion(update.version);
        setNotes(update.body ?? null);
        setDismissed(false);
        setPhase("available");
      } else {
        updateRef.current = null;
        setVersion(null);
        setNotes(null);
        // Manual checks want feedback ("you're up to date"); startup stays quiet.
        setPhase(manual ? "done" : "idle");
      }
    } catch (err) {
      // On startup, fail silently — no update chrome if the check itself
      // couldn't run. A manual check surfaces the error.
      const message = typeof err === "string" ? err : (err as Error)?.message ?? "Update check failed";
      if (manual) {
        setError(message);
        setPhase("error");
      } else {
        setPhase("idle");
      }
    }
  }, []);

  useEffect(() => {
    if (checkedRef.current) return;
    checkedRef.current = true;
    // Fire and forget on startup.
    void runCheck(false);
  }, [runCheck]);

  const installUpdate = useCallback(async () => {
    const update = updateRef.current;
    if (!update) return;

    setError(null);
    setProgress(null);
    setPhase("downloading");

    try {
      let downloaded = 0;
      let total = 0;
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            total = event.data.contentLength ?? 0;
            setProgress(total > 0 ? 0 : null);
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            if (total > 0) setProgress(Math.min(1, downloaded / total));
            break;
          case "Finished":
            setProgress(1);
            setPhase("installing");
            break;
        }
      });

      setPhase("done");
      // Relaunch into the freshly-installed version.
      await relaunch();
    } catch (err) {
      const message = typeof err === "string" ? err : (err as Error)?.message ?? "Update failed to install";
      setError(message);
      setPhase("error");
    }
  }, []);

  const dismiss = useCallback(() => setDismissed(true), []);
  const checkNow = useCallback(() => runCheck(true), [runCheck]);

  const visible = !dismissed && (phase === "available" || phase === "downloading" || phase === "installing" || phase === "error");

  return {
    phase,
    version,
    notes,
    progress,
    error,
    visible,
    installUpdate,
    dismiss,
    checkNow,
  };
}
