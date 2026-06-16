mod frame_server;
mod ndi;
mod preview;
mod sources;
mod types;
mod video_capture;
#[cfg(windows)]
mod windows_performance;
#[cfg(windows)]
mod windows_video;

#[cfg(windows)]
pub(crate) fn disable_background_throttling() {
    windows_performance::disable_process_power_throttling();
}

pub use preview::PreviewManager;
pub use video_capture::VideoCaptureManager;

pub use ndi::is_available as ndi_is_available;
pub use sources::{capture_preview, find_source, list_all_sources, list_presentation_windows};
pub use types::{
    CaptureManager, CaptureSource, PresentationWindow, SlotCaptureState, STREAM_SLOTS, slot_label,
};
