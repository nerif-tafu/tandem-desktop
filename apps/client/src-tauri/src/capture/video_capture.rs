use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use super::frame_server::{FrameServer, FrameSlot};
use super::sources::CaptureError;
use super::types::{CaptureSource, CaptureSourceKind};
#[cfg(windows)]
use super::windows_video::WindowsCaptureSession;

const STOP_JOIN_TIMEOUT: Duration = Duration::from_secs(3);

struct SlotCaptureWorker {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    #[cfg(windows)]
    windows_capture: Option<WindowsCaptureSession>,
}

impl SlotCaptureWorker {
    fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);

        #[cfg(windows)]
        if let Some(session) = self.windows_capture.take() {
            session.stop();
        }

        if let Some(join) = self.join.take() {
            join_with_timeout(join, STOP_JOIN_TIMEOUT);
        }
    }
}

pub struct VideoCaptureManager {
    server: FrameServer,
    workers: Mutex<HashMap<String, SlotCaptureWorker>>,
}

impl VideoCaptureManager {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            server: FrameServer::start()?,
            workers: Mutex::new(HashMap::new()),
        })
    }

    pub fn server_port(&self) -> u16 {
        self.server.port()
    }

    pub fn start_slot(&self, slot: &str, source: &CaptureSource) -> Result<String, String> {
        self.stop_slot(slot);

        let frame_slot = self.server.register_slot(slot);
        let stop = Arc::new(AtomicBool::new(false));
        let source = source.clone();
        let slot_name = slot.to_string();
        let first_frame_slot = frame_slot.clone();

        let worker = match source.kind {
            #[cfg(windows)]
            CaptureSourceKind::Screen => {
                let session =
                    WindowsCaptureSession::start(&source, frame_slot).map_err(|error| error.to_string())?;

                SlotCaptureWorker {
                    stop,
                    join: None,
                    windows_capture: Some(session),
                }
            }
            #[cfg(not(windows))]
            CaptureSourceKind::Screen => {
                spawn_threaded_capture(&source, frame_slot, stop.clone(), slot_name)
            }
            CaptureSourceKind::Ndi | CaptureSourceKind::Webcam => {
                if matches!(source.kind, CaptureSourceKind::Webcam) {
                    return Err(format!("Slot \"{slot}\" uses browser webcam capture"));
                }

                spawn_threaded_capture(&source, frame_slot, stop.clone(), slot_name)
            }
        };

        self.workers
            .lock()
            .expect("video workers lock")
            .insert(slot.to_string(), worker);

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if first_frame_slot.has_frame() {
                return Ok(self.server.socket_url(slot));
            }

            thread::sleep(Duration::from_millis(16));
        }

        self.stop_slot(slot);
        Err(format!(
            "Capture for slot \"{slot}\" produced no frames. Check that the source is available."
        ))
    }

    pub fn stop_slot(&self, slot: &str) {
        let worker = self.workers.lock().expect("video workers lock").remove(slot);

        if let Some(worker) = worker {
            worker.stop();
        }

        self.server.unregister_slot(slot);
    }
}

fn spawn_threaded_capture(
    source: &CaptureSource,
    frame_slot: Arc<FrameSlot>,
    stop: Arc<AtomicBool>,
    slot_name: String,
) -> SlotCaptureWorker {
    let source = source.clone();
    let stop_flag = stop.clone();
    let join = thread::spawn(move || {
        let result = match source.kind {
            CaptureSourceKind::Screen => capture_source_xcap_loop(&source, frame_slot, stop_flag),
            CaptureSourceKind::Ndi => capture_source_ndi_loop(&source, frame_slot, stop_flag),
            CaptureSourceKind::Webcam => Ok(()),
        };

        if let Err(error) = result {
            tracing::warn!(slot = %slot_name, %error, "slot video capture ended with error");
        }
    });

    SlotCaptureWorker {
        stop,
        join: Some(join),
        #[cfg(windows)]
        windows_capture: None,
    }
}

fn join_with_timeout(join: JoinHandle<()>, timeout: Duration) {
    let (tx, rx) = mpsc::sync_channel::<()>(1);

    thread::spawn(move || {
        let _ = join.join();
        let _ = tx.send(());
    });

    match rx.recv_timeout(timeout) {
        Ok(()) => {}
        Err(mpsc::RecvTimeoutError::Timeout) => {
            tracing::warn!("timed out waiting for capture worker to stop");
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {}
    }
}

fn publish_rgba_image(frame_slot: &FrameSlot, rgba: image::RgbaImage) {
    let width = rgba.width();
    let height = rgba.height();
    frame_slot.publish(width, height, rgba.into_raw());
}

fn capture_source_xcap_loop(
    source: &CaptureSource,
    frame_slot: Arc<FrameSlot>,
    stop: Arc<AtomicBool>,
) -> Result<(), CaptureError> {
    let mut stream = super::preview::PreviewStream::open(source)?;

    while !stop.load(Ordering::Relaxed) {
        let frame_start = std::time::Instant::now();

        match stream.capture_rgba() {
            Ok(rgba) => publish_rgba_image(&frame_slot, rgba),
            Err(error) => tracing::debug!(%error, "xcap capture failed"),
        }

        let elapsed = frame_start.elapsed();
        if elapsed < Duration::from_micros(16_666) {
            thread::sleep(Duration::from_micros(16_666) - elapsed);
        }
    }

    Ok(())
}

#[cfg(feature = "ndi")]
fn capture_source_ndi_loop(
    source: &CaptureSource,
    frame_slot: Arc<FrameSlot>,
    stop: Arc<AtomicBool>,
) -> Result<(), CaptureError> {
    let mut stream = super::ndi::NdiPreviewStream::open(&source.id)?;

    while !stop.load(Ordering::Relaxed) {
        match stream.capture_rgba() {
            Ok(rgba) => publish_rgba_image(&frame_slot, rgba),
            Err(error) => tracing::debug!(%error, "ndi capture failed"),
        }
    }

    Ok(())
}

#[cfg(not(feature = "ndi"))]
fn capture_source_ndi_loop(
    _source: &CaptureSource,
    _frame_slot: Arc<FrameSlot>,
    _stop: Arc<AtomicBool>,
) -> Result<(), CaptureError> {
    Err(CaptureError::Unsupported(
        "NDI support was not compiled into this build".into(),
    ))
}
