import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';

import type { StreamSlot } from '@tandem/shared';

interface SlotPreviewFrame {
  slot: string;
  preview: string;
}

export function useSlotPreview(slot: StreamSlot, active: boolean) {
  const [preview, setPreview] = useState<string | null>(null);

  useEffect(() => {
    if (!active) {
      setPreview(null);
      return;
    }

    let cancelled = false;

    const unlisten = listen<SlotPreviewFrame>('slot-preview-frame', (event) => {
      if (!cancelled && event.payload.slot === slot) {
        setPreview(event.payload.preview);
      }
    });

    return () => {
      cancelled = true;
      void unlisten.then((dispose) => dispose());
    };
  }, [slot, active]);

  return preview;
}
