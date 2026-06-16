use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use windows_capture::capture::{CaptureControl, Context, GraphicsCaptureApiHandler};
use windows_capture::frame::{Frame, FrameBuffer};
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::monitor::Monitor;
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};
use super::frame_server::FrameSlot;
use super::sources::{self, CaptureError};
use super::types::{CaptureSource, CaptureSourceKind};

type HandlerError = Box<dyn std::error::Error + Send + Sync>;
type HandlerControl = CaptureControl<FrameCaptureHandler, HandlerError>;

type HandlerFlags = (Arc<FrameSlot>, Arc<AtomicBool>);

/// Copies RGBA8 pixels out of a DXGI frame buffer, skipping frames whose metadata
/// does not match the backing slice (common while a captured window is resizing).
fn try_extract_rgba8_pixels(buffer: &mut FrameBuffer<'_>, scratch: &mut Vec<u8>) -> Option<()> {
    let width = buffer.width();
    let height = buffer.height();
    if width == 0 || height == 0 {
        return None;
    }

    let width_bytes = (width as usize).checked_mul(4)?;
    let frame_size = width_bytes.checked_mul(height as usize)?;
    let has_padding = buffer.has_padding();
    let row_pitch = buffer.row_pitch() as usize;
    let raw = buffer.as_raw_buffer();

    if !has_padding {
        if raw.len() < frame_size {
            return None;
        }

        scratch.resize(frame_size, 0);
        scratch.copy_from_slice(&raw[..frame_size]);
        return Some(());
    }

    let required_raw = row_pitch.checked_mul(height as usize)?;
    if raw.len() < required_raw {
        return None;
    }

    scratch.resize(frame_size, 0);

    for y in 0..height as usize {
        let src_start = y * row_pitch;
        let dst_start = y * width_bytes;
        scratch[dst_start..dst_start + width_bytes]
            .copy_from_slice(&raw[src_start..src_start + width_bytes]);
    }

    Some(())
}

struct FrameCaptureHandler {
    frame_slot: Arc<FrameSlot>,
    stop: Arc<AtomicBool>,
    scratch: Vec<u8>,
}

impl GraphicsCaptureApiHandler for FrameCaptureHandler {
    type Flags = HandlerFlags;
    type Error = HandlerError;

    fn new(context: Context<Self::Flags>) -> Result<Self, Self::Error> {
        super::windows_performance::configure_high_priority_worker_thread();

        Ok(Self {
            frame_slot: context.flags.0,
            stop: context.flags.1,
            scratch: Vec::new(),
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        if self.stop.load(Ordering::Relaxed) {
            capture_control.stop();
            return Ok(());
        }

        let mut buffer = frame.buffer().map_err(|error| error.to_string())?;
        let width = buffer.width();
        let height = buffer.height();

        if try_extract_rgba8_pixels(&mut buffer, &mut self.scratch).is_none() {
            tracing::trace!(
                width,
                height,
                raw_len = buffer.as_raw_buffer().len(),
                row_pitch = buffer.row_pitch(),
                "skipping DXGI frame with inconsistent buffer during resize"
            );
            return Ok(());
        }

        self.frame_slot
            .publish(width, height, std::mem::take(&mut self.scratch));
        Ok(())
    }
}

pub struct WindowsCaptureSession {
    stop: Arc<AtomicBool>,
    control: Option<HandlerControl>,
    #[cfg(windows)]
    _active_guard: super::windows_performance::ActiveCaptureGuard,
}

impl WindowsCaptureSession {
    pub fn start(source: &CaptureSource, frame_slot: Arc<FrameSlot>) -> Result<Self, CaptureError> {
        #[cfg(windows)]
        let _active_guard = super::windows_performance::ActiveCaptureGuard::acquire();

        let stop = Arc::new(AtomicBool::new(false));
        let flags = (frame_slot, stop.clone());
        let control = match source.kind {
            CaptureSourceKind::Screen => {
                let index = sources::parse_id_suffix(&source.id, "screen:")? as usize;
                let monitor = Monitor::from_index(index)
                    .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

                let settings = Settings::new(
                    monitor,
                    CursorCaptureSettings::WithoutCursor,
                    DrawBorderSettings::Default,
                    SecondaryWindowSettings::Default,
                    MinimumUpdateIntervalSettings::Custom(Duration::from_millis(33)),
                    DirtyRegionSettings::Default,
                    ColorFormat::Rgba8,
                    flags,
                );

                FrameCaptureHandler::start_free_threaded(settings)
                    .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?
            }
            _ => {
                return Err(CaptureError::Unsupported(
                    "windows capture only supports screen sources".into(),
                ));
            }
        };

        Ok(Self {
            stop,
            control: Some(control),
            #[cfg(windows)]
            _active_guard,
        })
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);

        if let Some(control) = self.control.take() {
            if let Err(error) = control.stop() {
                tracing::warn!(%error, "failed to stop DXGI capture session cleanly");
            }
        }
    }
}
