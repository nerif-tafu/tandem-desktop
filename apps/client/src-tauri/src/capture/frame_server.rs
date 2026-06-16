use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::net::TcpListener;
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        handshake::server::{Request, Response},
        Message,
    },
};

#[derive(Clone)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub pixels: Arc<Vec<u8>>,
    pub generation: u64,
}

/// Cap frame fan-out to the local websocket consumers (~30fps) to limit memory churn.
const MIN_PUBLISH_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Debug, Clone, Serialize)]
pub struct SlotFrameStats {
    pub published: u64,
    pub publish_throttled: u64,
    pub websocket_sent: u64,
    pub websocket_skipped: u64,
    pub latest_width: u32,
    pub latest_height: u32,
    pub latest_bytes: usize,
}

pub struct FrameSlot {
    latest: Mutex<Option<CapturedFrame>>,
    generation: AtomicU64,
    last_publish: Mutex<Option<Instant>>,
    published: AtomicU64,
    publish_throttled: AtomicU64,
    websocket_sent: AtomicU64,
    websocket_skipped: AtomicU64,
}

impl Default for FrameSlot {
    fn default() -> Self {
        Self {
            latest: Mutex::new(None),
            generation: AtomicU64::new(0),
            last_publish: Mutex::new(None),
            published: AtomicU64::new(0),
            publish_throttled: AtomicU64::new(0),
            websocket_sent: AtomicU64::new(0),
            websocket_skipped: AtomicU64::new(0),
        }
    }
}

impl FrameSlot {
    pub fn publish(&self, width: u32, height: u32, pixels: Vec<u8>) {
        let mut last_publish = self.last_publish.lock().expect("frame slot publish lock");
        let now = Instant::now();
        if last_publish
            .map(|instant| {
                now.checked_duration_since(instant)
                    .is_some_and(|elapsed| elapsed < MIN_PUBLISH_INTERVAL)
            })
            .unwrap_or(false)
        {
            self.publish_throttled.fetch_add(1, Ordering::Relaxed);
            return;
        }
        *last_publish = Some(now);

        let generation = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
        self.published.fetch_add(1, Ordering::Relaxed);
        *self.latest.lock().expect("frame slot lock") = Some(CapturedFrame {
            width,
            height,
            pixels: Arc::new(pixels),
            generation,
        });
    }

    pub fn has_frame(&self) -> bool {
        self.latest.lock().expect("frame slot lock").is_some()
    }

    pub fn latest_frame(&self) -> Option<CapturedFrame> {
        self.latest.lock().expect("frame slot lock").clone()
    }

    pub fn clear(&self) {
        *self.latest.lock().expect("frame slot lock") = None;
        *self.last_publish.lock().expect("frame slot publish lock") = None;
    }

    pub fn stats(&self) -> SlotFrameStats {
        let latest = self.latest.lock().expect("frame slot lock");
        let (latest_width, latest_height, latest_bytes) = latest
            .as_ref()
            .map(|frame| (frame.width, frame.height, frame.pixels.len()))
            .unwrap_or((0, 0, 0));

        SlotFrameStats {
            published: self.published.load(Ordering::Relaxed),
            publish_throttled: self.publish_throttled.load(Ordering::Relaxed),
            websocket_sent: self.websocket_sent.load(Ordering::Relaxed),
            websocket_skipped: self.websocket_skipped.load(Ordering::Relaxed),
            latest_width,
            latest_height,
            latest_bytes,
        }
    }
}

type SlotMap = Arc<Mutex<HashMap<String, Arc<FrameSlot>>>>;

pub struct FrameServer {
    port: u16,
    slots: SlotMap,
    _server_thread: JoinHandle<()>,
}

