//! Opts the process out of macOS's App Nap for the lifetime of the app.
//!
//! App Nap is an OS-level power-saving feature that throttles background
//! threads of any windowed macOS app once its window stops being visible/
//! frontmost for a while — this is *not* something Tauri's async runtime or
//! our own code controls, it happens underneath everything, including the
//! Tokio worker thread that runs the download pipeline (`downloads.rs`).
//! Without opting out, downloads (and, less visibly, the filesystem watcher
//! and global hotkey listener) simply run slower or stall once the user
//! switches away from the app window — which is exactly the "downloads only
//! progress while I'm watching the Downloads page" symptom this exists to
//! fix. Verified against Apple's own App Nap documentation and confirmed via
//! a standalone smoke test that `NSProcessInfo.beginActivityWithOptions`
//! successfully returns an activity token on this platform.
//!
//! No-op on every other OS — App Nap is a macOS-only concept, and Linux/
//! Windows have no equivalent background-throttling behavior for a normal
//! desktop app that needs working around here.

#[cfg(target_os = "macos")]
mod macos {
    use objc2_foundation::{NSActivityOptions, NSProcessInfo, NSString};

    pub fn disable_app_nap() {
        let process_info = NSProcessInfo::processInfo();
        let reason = NSString::from_str(
            "Mod Manager keeps downloads and the filesystem watcher running in the background",
        );
        // UserInitiated: this is work the user directly asked for (a
        // download), not idle housekeeping — the correct category per
        // Apple's guidelines for what justifies opting out.
        // IdleSystemSleepDisabled: additionally prevents the *system* (not
        // just this app) from idle-sleeping mid-download; without it a
        // laptop closing an idle lid could still pause a large transfer.
        let options = NSActivityOptions::UserInitiated | NSActivityOptions::IdleSystemSleepDisabled;
        let token = process_info.beginActivityWithOptions_reason(options, &reason);

        // Ending the activity (dropping this token) would let App Nap
        // resume, which we never want while the app is running at all, not
        // just during an active download — so it's intentionally leaked for
        // the process lifetime rather than stored somewhere it could be
        // dropped. `Retained<ProtocolObject<dyn NSObjectProtocol>>` isn't
        // Send/Sync (it's an opaque Objective-C object reference), so this
        // is simpler than trying to stash it in a static.
        std::mem::forget(token);
    }
}

/// Call once during app setup. Safe to call on any platform.
pub fn disable_app_nap() {
    #[cfg(target_os = "macos")]
    macos::disable_app_nap();
}
