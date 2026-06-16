import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from 'react';

import type { StreamSlot } from '@tandem/shared';

type SlotStreams = Partial<Record<StreamSlot, MediaStream>>;
type SlotCaptureErrors = Partial<Record<StreamSlot, string>>;

interface SlotPreviewStreamsContextValue {
  streams: SlotStreams;
  captureErrors: SlotCaptureErrors;
  setSlotStream: (slot: StreamSlot, stream: MediaStream | null) => void;
  setCaptureError: (slot: StreamSlot, error: string | null) => void;
}

const SlotPreviewStreamsContext = createContext<SlotPreviewStreamsContextValue | null>(null);

export function SlotPreviewStreamsProvider({ children }: { children: ReactNode }) {
  const [streams, setStreams] = useState<SlotStreams>({});
  const [captureErrors, setCaptureErrors] = useState<SlotCaptureErrors>({});

  const setSlotStream = useCallback((slot: StreamSlot, stream: MediaStream | null) => {
    setStreams((current) => {
      if (!stream) {
        if (!current[slot]) {
          return current;
        }

        current[slot]?.getTracks().forEach((track) => track.stop());
        const next = { ...current };
        delete next[slot];
        return next;
      }

      if (current[slot] === stream) {
        return current;
      }

      current[slot]?.getTracks().forEach((track) => track.stop());
      return { ...current, [slot]: stream };
    });
  }, []);

  const setCaptureError = useCallback((slot: StreamSlot, error: string | null) => {
    setCaptureErrors((current) => {
      if (!error) {
        if (!current[slot]) {
          return current;
        }

        const next = { ...current };
        delete next[slot];
        return next;
      }

      if (current[slot] === error) {
        return current;
      }

      return { ...current, [slot]: error };
    });
  }, []);

  const value = useMemo(
    () => ({ streams, captureErrors, setSlotStream, setCaptureError }),
    [streams, captureErrors, setSlotStream, setCaptureError],
  );

  return (
    <SlotPreviewStreamsContext.Provider value={value}>{children}</SlotPreviewStreamsContext.Provider>
  );
}

export function useSlotPreviewStreams(): SlotPreviewStreamsContextValue {
  const context = useContext(SlotPreviewStreamsContext);
  if (!context) {
    throw new Error('useSlotPreviewStreams must be used within SlotPreviewStreamsProvider');
  }

  return context;
}
