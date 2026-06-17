mod frame_server;
mod macos_screen_permission;
mod ndi;
mod preview;
mod sources;
mod types;
mod video_capture;
#[cfg(windows)]
mod windows_performance;
#[cfg(windows)]
mod windows_video;

use std::time::Duration;

use tauri::Manager;

#[cfg(windows)]
pub(crate) fn disable_background_throttling() {
    windows_performance::disable_process_power_throttling();
}

pub use preview::PreviewManager;
pub use video_capture::{CaptureDiagnostics, VideoCaptureManager};

pub fn spawn_diagnostics_task(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(300));
            let video = app.state::<std::sync::Mutex<VideoCaptureManager>>();
            let Ok(guard) = video.lock() else {
                tracing::warn!("capture diagnostics: video manager lock unavailable");
                continue;
            };

            let diagnostics = guard.diagnostics();
            tracing::info!(?diagnostics, "capture diagnostics snapshot");
        }
    });
}

pub use macos_screen_permission::{
    ensure_access as ensure_screen_capture_access, has_access as has_screen_capture_access,
    request_access as request_screen_capture_access,
};
pub use ndi::is_available as ndi_is_available;
pub use sources::{capture_preview, find_source, list_all_sources, list_presentation_windows};
pub use types::{
    CaptureManager, CaptureSource, PresentationWindow, SlotCaptureState, STREAM_SLOTS, slot_label,
};
