// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Workarounds for WebKitGTK rendering failures on Linux that show up as a
    // blank (white/black) window even though the app is running fine. These
    // bite hardest on SteamOS / Steam Deck (Gamescope + Mesa) and in AppImage
    // environments. The system-installed .deb tends to avoid them because it
    // runs against the host WebKitGTK/graphics stack directly. See:
    //   https://tauri.app/develop/debug/linux-graphics/
    //   https://github.com/tauri-apps/tauri/issues/9394
    //   (SteamOS Game Mode blank window, same fix as a sibling Tauri app:)
    //   https://github.com/YARC-Official/YARC-Launcher/issues/42
    //
    // Each is only set when the user hasn't already set it, so anyone can
    // override our defaults from the environment / Steam launch options.
    #[cfg(target_os = "linux")]
    {
        // Disabling accelerated compositing is the variable that actually
        // fixes the SteamOS blank-window case. Apply it on all Linux (not just
        // AppImage) so Game Mode is covered regardless of how the app was
        // packaged. It falls back to a software-composited path that the Deck's
        // driver stack handles reliably.
        if std::env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_none() {
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }

        // The DMABUF renderer fails specifically inside an AppImage's sandboxed
        // FUSE mount (buffer negotiation with the host GPU). Gate this one to
        // AppImage so native installs keep the faster path.
        if std::env::var_os("APPIMAGE").is_some()
            && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none()
        {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    mod_manager_lib::run();
}
