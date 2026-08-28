// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Workaround for WebKitGTK rendering issues on Linux, particularly in
    // AppImage environments where the DMABUF renderer frequently fails due
    // to the sandboxed FUSE mount conflicting with WebKit's buffer
    // negotiation. The .deb (system-installed) binary doesn't hit this
    // because it runs with direct access to /dev/dri. See:
    // https://v2.tauri.app/develop/debug/linux-graphics/
    // https://github.com/tauri-apps/tauri/issues/9394
    #[cfg(target_os = "linux")]
    {
        // Only set if running inside an AppImage (APPIMAGE env var is set by
        // the AppImage runtime) to avoid penalizing native package installs
        // that don't need the workaround.
        if std::env::var_os("APPIMAGE").is_some() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    mod_manager_lib::run();
}
