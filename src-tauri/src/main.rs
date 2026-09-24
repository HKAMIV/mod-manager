// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebKitGTK rendering fallbacks on Linux for blank-window / accelerated-
    // rendering failures on some driver stacks.
    //
    // NOTE: the SteamOS/Steam Deck blank-window bug ("Could not create default
    // EGL display: EGL_BAD_PARAMETER. Aborting...") is NOT fixed here — its
    // root cause is the AppImage over-bundling its own libwayland-client, which
    // is loaded by AppRun before this code runs, so an in-process env var is
    // too late to matter (verified: tauri-apps/tauri#15665). That's fixed in
    // the release workflow by stripping the bundled Wayland libs from the
    // AppImage so the host's correct versions are used instead.
    //
    // The vars below remain as harmless, user-overridable fallbacks for other
    // setups (e.g. some NVIDIA/Wayland combos) where disabling the DMABUF
    // renderer or accelerated compositing avoids a blank window.
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("APPIMAGE").is_some()
            && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none()
        {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        if std::env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_none() {
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
    }

    mod_manager_lib::run();
}
