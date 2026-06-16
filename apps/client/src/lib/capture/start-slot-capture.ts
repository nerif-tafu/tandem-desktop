import { invoke } from '@tauri-apps/api/core';

import type { StreamSlot } from '@tandem/shared';

import type { CaptureSource } from '../../types/capture';
import { createMediaStreamFromFrameSocket } from './native-frame-stream';
import { createWebcamStream } from './webcam';

export async function stopSlotCapture(slot: StreamSlot): Promise<void> {
  await invoke('stop_slot_video', { slot });
}

export async function startSlotCapture(
  slot: StreamSlot,
  source: CaptureSource,
): Promise<{ stream: MediaStream; cleanup: () => Promise<void> }> {
  if (source.kind === 'webcam') {
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
  const { stream, cleanup: stopSocket } = await createMediaStreamFromFrameSocket(wsUrl);

  return {
    stream,
    cleanup: async () => {
      stopSocket();
      await stopSlotCapture(slot);
    },
  };
}
