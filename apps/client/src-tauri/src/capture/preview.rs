use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use image::RgbaImage;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use xcap::Monitor;

use super::sources::{self, CaptureError};
use super::types::{CaptureSource, CaptureSourceKind};

const TARGET_FRAME_INTERVAL: Duration = Duration::from_micros(33_333);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotPreviewFrame {
    pub slot: String,
    pub preview: String,
}

struct SlotPreviewWorker {
    source_id: String,
    stop: Arc<AtomicBool>,
    join: JoinHandle<()>,
}

pub struct PreviewManager {
    workers: Mutex<HashMap<String, SlotPreviewWorker>>,
}

impl PreviewManager {
    pub fn new() -> Self {
        Self {
            workers: Mutex::new(HashMap::new()),
        }
    }

    pub fn sync_slot(
        &self,
        app: &AppHandle,
        slot: &str,
        source_id: Option<&str>,
        source: Option<CaptureSource>,
    ) {
        match source_id {
            Some(id) if source.is_some() => {
                self.start_slot(app, slot, id, source.expect("source checked"));
            }
            _ => self.stop_slot(slot),
        }
    }

    pub fn start_slot(&self, app: &AppHandle, slot: &str, source_id: &str, source: CaptureSource) {
        {
            let workers = self.workers.lock().expect("preview workers lock");
            if let Some(worker) = workers.get(slot) {
                if worker.source_id == source_id {
                    return;
                }
            }
        }

        self.stop_slot(slot);

        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let stop_worker = stop.clone();
        let app = app.clone();
        let slot_name = slot.to_string();
        let source_id_owned = source_id.to_string();

        let join = thread::spawn(move || {
            let mut stream = match PreviewStream::open(&source) {
                Ok(stream) => stream,
                Err(error) => {
                    tracing::warn!(
                        slot = %slot_name,
                        source_id = %source_id_owned,
                        %error,
                        "failed to open preview stream"
                    );
                    return;
                }
            };

            let (sender, receiver) = std::sync::mpsc::sync_channel::<RgbaImage>(1);
            let stop_encode = stop.clone();
            let app_encode = app.clone();
            let slot_encode = slot_name.clone();

            let encode_join = thread::spawn(move || {
                while !stop_encode.load(Ordering::Relaxed) {
                    match receiver.recv_timeout(TARGET_FRAME_INTERVAL) {
                        Ok(image) => match sources::encode_preview(image) {
                            Ok(preview) => {
                                let payload = SlotPreviewFrame {
                                    slot: slot_encode.clone(),
                                    preview,
                                };

                                if let Err(error) = app_encode.emit("slot-preview-frame", payload) {
                                    tracing::warn!(
                                        slot = %slot_encode,
                                        %error,
                                        "failed to emit preview frame"
                                    );
                                }
                            }
                            Err(error) => {
                                tracing::debug!(
                                    slot = %slot_encode,
                                    %error,
                                    "preview encode failed"
                                );
                            }
                        },
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            });

            while !stop_flag.load(Ordering::Relaxed) {
                match stream.capture_rgba() {
                    Ok(rgba) => {
                        let _ = sender.try_send(rgba);
                    }
                    Err(error) => {
                        tracing::debug!(
                            slot = %slot_name,
                            %error,
                            "preview capture failed"
                        );
                        thread::sleep(Duration::from_millis(8));
                    }
                }
            }

            drop(sender);
            let _ = encode_join.join();
        });

        self.workers.lock().expect("preview workers lock").insert(
            slot.to_string(),
            SlotPreviewWorker {
                source_id: source_id.to_string(),
                stop: stop_worker,
                join,
            },
        );
    }

    pub fn stop_slot(&self, slot: &str) {
        let worker = self.workers.lock().expect("preview workers lock").remove(slot);

        if let Some(worker) = worker {
            worker.stop.store(true, Ordering::Relaxed);
            let _ = worker.join.join();
        }
    }
}

impl Default for PreviewManager {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) enum PreviewStream {
    Screen(Monitor),
    Webcam(nokhwa::Camera),
    #[cfg(feature = "ndi")]
    Ndi(super::ndi::NdiPreviewStream),
}

impl Drop for PreviewStream {
    fn drop(&mut self) {
        if let Self::Webcam(camera) = self {
            let _ = camera.stop_stream();
        }
    }
}

impl PreviewStream {
    pub(crate) fn open(source: &CaptureSource) -> Result<Self, CaptureError> {
        match source.kind {
            CaptureSourceKind::Screen => {
                let monitor_id = sources::parse_id_suffix(&source.id, "screen:")?;
                let monitor = find_monitor(monitor_id)?;
                Ok(Self::Screen(monitor))
            }
            CaptureSourceKind::Webcam => Ok(Self::Webcam(open_webcam(&source.id)?)),
            CaptureSourceKind::Ndi => {
                #[cfg(feature = "ndi")]
                {
                    return Ok(Self::Ndi(super::ndi::NdiPreviewStream::open(&source.id)?));
                }
                #[cfg(not(feature = "ndi"))]
                {
                    return Err(CaptureError::Unsupported(
                        "NDI support was not compiled into this build".into(),
                    ));
                }
            }
        }
    }

    pub(crate) fn capture_rgba(&mut self) -> Result<RgbaImage, CaptureError> {
        match self {
            Self::Screen(monitor) => monitor
                .capture_image()
                .map_err(|error| CaptureError::CaptureFailed(error.to_string())),
            Self::Webcam(camera) => capture_webcam_rgba(camera),
            #[cfg(feature = "ndi")]
            Self::Ndi(stream) => stream.capture_rgba(),
        }
    }
}

fn find_monitor(monitor_id: u32) -> Result<Monitor, CaptureError> {
    #[cfg(windows)]
    {
        let index = monitor_id as usize;
        return xcap::Monitor::all()
            .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?
            .into_iter()
            .nth(index.saturating_sub(1))
            .ok_or_else(|| CaptureError::SourceNotFound(format!("screen:{monitor_id}")));
    }

    #[cfg(not(windows))]
    {
        xcap::Monitor::all()
            .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?
            .into_iter()
            .find(|monitor| monitor.id() == monitor_id)
            .ok_or_else(|| CaptureError::SourceNotFound(format!("screen:{monitor_id}")))
    }
}

fn open_webcam(source_id: &str) -> Result<nokhwa::Camera, CaptureError> {
    use nokhwa::pixel_format::RgbFormat;
    use nokhwa::utils::{CameraFormat, FrameFormat, RequestedFormat, RequestedFormatType, Resolution};

    let index = sources::parse_id_suffix(source_id, "webcam:")? as usize;

    let cameras = nokhwa::query(nokhwa::utils::ApiBackend::Auto)
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    let camera_info = cameras
        .get(index)
        .ok_or_else(|| CaptureError::SourceNotFound(source_id.to_string()))?;

    let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(CameraFormat::new(
        Resolution::new(sources::PREVIEW_MAX_WIDTH, sources::PREVIEW_MAX_HEIGHT),
        FrameFormat::MJPEG,
        30,
    )));

    let mut camera = nokhwa::Camera::new(camera_info.index().clone(), requested)
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    camera
        .open_stream()
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    Ok(camera)
}

fn capture_webcam_rgba(camera: &mut nokhwa::Camera) -> Result<RgbaImage, CaptureError> {
    use nokhwa::pixel_format::RgbFormat;

    let frame = camera
        .frame()
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    let decoded = frame
        .decode_image::<RgbFormat>()
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    Ok(image::DynamicImage::ImageRgb8(decoded).to_rgba8())
}
