use std::io::Cursor;
use std::os::fd::OwnedFd;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, RecvTimeoutError, SyncSender},
    Arc, OnceLock,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType};
use ashpd::desktop::PersistMode;
use ashpd::WindowIdentifier;
use enumflags2::BitFlags;
use glib::object::Cast;
use gtk::prelude::WidgetExt;
use pipewire::{
    channel,
    context::ContextRc,
    keys::{MEDIA_CATEGORY, MEDIA_ROLE, MEDIA_TYPE},
    main_loop::MainLoopRc,
    properties,
    spa::{
        param::{
            ParamType,
            format::{FormatProperties, MediaSubtype, MediaType},
            format_utils,
            video::{VideoFormat, VideoInfoRaw},
        },
        pod::{self, serialize::PodSerializer, Pod},
        utils::{Direction, Fraction, Rectangle, SpaTypes},
    },
    stream::{StreamFlags, StreamRc},
};
use tauri::{AppHandle, Manager};
use xcap::{Frame, Monitor};

use super::frame_server::FrameSlot;
use super::sources::{self, CaptureError};
use super::types::CaptureSource;

const PORTAL_INIT_TIMEOUT: Duration = Duration::from_secs(120);
const FRAME_RECV_TIMEOUT: Duration = Duration::from_millis(250);

/// Pseudo source id shown on Wayland. The actual screen is chosen in the
/// system portal picker every time capture starts (OBS-style).
pub const PORTAL_SOURCE_ID: &str = "screen:portal";

pub struct LinuxPortalContext {
    pub window_identifier: Option<WindowIdentifier>,
}

pub(crate) fn find_monitor(monitor_id: u32) -> Result<Monitor, CaptureError> {
    Monitor::all()
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?
        .into_iter()
        .find(|monitor| monitor.id().map(|id| id == monitor_id).unwrap_or(false))
        .ok_or_else(|| CaptureError::SourceNotFound(format!("screen:{monitor_id}")))
}

pub fn portal_window_identifier_from_app(app: &AppHandle) -> Option<WindowIdentifier> {
    let window = app.get_webview_window("main")?;
    let gtk_window = window.gtk_window().ok()?;
    portal_window_identifier(&gtk_window)
}

pub fn portal_window_identifier(gtk_window: &gtk::ApplicationWindow) -> Option<WindowIdentifier> {
    let gdk_window = gtk_window.window()?;
    if let Some(x11_window) = gdk_window.downcast_ref::<gdkx11::X11Window>() {
        let xid = x11_window.xid();
        tracing::debug!(xid, "using X11 parent window for screen-cast portal");
        return Some(WindowIdentifier::from_xid(xid));
    }

    tracing::warn!(
        "could not resolve X11 window id for portal parent; approve the picker if it appears behind the app"
    );
    None
}

pub fn is_wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE")
        .map(|value| value.eq_ignore_ascii_case("wayland"))
        .unwrap_or(false)
        || std::env::var("WAYLAND_DISPLAY")
            .map(|value| !value.is_empty())
            .unwrap_or(false)
}

