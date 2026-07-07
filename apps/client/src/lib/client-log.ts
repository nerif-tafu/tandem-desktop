import { invoke } from '@tauri-apps/api/core';

export async function appendClientLog(line: string): Promise<void> {
  try {
    await invoke('append_client_log', { line });
  } catch {
    // Logging must never break capture or publishing flows.
  }
}
