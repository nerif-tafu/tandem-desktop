import { useEffect, useRef } from 'react';

import type { StreamSlot } from '@tandem/shared';

import { useSlotPreviewStreams } from '../contexts/slot-preview-streams';
import { startSlotCapture } from '../lib/capture/start-slot-capture';
import type { SlotCaptureState } from '../types/capture';

interface SlotSession {
  sourceId: string;
  cleanup: () => Promise<void>;
}

function stopStreamTracks(stream: MediaStream): void {
  stream.getTracks().forEach((track) => track.stop());
}

export function useSlotCapture(
  slots: SlotCaptureState[],
  suspended = false,
  resumeNonce = 0,
) {
  const { setSlotStream, setCaptureError } = useSlotPreviewStreams();
  const sessionsRef = useRef<Map<StreamSlot, SlotSession>>(new Map());
  const attemptRef = useRef<Map<StreamSlot, number>>(new Map());
  const syncRef = useRef<Promise<void>>(Promise.resolve());
  const lastResumeNonceRef = useRef(resumeNonce);

  useEffect(() => {
    const forceRestart = lastResumeNonceRef.current !== resumeNonce;
    lastResumeNonceRef.current = resumeNonce;

    const desired = new Map<StreamSlot, NonNullable<SlotCaptureState['source']>>(
      suspended
        ? []
        : slots
            .filter(
              (slot): slot is SlotCaptureState & { source: NonNullable<SlotCaptureState['source']> } =>
                Boolean(slot.active && slot.source),
            )
            .map((slot) => [slot.slot, slot.source]),
    );

    syncRef.current = syncRef.current
      .then(async () => {
        if (forceRestart && !suspended) {
          for (const [slot, session] of [...sessionsRef.current.entries()]) {
            attemptRef.current.set(slot, (attemptRef.current.get(slot) ?? 0) + 1);
            await session.cleanup();
            sessionsRef.current.delete(slot);
            setSlotStream(slot, null);
            setCaptureError(slot, null);
          }
        }

        for (const slot of Object.keys(sessionsRef.current) as StreamSlot[]) {
          if (!desired.has(slot)) {
            setCaptureError(slot, null);
          }
        }

        for (const [slot, session] of [...sessionsRef.current.entries()]) {
          const source = desired.get(slot);
          if (!source || source.id !== session.sourceId) {
            attemptRef.current.set(slot, (attemptRef.current.get(slot) ?? 0) + 1);
            await session.cleanup();
            sessionsRef.current.delete(slot);
            setSlotStream(slot, null);
            setCaptureError(slot, null);
          }
        }

        for (const [slot, source] of desired) {
          if (sessionsRef.current.has(slot)) {
            continue;
          }

          const attempt = (attemptRef.current.get(slot) ?? 0) + 1;
          attemptRef.current.set(slot, attempt);
          setCaptureError(slot, null);

          try {
            const { stream, cleanup } = await startSlotCapture(slot, source);

            if (attemptRef.current.get(slot) !== attempt) {
              await cleanup();
              stopStreamTracks(stream);
              return;
            }

            if (sessionsRef.current.has(slot)) {
              await cleanup();
              stopStreamTracks(stream);
              return;
            }

            sessionsRef.current.set(slot, { sourceId: source.id, cleanup });
            setSlotStream(slot, stream);
          } catch (error) {
            if (attemptRef.current.get(slot) !== attempt) {
              return;
            }

            const message = error instanceof Error ? error.message : 'Failed to start capture';
            console.error(`Failed to start capture for ${slot}`, error);
            setCaptureError(slot, message);
          }
        }
      })
      .catch((error: unknown) => {
        console.error('Slot capture sync failed', error);
      });
  }, [slots, suspended, resumeNonce, setSlotStream, setCaptureError]);

  useEffect(() => {
    return () => {
      const pending = syncRef.current;

      void pending.then(async () => {
        for (const slot of sessionsRef.current.keys()) {
          attemptRef.current.set(slot, (attemptRef.current.get(slot) ?? 0) + 1);
        }

        for (const [, session] of sessionsRef.current) {
          await session.cleanup();
        }

        sessionsRef.current.clear();
      });
    };
  }, []);
}