pub struct LinuxScreenCaptureSession {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl LinuxScreenCaptureSession {
    pub fn start(
        source: &CaptureSource,
        frame_slot: Arc<FrameSlot>,
        portal: LinuxPortalContext,
    ) -> Result<Self, CaptureError> {
        let monitor_id = if source.id == PORTAL_SOURCE_ID {
            None
        } else {
            Some(sources::parse_id_suffix(&source.id, "screen:")?)
        };
        let stop = Arc::new(AtomicBool::new(false));
        let (init_tx, init_rx) = mpsc::sync_channel(1);

        let stop_flag = stop.clone();
        let join = thread::spawn(move || {
            run_capture_thread(monitor_id, frame_slot, stop_flag, init_tx, portal);
        });

        match init_rx.recv_timeout(PORTAL_INIT_TIMEOUT) {
            Ok(Ok(())) => {}
            Ok(Err(message)) => {
                stop.store(true, Ordering::Relaxed);
                let _ = join.join();
                return Err(CaptureError::CaptureFailed(message));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                stop.store(true, Ordering::Relaxed);
                return Err(CaptureError::CaptureFailed(
                    "Timed out waiting for screen capture permission. Approve the system screen-share dialog, then try again.".into(),
                ));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = join.join();
                return Err(CaptureError::CaptureFailed(
                    "Screen capture setup failed before producing frames".into(),
                ));
            }
        }

        Ok(Self {
            stop,
            join: Some(join),
        })
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn run_capture_thread(
    monitor_id: Option<u32>,
    frame_slot: Arc<FrameSlot>,
    stop: Arc<AtomicBool>,
    init_tx: SyncSender<Result<(), String>>,
    portal: LinuxPortalContext,
) {
    match monitor_id {
        None => run_portal_capture(frame_slot, stop, init_tx, portal),
        Some(monitor_id) => run_xcap_capture(monitor_id, frame_slot, stop, init_tx),
    }
}

/// OBS-style Wayland capture: every capture opens its own portal session and the
/// user picks the screen in the system dialog. No monitor matching, no cached
/// sessions, no restore tokens.
fn run_portal_capture(
    frame_slot: Arc<FrameSlot>,
    stop: Arc<AtomicBool>,
    init_tx: SyncSender<Result<(), String>>,
    portal: LinuxPortalContext,
) {
    tracing::info!("starting portal + PipeWire screen capture (system picker)");

    let (node_id, pipewire_fd) = match open_portal_stream(portal.window_identifier) {
        Ok(result) => result,
        Err(error) => {
            let _ = init_tx.send(Err(error.to_string()));
            return;
        }
    };

    let (first_frame_tx, first_frame_rx) = mpsc::sync_channel(1);
    let stop_pw = stop.clone();
    let frame_slot_pw = frame_slot.clone();
    let pw_join = thread::spawn(move || {
        if let Err(error) =
            run_pipewire_loop(node_id, pipewire_fd, frame_slot_pw, stop_pw, first_frame_tx)
        {
            tracing::warn!(%error, "pipewire capture loop ended");
        }
    });

    match first_frame_rx.recv_timeout(PORTAL_INIT_TIMEOUT) {
        Ok(Ok(())) => {
            let _ = init_tx.send(Ok(()));
        }
        Ok(Err(message)) => {
            stop.store(true, Ordering::Relaxed);
            let _ = init_tx.send(Err(message));
            let _ = pw_join.join();
            return;
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            stop.store(true, Ordering::Relaxed);
            let _ = init_tx.send(Err(
                "PipeWire stream produced no frames after portal approval".into(),
            ));
            let _ = pw_join.join();
            return;
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            stop.store(true, Ordering::Relaxed);
            let _ = init_tx.send(Err("PipeWire capture thread exited early".into()));
            let _ = pw_join.join();
            return;
        }
    }

    let _ = pw_join.join();
}

fn run_xcap_capture(
    monitor_id: u32,
    frame_slot: Arc<FrameSlot>,
    stop: Arc<AtomicBool>,
    init_tx: SyncSender<Result<(), String>>,
) {
    let monitor = match find_monitor(monitor_id) {
        Ok(monitor) => monitor,
        Err(error) => {
            let _ = init_tx.send(Err(error.to_string()));
            return;
        }
    };
    let monitor_name = monitor.name().unwrap_or_else(|_| "unknown".into());

    tracing::info!(
        monitor_id,
        monitor_name = %monitor_name,
        "starting xcap screen capture (X11 session)"
    );

    let setup = (|| -> Result<(xcap::VideoRecorder, mpsc::Receiver<Frame>), CaptureError> {
        let (recorder, receiver) = monitor
            .video_recorder()
            .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;
        recorder
            .start()
            .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;
        Ok((recorder, receiver))
    })();

    let (recorder, receiver) = match setup {
        Ok(pair) => pair,
        Err(error) => {
            let _ = init_tx.send(Err(error.to_string()));
            return;
        }
    };

    let _ = init_tx.send(Ok(()));

    while !stop.load(Ordering::Relaxed) {
        match receiver.recv_timeout(FRAME_RECV_TIMEOUT) {
            Ok(frame) => frame_slot.publish(frame.width, frame.height, frame.raw),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                tracing::warn!("xcap screen recorder disconnected");
                break;
            }
        }
    }

    if let Err(error) = recorder.stop() {
        tracing::debug!(%error, "failed to stop xcap screen recorder");
    }
}

struct PortalRequest {
    window_identifier: Option<WindowIdentifier>,
    reply: SyncSender<Result<(u32, OwnedFd), CaptureError>>,
}

static PORTAL_WORKER: OnceLock<std::sync::Mutex<mpsc::Sender<PortalRequest>>> = OnceLock::new();

/// All portal D-Bus calls must run on one long-lived tokio runtime: ashpd
/// caches its zbus connection globally, and that connection's I/O task lives
/// on the runtime that first created it. Short-lived per-capture runtimes
/// would leave the cached connection dead and hang every later request.
fn portal_worker() -> &'static std::sync::Mutex<mpsc::Sender<PortalRequest>> {
    PORTAL_WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<PortalRequest>();
        thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    tracing::error!(%error, "failed to build portal worker runtime");
                    return;
                }
            };

            while let Ok(request) = rx.recv() {
                let result = runtime
                    .block_on(open_portal_stream_async(request.window_identifier.as_ref()));
                let _ = request.reply.send(result);
            }
        });
        std::sync::Mutex::new(tx)
    })
}

