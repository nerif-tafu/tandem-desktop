import readline from 'node:readline';
import { setTimeout as delay } from 'node:timers/promises';
import {
  LocalVideoTrack,
  Room,
  TrackPublishOptions,
  TrackSource,
  VideoBufferType,
  VideoFrame,
  VideoSource,
  dispose,
} from '@livekit/rtc-node';

const LOG_PREFIX = '[livekit-sidecar]';
const FRAME_SOCKET_RETRY_MS = 1000;

function publishOptions() {
  const options = new TrackPublishOptions();
  options.source = TrackSource.SOURCE_UNKNOWN;
  options.simulcast = true;
  return options;
}

function log(message, extra) {
  const suffix = extra === undefined ? '' : ` ${JSON.stringify(extra)}`;
  process.stderr.write(`${LOG_PREFIX} ${message}${suffix}\n`);
}

function toArrayBuffer(buffer) {
  if (buffer instanceof ArrayBuffer) {
    return buffer;
  }

  if (ArrayBuffer.isView(buffer)) {
    return buffer.buffer.slice(buffer.byteOffset, buffer.byteOffset + buffer.byteLength);
  }

  if (typeof Buffer !== 'undefined' && Buffer.isBuffer(buffer)) {
    return buffer.buffer.slice(buffer.byteOffset, buffer.byteOffset + buffer.byteLength);
  }

  return null;
}

class SlotPublisher {
  constructor(name) {
    this.name = name;
    this.wsUrl = null;
    this.ws = null;
    this.room = null;
    this.reconnectTimer = null;
    this.source = null;
    this.track = null;
    this.publication = null;
    this.width = 0;
    this.height = 0;
    this.active = false;
    this.pending = false;
  }

  async start(wsUrl, room) {
    if (this.wsUrl === wsUrl && this.active) {
      return;
    }

    await this.stop(room);
    this.active = true;
    this.wsUrl = wsUrl;
    this.room = room;
    this.connectSocket();
  }

  connectSocket() {
    if (!this.active || !this.wsUrl) {
      return;
    }

    const wsUrl = this.wsUrl;
    const ws = new WebSocket(wsUrl);
    ws.binaryType = 'arraybuffer';
    this.ws = ws;

    ws.addEventListener('open', () => {
      log('frame socket connected', { slot: this.name, wsUrl });
    });

    ws.addEventListener('error', () => {
      log('frame socket error', { slot: this.name, wsUrl });
    });

    ws.addEventListener('close', () => {
      if (this.ws !== ws) {
        return;
      }

      this.ws = null;

      if (this.active && !shuttingDown) {
        log('frame socket closed; retrying', { slot: this.name });
        this.reconnectTimer = setTimeout(() => {
          this.reconnectTimer = null;
          this.connectSocket();
        }, FRAME_SOCKET_RETRY_MS);
      } else {
        log('frame socket closed', { slot: this.name });
      }
    });

    ws.addEventListener('message', (event) => {
      void this.onFrame(event.data, this.room);
    });
  }

  async onFrame(buffer, room) {
    if (!this.active) {
      return;
    }

    const arrayBuffer = toArrayBuffer(buffer);
    if (!arrayBuffer || arrayBuffer.byteLength < 8) {
      return;
    }

    const view = new DataView(arrayBuffer);
    const width = view.getUint32(0, true);
    const height = view.getUint32(4, true);
    const expectedLength = width * height * 4;

    if (width === 0 || height === 0 || arrayBuffer.byteLength - 8 !== expectedLength) {
      return;
    }

    if (this.pending) {
      return;
    }

    this.pending = true;

    try {
      if (!this.source || width !== this.width || height !== this.height) {
        await this.republish(width, height, room);
      }

      if (!this.source) {
        return;
      }

      const pixels = new Uint8Array(arrayBuffer, 8, expectedLength);
      const frame = new VideoFrame(pixels, width, height, VideoBufferType.RGBA);
      this.source.captureFrame(frame, BigInt(Math.round(performance.now() * 1000)));
    } catch (error) {
      log('failed to publish frame', {
        slot: this.name,
        error: error instanceof Error ? error.message : String(error),
      });
    } finally {
      this.pending = false;
    }
  }

  async republish(width, height, room) {
    await this.unpublish(room);

    this.width = width;
    this.height = height;
    this.source = new VideoSource(width, height);
    this.track = LocalVideoTrack.createVideoTrack(this.name, this.source);
    this.publication = await room.localParticipant.publishTrack(this.track, publishOptions());
    log('published track', { slot: this.name, width, height });
  }