impl FrameServer {
    pub fn start() -> Result<Self, String> {
        let slots: SlotMap = Arc::new(Mutex::new(HashMap::new()));
        let slots_for_server = slots.clone();
        let (port_tx, port_rx) = std::sync::mpsc::sync_channel::<Result<u16, String>>(1);

        let server_thread = thread::spawn(move || {
            #[cfg(windows)]
            super::windows_performance::configure_high_priority_worker_thread();

            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(2)
                .build()
                .expect("frame server tokio runtime");

            let result = runtime.block_on(run_server(slots_for_server, port_tx));
            if let Err(error) = result {
                tracing::error!(%error, "frame websocket server exited");
            }
        });

        let port = port_rx
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| "Timed out starting frame websocket server".to_string())?
            .map_err(|error| error)?;

        Ok(Self {
            port,
            slots,
            _server_thread: server_thread,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn register_slot(&self, slot: &str) -> Arc<FrameSlot> {
        let frame_slot = Arc::new(FrameSlot::default());
        self.slots
            .lock()
            .expect("frame slots lock")
            .insert(slot.to_string(), frame_slot.clone());
        frame_slot
    }

    pub fn unregister_slot(&self, slot: &str) {
        if let Some(frame_slot) = self.slots.lock().expect("frame slots lock").remove(slot) {
            frame_slot.clear();
        }
    }

    pub fn socket_url(&self, slot: &str) -> String {
        format!("ws://127.0.0.1:{}/ws/{}", self.port, slot)
    }

    pub fn slot_stats(&self) -> HashMap<String, SlotFrameStats> {
        self.slots
            .lock()
            .expect("frame slots lock")
            .iter()
            .map(|(slot, frame_slot)| (slot.clone(), frame_slot.stats()))
            .collect()
    }
}

async fn run_server(
    slots: SlotMap,
    port_tx: std::sync::mpsc::SyncSender<Result<u16, String>>,
) -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| error.to_string())?;
    let port = listener.local_addr().map_err(|error| error.to_string())?.port();
    let _ = port_tx.send(Ok(port));

    loop {
        let (stream, _) = listener.accept().await.map_err(|error| error.to_string())?;
        let slots = slots.clone();

        tokio::spawn(async move {
            if let Err(error) = handle_client(stream, slots).await {
                tracing::debug!(%error, "frame websocket client disconnected");
            }
        });
    }
}

async fn handle_client(stream: tokio::net::TcpStream, slots: SlotMap) -> Result<(), String> {
    let mut requested_slot: Option<String> = None;

    let websocket = accept_hdr_async(stream, |request: &Request, response: Response| {
        requested_slot = request
            .uri()
            .path()
            .strip_prefix("/ws/")
            .map(|slot| slot.to_string());

        Ok(response)
    })
    .await
    .map_err(|error| error.to_string())?;

    let slot_name = requested_slot.ok_or_else(|| "Missing websocket slot path".to_string())?;
    let frame_slot = slots
        .lock()
        .expect("frame slots lock")
        .get(&slot_name)
        .cloned()
        .ok_or_else(|| format!("Unknown frame slot: {slot_name}"))?;

    let (mut sink, mut stream) = websocket.split();
    let mut last_generation = 0_u64;

    loop {
        tokio::select! {
            incoming = stream.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(error)) => return Err(error.to_string()),
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(16)) => {
                let frame = frame_slot.latest_frame();
                let Some(frame) = frame else {
                    continue;
                };

                if frame.generation == last_generation {
                    frame_slot.websocket_skipped.fetch_add(1, Ordering::Relaxed);
                    continue;
                }

                last_generation = frame.generation;
                let mut packet = Vec::with_capacity(8 + frame.pixels.len());
                packet.extend_from_slice(&frame.width.to_le_bytes());
                packet.extend_from_slice(&frame.height.to_le_bytes());
                packet.extend_from_slice(frame.pixels.as_ref());

                sink.send(Message::Binary(packet.into()))
                    .await
                    .map_err(|error| error.to_string())?;
                frame_slot.websocket_sent.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    Ok(())
}