/// Open a fresh portal screencast session. Always shows the system picker and
/// captures whichever screen the user selects.
fn open_portal_stream(
    window_identifier: Option<WindowIdentifier>,
) -> Result<(u32, OwnedFd), CaptureError> {
    let (reply_tx, reply_rx) = mpsc::sync_channel(1);
    let request = PortalRequest {
        window_identifier,
        reply: reply_tx,
    };

    portal_worker()
        .lock()
        .map_err(|_| CaptureError::CaptureFailed("portal worker unavailable".into()))?
        .send(request)
        .map_err(|_| CaptureError::CaptureFailed("portal worker stopped".into()))?;

    reply_rx
        .recv_timeout(PORTAL_INIT_TIMEOUT)
        .map_err(|_| {
            CaptureError::CaptureFailed(
                "Timed out waiting for the screen selection dialog".into(),
            )
        })?
}

async fn open_portal_stream_async(
    window_identifier: Option<&WindowIdentifier>,
) -> Result<(u32, OwnedFd), CaptureError> {
    let proxy = Screencast::new()
            .await
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    let session = proxy
        .create_session()
        .await
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    proxy
        .select_sources(
            &session,
            CursorMode::Hidden,
            BitFlags::from(SourceType::Monitor),
            false,
            None,
            PersistMode::DoNot,
        )
        .await
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?
        .response()
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    tracing::info!("portal screen picker shown; choose the screen to share");

    let streams = proxy
        .start(&session, window_identifier)
        .await
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?
        .response()
        .map_err(|error| {
            CaptureError::CaptureFailed(format!("screen selection cancelled: {error}"))
        })?;

    let node_id = streams
        .streams()
        .first()
        .map(|stream| stream.pipe_wire_node_id())
        .ok_or_else(|| {
            CaptureError::CaptureFailed("portal returned no capture streams".into())
        })?;

    let pipewire_fd = proxy
        .open_pipe_wire_remote(&session)
        .await
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    tracing::info!(node_id, "portal screencast stream selected");

    Ok((node_id, pipewire_fd))
}

struct ListenerUserData {
    format: VideoInfoRaw,
    format_ready: AtomicBool,
}