  async unpublish(room) {
    if (this.publication && this.track) {
      try {
        const sid = this.publication.sid ?? this.track.sid;
        if (sid) {
          await room.localParticipant.unpublishTrack(sid, true);
        }
      } catch (error) {
        log('failed to unpublish track', {
          slot: this.name,
          error: error instanceof Error ? error.message : String(error),
        });
      }
    }

    if (this.track) {
      try {
        await this.track.close(true);
      } catch {
        // ignore cleanup errors
      }
    }

    this.publication = null;
    this.track = null;
    this.source = null;
    this.width = 0;
    this.height = 0;
  }

  async stop(room) {
    this.active = false;

    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    if (this.ws) {
      try {
        this.ws.close();
      } catch {
        // ignore
      }
      this.ws = null;
    }

    await this.unpublish(room);
    this.wsUrl = null;
  }
}

const room = new Room();
const publishers = new Map();
let connected = false;
let shuttingDown = false;
let pendingSlots = null;
let messageChain = Promise.resolve();

async function syncSlots(slots) {
  if (!connected) {
    pendingSlots = slots;
    log('queued slot sync until room connect', { slotCount: slots.length });
    return;
  }

  const desired = new Map(slots.map((entry) => [entry.slot, entry.wsUrl]));

  for (const [name, publisher] of [...publishers.entries()]) {
    if (!desired.has(name)) {
      await publisher.stop(room);
      publishers.delete(name);
      log('removed slot publisher', { slot: name });
    }
  }

  for (const [name, wsUrl] of desired.entries()) {
    let publisher = publishers.get(name);
    if (!publisher) {
      publisher = new SlotPublisher(name);
      publishers.set(name, publisher);
    }

    publisher.active = true;
    await publisher.start(wsUrl, room);
  }
}

async function startRoom(url, token) {
  if (connected) {
    return;
  }

  log('connecting to livekit', { url });
  await room.connect(url, token, { autoSubscribe: false, dynacast: true });
  connected = true;
  log('connected to livekit', { room: room.name });

  if (pendingSlots) {
    const slots = pendingSlots;
    pendingSlots = null;
    await syncSlots(slots);
  }
}

async function shutdown() {
  if (shuttingDown) {
    return;
  }

  shuttingDown = true;
  log('shutting down');

  for (const publisher of publishers.values()) {
    await publisher.stop(room);
  }
  publishers.clear();

  if (connected) {
    try {
      await room.disconnect();
    } catch {
      // ignore
    }
    connected = false;
  }

  try {
    await dispose();
  } catch {
    // ignore
  }
}

async function handleMessage(message) {
  switch (message.type) {
    case 'start':
      await startRoom(message.url, message.token);
      return;
    case 'sync':
      await syncSlots(message.slots ?? []);
      return;
    default:
      log('unknown message type', { type: message.type });
  }
}

function enqueueMessage(message) {
  messageChain = messageChain
    .then(() => handleMessage(message))
    .catch((error) => {
      log('failed to handle message', {
        type: message.type,
        error: error instanceof Error ? error.message : String(error),
      });
    });
}

function parseInitialConfig() {
  const url = process.env.LIVEKIT_URL;
  const token = process.env.LIVEKIT_TOKEN;
  if (url && token) {
    return { type: 'start', url, token };
  }

  const arg = process.argv[2];
  if (arg) {
    return JSON.parse(arg);
  }

  return null;
}

const initial = parseInitialConfig();
if (initial) {
  enqueueMessage(initial);
}

const rl = readline.createInterface({
  input: process.stdin,
  crlfDelay: Infinity,
});

rl.on('line', (line) => {
  const trimmed = line.trim();
  if (!trimmed) {
    return;
  }

  try {
    enqueueMessage(JSON.parse(trimmed));
  } catch (error) {
    log('failed to parse stdin message', {
      error: error instanceof Error ? error.message : String(error),
    });
  }
});

rl.on('close', () => {
  void shutdown().finally(() => {
    process.exit(process.exitCode ?? 0);
  });
});

for (const signal of ['SIGTERM', 'SIGINT']) {
  process.on(signal, () => {
    void shutdown().finally(() => {
      process.exit(0);
    });
  });
}

process.on('uncaughtException', (error) => {
  log('uncaught exception', { error: error.message });
  void shutdown().finally(() => {
    process.exit(1);
  });
});

process.on('unhandledRejection', (error) => {
  log('unhandled rejection', {
    error: error instanceof Error ? error.message : String(error),
  });
});

await delay(0);
log('ready');
