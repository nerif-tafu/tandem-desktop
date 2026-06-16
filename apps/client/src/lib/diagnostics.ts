import { invoke } from '@tauri-apps/api/core';

interface MemorySnapshot {
  usedJsHeapMb: number | null;
  totalJsHeapMb: number | null;
  jsHeapLimitMb: number | null;
}

interface FrameStreamCounters {
  received: number;
  accepted: number;
  droppedBusy: number;
  droppedBackpressure: number;
}

const globalCounters = new Map<string, FrameStreamCounters>();

export function registerFrameStreamCounters(slot: string, counters: FrameStreamCounters): void {
  globalCounters.set(slot, counters);
}

export function unregisterFrameStreamCounters(slot: string): void {
  globalCounters.delete(slot);
}

function readMemorySnapshot(): MemorySnapshot {
  const memory = (performance as Performance & {
    memory?: { usedJSHeapSize: number; totalJSHeapSize: number; jsHeapSizeLimit: number };
  }).memory;

  if (!memory) {
    return { usedJsHeapMb: null, totalJsHeapMb: null, jsHeapLimitMb: null };
  }

  const toMb = (bytes: number) => Math.round((bytes / (1024 * 1024)) * 10) / 10;

  return {
    usedJsHeapMb: toMb(memory.usedJSHeapSize),
    totalJsHeapMb: toMb(memory.totalJSHeapSize),
    jsHeapLimitMb: toMb(memory.jsHeapSizeLimit),
  };
}

async function writeClientLog(line: string): Promise<void> {
  try {
    await invoke('append_client_log', { line });
  } catch {
  }
}

export function startClientDiagnostics(active: boolean): () => void {
  if (!active) {
    return () => undefined;
  }

  let cancelled = false;

  const sample = async (reason: string) => {
    const memory = readMemorySnapshot();
    const streams = Object.fromEntries(globalCounters.entries());

    let capture: unknown = null;
    try {
      capture = await invoke('get_capture_diagnostics');
    } catch {
    }

    const payload = {
      at: new Date().toISOString(),
      reason,
      memory,
      streams,
      capture,
    };

    const line = `[diagnostics] ${JSON.stringify(payload)}`;
    console.info(line);
    await writeClientLog(line);
  };

  void sample('startup');
  const interval = window.setInterval(() => {
    if (!cancelled) {
      void sample('interval');
    }
  }, 60_000);

  const onVisibility = () => {
    if (!cancelled && document.visibilityState === 'visible') {
      void sample('visible');
    }
  };

  document.addEventListener('visibilitychange', onVisibility);

  return () => {
    cancelled = true;
    window.clearInterval(interval);
    document.removeEventListener('visibilitychange', onVisibility);
    void sample('shutdown');
  };
}

export async function logClientLogPath(): Promise<void> {
  try {
    const path = await invoke<string | null>('get_client_log_path');
    if (path) {
      console.info(`Tandem client log file: ${path}`);
    }
  } catch {
  }
}