fn run_pipewire_loop(
    stream_id: u32,
    pipewire_fd: OwnedFd,
    frame_slot: Arc<FrameSlot>,
    stop: Arc<AtomicBool>,
    first_frame_tx: SyncSender<Result<(), String>>,
) -> Result<(), CaptureError> {
    pipewire::init();

    tracing::debug!(stream_id, "connecting pipewire stream to portal node via remote fd");

    let main_loop = MainLoopRc::new(None)
        .map_err(|error| CaptureError::CaptureFailed(format!("pipewire main loop: {error}")))?;
    let context = ContextRc::new(&main_loop, None)
        .map_err(|error| CaptureError::CaptureFailed(format!("pipewire context: {error}")))?;
    let core = context
        .connect_fd_rc(pipewire_fd, None)
        .map_err(|error| CaptureError::CaptureFailed(format!("pipewire portal connect: {error}")))?;

    let user_data = ListenerUserData {
        format: VideoInfoRaw::default(),
        format_ready: AtomicBool::new(false),
    };

    let stream = StreamRc::new(
        core.clone(),
        "tandem-screen-capture",
        properties::properties! {
            *MEDIA_TYPE => "Video",
            *MEDIA_CATEGORY => "Capture",
            *MEDIA_ROLE => "Screen",
        },
    )
    .map_err(|error| CaptureError::CaptureFailed(format!("pipewire stream create: {error}")))?;

    let stop_flag = stop.clone();
    let first_frame_flag = Arc::new(AtomicBool::new(false));
    let first_frame_flag_cb = first_frame_flag.clone();
    let first_frame_tx_cb = first_frame_tx.clone();
    let _listener = stream
        .add_local_listener_with_user_data(user_data)
        .param_changed(|_, user_data, id, param| {
            let Some(param) = param else {
                return;
            };
            if id != ParamType::Format.as_raw() {
                return;
            }
            let Ok((media_type, media_subtype)) = format_utils::parse_format(param) else {
                return;
            };
            if media_type != MediaType::Video || media_subtype != MediaSubtype::Raw {
                return;
            }
            if user_data.format.parse(param).is_ok() {
                user_data.format_ready.store(true, Ordering::Relaxed);
                let size = user_data.format.size();
                tracing::debug!(
                    width = size.width,
                    height = size.height,
                    format = ?user_data.format.format(),
                    "pipewire capture format negotiated"
                );
            }
        })
        .process(move |stream, user_data| {
            if stop_flag.load(Ordering::Relaxed) || !user_data.format_ready.load(Ordering::Relaxed) {
                return;
            }
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            if datas.is_empty() {
                return;
            }
            let size = user_data.format.size();
            if size.width == 0 || size.height == 0 {
                return;
            }
            let stride = datas[0].chunk().stride() as usize;
            let Some(frame_data) = datas[0].data() else {
                return;
            };

            let Some(raw) = frame_to_rgba(frame_data, stride, &user_data.format) else {
                return;
            };

            frame_slot.publish(size.width, size.height, raw);

            if !first_frame_flag_cb.swap(true, Ordering::Relaxed) {
                tracing::info!(stream_id, width = size.width, height = size.height, "pipewire capture producing frames");
                let _ = first_frame_tx_cb.send(Ok(()));
            }
        })
        .register()
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    let obj = pod::object!(
        SpaTypes::ObjectParamFormat,
        ParamType::EnumFormat,
        pod::property!(FormatProperties::MediaType, Id, MediaType::Video),
        pod::property!(FormatProperties::MediaSubtype, Id, MediaSubtype::Raw),
        pod::property!(
            FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            VideoFormat::RGB,
            VideoFormat::RGBA,
            VideoFormat::RGBx,
            VideoFormat::BGRx,
        ),
        pod::property!(
            FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            Rectangle { width: 128, height: 128 },
            Rectangle { width: 1, height: 1 },
            Rectangle { width: 4096, height: 4096 }
        ),
        pod::property!(
            FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            Fraction { num: 24, denom: 1 },
            Fraction { num: 0, denom: 1 },
            Fraction { num: 60, denom: 1 }
        ),
    );

    let values = PodSerializer::serialize(Cursor::new(Vec::new()), &pod::Value::Object(obj))
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?
        .0
        .into_inner();

    let mut params = [Pod::from_bytes(&values).ok_or_else(|| {
        CaptureError::CaptureFailed("failed to build pipewire format pod".into())
    })?];

    stream
        .connect(
            Direction::Input,
            Some(stream_id),
            StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS,
            &mut params,
        )
        .map_err(|error| {
            CaptureError::CaptureFailed(format!(
                "pipewire stream connect to portal node {stream_id}: {error}"
            ))
        })?;

    let (active_sender, active_receiver) = channel::channel::<bool>();
    let _attached = active_receiver.attach(main_loop.loop_(), move |active| {
        if let Err(error) = stream.set_active(active) {
            tracing::debug!(%error, "failed to set pipewire stream active");
        }
    });
    let _ = active_sender.send(true);

    while !stop.load(Ordering::Relaxed) {
        main_loop.loop_().iterate(Duration::ZERO);
        thread::sleep(Duration::from_millis(1));
    }

    let _ = active_sender.send(false);
    Ok(())
}

