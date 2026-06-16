import { invoke } from '@tauri-apps/api/core';
import { useCallback, useEffect, useState } from 'react';

import type { StreamSlot } from '@tandem/shared';

import type { CaptureSource, SlotCaptureState } from '../types/capture';

export function useCaptureSources(enabled: boolean) {
  const [sources, setSources] = useState<CaptureSource[]>([]);
  const [slots, setSlots] = useState<SlotCaptureState[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refreshSources = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const listed = await invoke<CaptureSource[]>('list_capture_sources');
      setSources(listed);
    } catch (listError) {
      setError(listError instanceof Error ? listError.message : 'Failed to list sources');
    } finally {
      setLoading(false);
    }
  }, []);

  const refreshSlots = useCallback(async () => {
    try {
      const states = await invoke<SlotCaptureState[]>('get_slot_states');
      setSlots(states);
    } catch (slotError) {
      setError(slotError instanceof Error ? slotError.message : 'Failed to load slot states');
    }
  }, []);

  const assignSource = useCallback(async (slot: StreamSlot, sourceId: string | null) => {
    setError(null);

    try {
      const state = await invoke<SlotCaptureState>('set_slot_source', {
        slot,
        sourceId,
      });

      setSlots((current) => current.map((entry) => (entry.slot === slot ? state : entry)));
      return state;
    } catch (assignError) {
      setError(assignError instanceof Error ? assignError.message : 'Failed to assign source');
      throw assignError;
    }
  }, []);

  const refreshPreview = useCallback(async (slot: StreamSlot) => {
    setError(null);

    try {
      const state = await invoke<SlotCaptureState>('refresh_slot_preview', { slot });
      setSlots((current) => current.map((entry) => (entry.slot === slot ? state : entry)));
      return state;
    } catch (previewError) {
      setError(previewError instanceof Error ? previewError.message : 'Failed to refresh preview');
      throw previewError;
    }
  }, []);

  useEffect(() => {
    if (!enabled) {
      return;
    }

    void refreshSources();
    void refreshSlots();
  }, [enabled, refreshSources, refreshSlots]);

  return {
    sources,
    slots,
    loading,
    error,
    setError,
    refreshSources,
    refreshSlots,
    assignSource,
    refreshPreview,
  };
}
