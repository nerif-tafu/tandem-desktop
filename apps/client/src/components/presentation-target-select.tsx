import { useCallback, useEffect, useState } from 'react';

import { invoke } from '@tauri-apps/api/core';

import type { PresentationWindow } from '../types/presentation';

interface PresentationTargetSelectProps {
  className?: string;
}

export function PresentationTargetSelect({ className = '' }: PresentationTargetSelectProps) {
  const [windows, setWindows] = useState<PresentationWindow[]>([]);
  const [selectedId, setSelectedId] = useState('');
  const [busy, setBusy] = useState(false);

  const refreshWindows = useCallback(async () => {
    const next = await invoke<PresentationWindow[]>('list_presentation_windows');
    setWindows(next);
  }, []);

  useEffect(() => {
    void refreshWindows();
  }, [refreshWindows]);

  useEffect(() => {
    let cancelled = false;

    void invoke<string | null>('get_presentation_target').then((targetId) => {
      if (!cancelled) {
        setSelectedId(targetId ?? '');
      }
    });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!selectedId) {
      return;
    }

    if (!windows.some((window) => window.id === selectedId)) {
      setSelectedId('');
      void invoke('set_presentation_target', { sourceId: null });
    }
  }, [selectedId, windows]);

  async function handleChange(nextId: string): Promise<void> {
    setSelectedId(nextId);
    setBusy(true);

    try {
      await invoke('set_presentation_target', {
        sourceId: nextId || null,
      });
    } finally {
      setBusy(false);
    }
  }

  return (
    <label className={`block min-w-0 space-y-1.5 ${className}`}>
      <span className="text-xs font-medium text-muted-foreground">Remote clicker target</span>
      <select
        className="h-9 w-full rounded-lg border border-border bg-background px-3 text-sm outline-none focus:ring-2 focus:ring-ring disabled:opacity-60"
        value={selectedId}
        disabled={busy}
        onMouseDown={() => void refreshWindows()}
        onChange={(event) => void handleChange(event.target.value)}
      >
        <option value="">None — send keys globally</option>
        {windows.map((window) => (
          <option key={window.id} value={window.id}>
            {window.label}
          </option>
        ))}
      </select>
    </label>
  );
}
