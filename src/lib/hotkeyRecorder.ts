/**
 * Converts a KeyboardEvent into a Tauri global-shortcut accelerator string
 * (e.g. "CommandOrControl+Alt+1"), or null if the event is just a bare
 * modifier press with no "real" key yet.
 *
 * Tauri's shortcut parser expects modifier names joined with "+" followed by
 * a single key code. We use "CommandOrControl" so the same binding works as
 * Ctrl on Linux/Windows and Cmd on macOS.
 */
export function eventToAccelerator(e: KeyboardEvent): string | null {
  const parts: string[] = [];

  if (e.ctrlKey || e.metaKey) parts.push("CommandOrControl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");

  const key = normalizeKey(e.key, e.code);
  if (key === null) {
    // Bare modifier key with nothing else pressed yet — not a complete shortcut.
    return null;
  }

  parts.push(key);
  return parts.join("+");
}

const MODIFIER_KEYS = new Set(["Control", "Meta", "Alt", "Shift"]);

function normalizeKey(key: string, code: string): string | null {
  if (MODIFIER_KEYS.has(key)) {
    return null;
  }

  // Single letters/digits: use the uppercase key.
  if (/^[a-zA-Z0-9]$/.test(key)) {
    return key.toUpperCase();
  }

  // Function keys (F1-F24) map directly.
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(key)) {
    return key;
  }

  const named: Record<string, string> = {
    " ": "Space",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    Escape: "Escape",
    Enter: "Enter",
    Tab: "Tab",
    Backspace: "Backspace",
    Delete: "Delete",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
    Insert: "Insert",
  };
  if (key in named) {
    return named[key];
  }

  // Fall back to the physical key code for punctuation/symbol keys
  // (e.g. "Comma", "Period", "Minus") which KeyboardEvent.code reports
  // in a Tauri-friendly form already.
  if (code && /^[A-Za-z]+$/.test(code)) {
    return code;
  }

  return null;
}

/** Human-readable rendering of an accelerator string, e.g. "Ctrl+Alt+1". */
export function formatAccelerator(accelerator: string): string {
  return accelerator.replace("CommandOrControl", navigator.platform.includes("Mac") ? "Cmd" : "Ctrl");
}