fn frame_to_rgba(frame_data: &[u8], stride: usize, format: &VideoInfoRaw) -> Option<Vec<u8>> {
    let width = format.size().width as usize;
    let height = format.size().height as usize;
    if width == 0 || height == 0 {
        return None;
    }

    let pixel_count = width.checked_mul(height)?;
    let mut rgba = vec![0_u8; pixel_count.checked_mul(4)?];

    match format.format() {
        VideoFormat::RGBA => copy_rows(frame_data, stride, width, height, 4, &mut rgba, 4, |src, dst| {
            dst.copy_from_slice(src);
        })?,
        VideoFormat::RGBx => copy_rows(frame_data, stride, width, height, 4, &mut rgba, 4, |src, dst| {
            dst[0] = src[0];
            dst[1] = src[1];
            dst[2] = src[2];
            dst[3] = 255;
        })?,
        VideoFormat::BGRx => copy_rows(frame_data, stride, width, height, 4, &mut rgba, 4, |src, dst| {
            dst[0] = src[2];
            dst[1] = src[1];
            dst[2] = src[0];
            dst[3] = 255;
        })?,
        VideoFormat::RGB => copy_rows(frame_data, stride, width, height, 3, &mut rgba, 4, |src, dst| {
            dst[0] = src[0];
            dst[1] = src[1];
            dst[2] = src[2];
            dst[3] = 255;
        })?,
        other => {
            tracing::debug!(?other, "unsupported pipewire pixel format");
            return None;
        }
    }

    Some(rgba)
}

fn copy_rows(
    src: &[u8],
    src_stride: usize,
    width: usize,
    height: usize,
    src_bpp: usize,
    dst: &mut [u8],
    dst_bpp: usize,
    mut copy_pixel: impl FnMut(&[u8], &mut [u8]),
) -> Option<()> {
    let row_bytes = width.checked_mul(src_bpp)?;
    let effective_stride = src_stride.max(row_bytes);
    let dst_row_bytes = width.checked_mul(dst_bpp)?;

    for row in 0..height {
        let src_start = row.checked_mul(effective_stride)?;
        let src_row = src.get(src_start..src_start + row_bytes)?;
        let dst_start = row.checked_mul(dst_row_bytes)?;
        let dst_row = dst.get_mut(dst_start..dst_start + dst_row_bytes)?;

        for (src_px, dst_px) in src_row.chunks_exact(src_bpp).zip(dst_row.chunks_exact_mut(dst_bpp)) {
            copy_pixel(src_px, dst_px);
        }
    }

    Some(())
}
