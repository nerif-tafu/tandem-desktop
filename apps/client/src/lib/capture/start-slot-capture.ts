import { invoke } from '@tauri-apps/api/core';

import type { StreamSlot } from '@tandem/shared';

import type { CaptureSource } from '../../types/capture';
import type { FrameStreamCounters } from './native-frame-stream';
import { createMediaStreamFromFrameSocket } from './native-frame-stream';
import { createWebcamStream } from './webcam';

export async function stopSlotCapture(slot: StreamSlot): Promise<void> {
  await invoke('stop_slot_video', { slot });
}

function canUseBrowserWebcam(): boolean {
  return typeof navigator.mediaDevices?.getUserMedia === 'function';
}

function prefersNativeWebcam(): boolean {
  // WebKit getUserMedia is unreliable inside the Linux AppImage (portal/GStreamer deps).
  return /Linux/i.test(navigator.userAgent);
}

export async function startSlotCapture(
  slot: StreamSlot,
  source: CaptureSource,
  counters?: FrameStreamCounters,
): Promise<{ stream: MediaStream; cleanup: () => Promise<void> }> {
  if (source.kind === 'webcam' && canUseBrowserWebcam() && !prefersNativeWebcam()) {
    const { stream, cleanup } = await createWebcamStream(source.id);
    return {
      stream,
      cleanup: async () => {
        cleanup();
      },
    };
  }

  await stopSlotCapture(slot);
  const wsUrl = await invoke<string>('start_slot_video', { slot, sourceId: source.id });
  const { stream, cleanup: stopSocket } = await createMediaStreamFromFrameSocket(wsUrl, counters);

  return {
    stream,
    cleanup: async () => {
      stopSocket();
      await stopSlotCapture(slot);
    },
  };
}
