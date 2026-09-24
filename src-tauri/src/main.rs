// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Workarounds for WebKitGTK rendering failures on Linux that show up as a
    // blank window (often with "Could not create default EGL display:
    // EGL_BAD_PARAMETER. Aborting."). The system-installed .deb avoids them
    // because it uses the host graphics/Wayland libraries directly.
    //
    // Root cause on SteamOS/Steam Deck (verified, tauri-apps/tauri#15665):
    // the AppImage over-bundles its own libwayland-client (~1.22), and loading
    // that against the host's newer Mesa (25+) makes
    // eglGetDisplay(EGL_DEFAULT_DISPLAY) fail with EGL_BAD_PARAMETER under a
    // Wayland session — WebKitWebProcess then aborts before rendering anything.
    // See also:
    //   https://tauri.app/develop/debug/linux-graphics/
    //   https://github.com/tauri-apps/tauri/issues/15665
    //
    // Each var is only set when the user hasn't already set it, so anyone can
    // override from the environment / Steam launch options.
    #[cfg(target_os = "linux")]
    {
        // Primary fix for the EGL_BAD_PARAMETER abort: route GTK/WebKit through
        // X11 (XWayland) instead of native Wayland, sidestepping the bundled
        // libwayland-vs-host-Mesa mismatch entirely while keeping GPU
        // acceleration. XWayland is always present under Gamescope/SteamOS.
        // Only forced inside an AppImage — the .deb/native path is fine on
        // Wayland and shouldn't be pushed onto XWayland.
        let in_appimage = std::env::var_os("APPIMAGE").is_some();
        if in_appimage && std::env::var_os("GDK_BACKEND").is_none() {
            std::env::set_var("GDK_BACKEND", "x11");
        }

        // Belt-and-suspenders fallbacks. If X11/EGL still can't get a hardware
        // context, disabling the DMABUF renderer forces WebKit onto a path that
        // doesn't need the failing EGL display. AppImage-gated so native
        // installs keep the faster path.
        if in_appimage && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }

        // Accelerated compositing off is a broad safety net for the blank-window
        // class of bugs on the Deck's driver stack; harmless elsewhere.
        if std::env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_none() {
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
    }

    mod_manager_lib::run();
}
