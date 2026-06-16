import { useEffect, useState } from 'react';

interface FrameStreamSession {
  stream: MediaStream;
  cleanup: () => void;
}

interface PendingFrame {
  width: number;
  height: number;
  pixels: Uint8ClampedArray;
}

export interface FrameStreamCounters {
  received: number;
  accepted: number;
  droppedBusy: number;
  droppedBackpressure: number;
}

function stopStreamTracks(stream: MediaStream): void {
  stream.getTracks().forEach((track) => track.stop());
}

function canAcceptFrame(
  writer: WritableStreamDefaultWriter<VideoFrame>,
  pumping: boolean,
): 'accept' | 'busy' | 'backpressure' {
  if (pumping) {
    return 'busy';
  }

  if (writer.desiredSize !== null && writer.desiredSize <= 0) {
    return 'backpressure';
  }

  return 'accept';
}

export function createMediaStreamFromFrameSocket(
  wsUrl: string,
  counters?: FrameStreamCounters,
): Promise<FrameStreamSession> {
  const Generator = (
    globalThis as typeof globalThis & {
      MediaStreamTrackGenerator?: new (init: { kind: 'video' }) => MediaStreamTrack & {
        readable: ReadableStream<VideoFrame>;
        writable: WritableStream<VideoFrame>;
      };
    }
  ).MediaStreamTrackGenerator;

  if (!Generator || typeof VideoFrame === 'undefined') {
    return Promise.reject(
      new Error('This runtime does not support MediaStreamTrackGenerator / VideoFrame'),
    );
  }

  return new Promise((resolve, reject) => {
    const generator = new Generator({ kind: 'video' });
    const writer = generator.writable.getWriter();
    const stream = new MediaStream([generator]);
    const ws = new WebSocket(wsUrl);
    ws.binaryType = 'arraybuffer';

    let active = true;
    let settled = false;
    let pending: PendingFrame | null = null;
    let pumping = false;

    const cleanup = () => {
      active = false;
      pending = null;
      if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
        ws.close();
      }
      void writer.close().catch(() => undefined);
      stopStreamTracks(stream);
    };

    const fail = (error: Error) => {
      if (settled) {
        return;
      }

      settled = true;
      cleanup();
      reject(error);
    };

    const timeout = window.setTimeout(() => {
      fail(new Error('Timed out waiting for the first capture frame'));
    }, 8_000);

    const pump = async () => {
      if (pumping || !active) {
        return;
      }

      pumping = true;

      while (active && pending) {
        const next = pending;
        pending = null;

        const frame = new VideoFrame(next.pixels, {
          format: 'RGBA',
          codedWidth: next.width,
          codedHeight: next.height,
          timestamp: Math.round(performance.now() * 1000),
        });

        try {
          await writer.write(frame);
        } catch (error: unknown) {
          frame.close();
          if (active) {
            fail(error instanceof Error ? error : new Error('Failed to write capture frame'));
          }
          pumping = false;
          return;
        }

        frame.close();

        if (!settled) {
          settled = true;
          window.clearTimeout(timeout);
          resolve({ stream, cleanup });
        }
      }

      pumping = false;

      if (active && pending) {
        void pump();
      }
    };

    ws.onerror = () => {
      fail(new Error('Lost connection to the native capture stream'));
    };

    ws.onmessage = (event) => {
      if (!active) {
        return;
      }

      counters && (counters.received += 1);

      const acceptance = canAcceptFrame(writer, pumping);
      if (acceptance !== 'accept') {
        if (counters) {
          if (acceptance === 'busy') {
            counters.droppedBusy += 1;
          } else {
            counters.droppedBackpressure += 1;
          }
        }
        return;
      }

      const buffer = event.data;
      if (!(buffer instanceof ArrayBuffer) || buffer.byteLength < 8) {
        return;
      }

      const view = new DataView(buffer);
      const width = view.getUint32(0, true);
      const height = view.getUint32(4, true);
      const expectedLength = width * height * 4;

      if (width === 0 || height === 0 || buffer.byteLength - 8 !== expectedLength) {
        return;
      }

      if (counters) {
        counters.accepted += 1;
      }

      pending = {
        width,
        height,
        pixels: new Uint8ClampedArray(buffer, 8, expectedLength).slice(),
      };

      void pump();
    };
  });
}
