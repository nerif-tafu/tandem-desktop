use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, RecvTimeoutError, SyncSender},
    Arc,
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
const RESTORE_TOKEN_FILE: &str = "portal-screencast-restore-token";

pub struct LinuxPortalContext {
    pub window_identifier: Option<WindowIdentifier>,
    pub config_dir: Option<PathBuf>,
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
        let monitor_id = sources::parse_id_suffix(&source.id, "screen:")?;
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
    monitor_id: u32,
    frame_slot: Arc<FrameSlot>,
    stop: Arc<AtomicBool>,
    init_tx: SyncSender<Result<(), String>>,
    portal: LinuxPortalContext,
) {
    let monitor = match find_monitor(monitor_id) {
        Ok(monitor) => monitor,
        Err(error) => {
            let _ = init_tx.send(Err(error.to_string()));
            return;
        }
    };
    let monitor_name = monitor.name().unwrap_or_else(|_| "unknown".into());

    if is_wayland_session() {
        tracing::info!(
            monitor_id,
            monitor_name = %monitor_name,
            "starting portal + PipeWire screen capture"
        );

        let config_dir = portal.config_dir.clone();
        let portal_result =
            open_portal_stream(portal.window_identifier.as_ref(), config_dir.clone());
        let (stream_id, restore_token) = match portal_result {
            Ok(result) => result,
            Err(error) => {
                let _ = init_tx.send(Err(error.to_string()));
                return;
            }
        };

        if let Some(token) = restore_token {
            save_restore_token(config_dir.as_deref(), &token);
        }

        let (first_frame_tx, first_frame_rx) = mpsc::sync_channel(1);
        let stop_pw = stop.clone();
        let frame_slot_pw = frame_slot.clone();
        let pw_join = thread::spawn(move || {
            if let Err(error) = run_pipewire_loop(stream_id, frame_slot_pw, stop_pw, first_frame_tx) {
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
        return;
    }

    tracing::info!(
        monitor_id,
        monitor_name = %monitor_name,
        "starting xcap PipeWire screen capture (X11 session)"
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

fn open_portal_stream(
    window_identifier: Option<&WindowIdentifier>,
    config_dir: Option<PathBuf>,
) -> Result<(u32, Option<String>), CaptureError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    runtime.block_on(async {
        let proxy = Screencast::new()
            .await
            .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

        let session = proxy
            .create_session()
            .await
            .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

        let restore_token = load_restore_token(config_dir.as_deref());

        proxy
            .select_sources(
                &session,
                CursorMode::Hidden,
                BitFlags::from(SourceType::Monitor),
                false,
                restore_token.as_deref(),
                PersistMode::Application,
            )
            .await
            .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?
            .response()
            .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

        tracing::info!("portal screen-share picker shown; select a monitor and confirm");

        let streams = proxy
            .start(&session, window_identifier)
            .await
            .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?
            .response()
            .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

        let stream = streams.streams().first().ok_or_else(|| {
            CaptureError::CaptureFailed("portal returned no capture streams".into())
        })?;

        Ok((stream.pipe_wire_node_id(), streams.restore_token().map(str::to_string)))
    })
}

struct ListenerUserData {
    format: VideoInfoRaw,
}

fn run_pipewire_loop(
    stream_id: u32,
    frame_slot: Arc<FrameSlot>,
    stop: Arc<AtomicBool>,
    first_frame_tx: SyncSender<Result<(), String>>,
) -> Result<(), CaptureError> {
    pipewire::init();

    let main_loop =
        MainLoopRc::new(None).map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;
    let context =
        ContextRc::new(&main_loop, None).map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;
    let core = context
        .connect_rc(None)
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    let user_data = ListenerUserData {
        format: VideoInfoRaw::default(),
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
    .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

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
            let _ = user_data.format.parse(param);
        })
        .process(move |stream, user_data| {
            if stop_flag.load(Ordering::Relaxed) {
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
            let Some(frame_data) = datas[0].data() else {
                return;
            };

            let raw = match user_data.format.format() {
                VideoFormat::RGB => {
                    let mut buf = vec![0; (size.width * size.height * 4) as usize];
                    for (src, dst) in frame_data.chunks_exact(3).zip(buf.chunks_exact_mut(4)) {
                        dst[0] = src[0];
                        dst[1] = src[1];
                        dst[2] = src[2];
                        dst[3] = 255;
                    }
                    buf
                }
                VideoFormat::RGBA => frame_data.to_vec(),
                VideoFormat::RGBx => frame_data.to_vec(),
                VideoFormat::BGRx => {
                    let mut buf = frame_data.to_vec();
                    for px in buf.chunks_exact_mut(4) {
                        px.swap(0, 2);
                    }
                    buf
                }
                other => {
                    tracing::debug!(?other, "unsupported pipewire pixel format");
                    return;
                }
            };

            frame_slot.publish(size.width, size.height, raw);

            if !first_frame_flag_cb.swap(true, Ordering::Relaxed) {
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
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

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

fn load_restore_token(config_dir: Option<&std::path::Path>) -> Option<String> {
    let path = config_dir?.join(RESTORE_TOKEN_FILE);
    let token = fs::read_to_string(path).ok()?;
    let token = token.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn save_restore_token(config_dir: Option<&std::path::Path>, token: &str) {
    let Some(config_dir) = config_dir else {
        return;
    };
    if let Err(error) = fs::create_dir_all(config_dir) {
        tracing::warn!(%error, "failed to create config dir for portal restore token");
        return;
    }
    let path = config_dir.join(RESTORE_TOKEN_FILE);
    if let Err(error) = fs::write(path, token) {
        tracing::warn!(%error, "failed to persist portal restore token");
    }
}
