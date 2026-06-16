import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';

import { getServerHostOverride, getServerUrl, setServerHostOverride } from '../lib/server-url';

interface SettingsDialogProps {
  open: boolean;
  onClose: () => void;
}

export function SettingsDialog({ open, onClose }: SettingsDialogProps) {
  const [hostOverride, setHostOverride] = useState('');
  const [resolvedUrl, setResolvedUrl] = useState('');
  const [ndiDiscoveryServer, setNdiDiscoveryServer] = useState('');

  useEffect(() => {
    if (!open) {
      return;
    }

    setHostOverride(getServerHostOverride() ?? '');
    void getServerUrl().then(setResolvedUrl);
    void invoke<string | null>('get_ndi_discovery_server').then((value) => {
      setNdiDiscoveryServer(value ?? '');
    });
  }, [open]);

  if (!open) {
    return null;
  }

  function save(): void {
    setServerHostOverride(hostOverride.trim() || null);
    void invoke('set_ndi_discovery_server', {
      discoveryServer: ndiDiscoveryServer.trim() || null,
    });
    void getServerUrl().then(setResolvedUrl);
    onClose();
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-foreground/40 p-6 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="relative w-full max-w-md rounded-2xl border border-border bg-card p-6 shadow-xl"
        role="dialog"
        aria-labelledby="settings-title"
        onClick={(event) => event.stopPropagation()}
      >
        <h2 id="settings-title" className="text-lg font-semibold">
          Settings
        </h2>

        <label className="mt-6 block space-y-2">
          <span className="text-sm font-medium">Server host override</span>
          <input
            className="h-12 w-full rounded-xl border border-border bg-background px-4 outline-none focus:ring-2 focus:ring-ring"
            value={hostOverride}
            onChange={(event) => setHostOverride(event.target.value)}
            placeholder={resolvedUrl || 'tandem.tafu.casa or 127.0.0.1:3841'}
          />
        </label>

        <label className="mt-4 block space-y-2">
          <span className="text-sm font-medium">NDI discovery server</span>
          <input
            className="h-12 w-full rounded-xl border border-border bg-background px-4 outline-none focus:ring-2 focus:ring-ring"
            value={ndiDiscoveryServer}
            onChange={(event) => setNdiDiscoveryServer(event.target.value)}
            placeholder="192.168.1.10:5959"
          />
        </label>
        <p className="mt-2 text-xs text-muted-foreground">
          Restart the app for NDI discovery server changes to take effect.
        </p>

        <div className="mt-6 flex gap-3">
          <button
            type="button"
            className="h-11 flex-1 rounded-xl border border-border px-4 text-sm font-medium hover:bg-muted"
            onClick={onClose}
          >
            Cancel
          </button>
          <button
            type="button"
            className="h-11 flex-1 rounded-xl bg-gradient-to-r from-accent to-accent-secondary px-4 text-sm font-medium text-white"
            onClick={save}
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}

export function SettingsButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      aria-label="Settings"
      className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-border bg-background text-muted-foreground transition-colors hover:border-accent/30 hover:text-foreground"
      onClick={onClick}
    >
      <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.75">
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Z"
        />
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9c.26.6.77 1.05 1.41 1.13H21a2 2 0 1 1 0 4h-.09c-.64.08-1.15.53-1.41 1.13Z"
        />
      </svg>
    </button>
  );
}
