use std::slice;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use windows::Win32::Graphics::Direct3D11::{D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ};
use windows_capture::capture::{CaptureControl, Context, GraphicsCaptureApiHandler};
use windows_capture::d3d11::StagingTexture;
use windows_capture::frame::Frame;
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

/// Copies RGBA8 rows out of a mapped staging texture into `scratch`, dropping row
/// padding. Returns `None` when the metadata does not match the backing slice
/// (common while a captured window is resizing).
fn try_extract_rgba8_pixels(
    raw: &[u8],
    width: u32,
    height: u32,
    row_pitch: usize,
    scratch: &mut Vec<u8>,
) -> Option<()> {
    if width == 0 || height == 0 {
        return None;
    }

    let width_bytes = (width as usize).checked_mul(4)?;
    let frame_size = width_bytes.checked_mul(height as usize)?;
    if row_pitch < width_bytes || raw.len() < row_pitch.checked_mul(height as usize)? {
        return None;
    }

    scratch.resize(frame_size, 0);

    if row_pitch == width_bytes {
        scratch.copy_from_slice(&raw[..frame_size]);
        return Some(());
    }

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
    staging: Option<StagingTexture>,
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
            staging: None,
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

        let desc = *frame.desc();
        let width = desc.Width;
        let height = desc.Height;
        if width == 0 || height == 0 {
            return Ok(());
        }

        // `Frame::buffer()` allocates a fresh staging texture on every frame and never
        // unmaps it, which leaks mapped memory on many drivers and OOMs the process
        // after a few minutes of capture. Copy through one reusable staging texture
        // with a proper Map/Unmap cycle instead.
        let staging_matches = self.staging.as_ref().is_some_and(|staging| {
            let staging_desc = staging.desc();
            staging_desc.Width == width
                && staging_desc.Height == height
                && staging_desc.Format == desc.Format
        });
        if !staging_matches {
            self.staging = Some(
                StagingTexture::new(frame.device(), width, height, desc.Format)
                    .map_err(|error| error.to_string())?,
            );
        }
        let staging = self.staging.as_ref().expect("staging texture set above");

        let context = frame.device_context();
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            context.CopyResource(staging.texture(), frame.as_raw_texture());
            context
                .Map(staging.texture(), 0, D3D11_MAP_READ, 0, Some(&raw mut mapped))
                .map_err(|error| error.to_string())?;
        }

        let row_pitch = mapped.RowPitch as usize;
        let raw = unsafe {
            slice::from_raw_parts(mapped.pData as *const u8, row_pitch * height as usize)
        };
        let extracted = try_extract_rgba8_pixels(raw, width, height, row_pitch, &mut self.scratch);

        unsafe {
            context.Unmap(staging.texture(), 0);
        }

        if extracted.is_none() {
            tracing::trace!(
                width,
                height,
                row_pitch,
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

        // WGC only delivers frames when DWM repaints the monitor. Seed a synchronous grab so
        // static desktops are not black until the user moves the mouse.
        if matches!(source.kind, CaptureSourceKind::Screen) {
            match sources::capture_monitor_pixels(&source.id) {
                Ok((width, height, pixels)) => {
                    frame_slot.publish(width, height, pixels);
                }
                Err(error) => {
                    tracing::warn!(%error, source_id = %source.id, "failed to seed initial screen frame");
                }
            }
        }

        let flags = (Arc::clone(&frame_slot), stop.clone());
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
