import { invoke } from '@tauri-apps/api/core';
import { getDefaultServerUrl } from '@tandem/shared';

const HOST_OVERRIDE_KEY = 'server-host-override';

const LOCAL_HOST_PATTERN =
  /^(127\.0\.0\.1|localhost|\d{1,3}(?:\.\d{1,3}){3})(?::\d+)?$/;

function normalizeServerOverride(host: string): string {
  const trimmed = host.trim();
  if (!trimmed) {
    return trimmed;
  }

  if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) {
    return trimmed.replace(/\/$/, '');
  }

  if (LOCAL_HOST_PATTERN.test(trimmed)) {
    return `http://${trimmed}`;
  }

  return `https://${trimmed}`;
}

export async function getServerUrl(): Promise<string> {
  const override = localStorage.getItem(HOST_OVERRIDE_KEY);
  if (override) {
    return normalizeServerOverride(override);
  }

  const isDev = await invoke<boolean>('is_dev_mode');
  return getDefaultServerUrl(isDev);
}

export function setServerHostOverride(host: string | null): void {
  if (!host) {
    localStorage.removeItem(HOST_OVERRIDE_KEY);
    return;
  }

  localStorage.setItem(HOST_OVERRIDE_KEY, host);
}

export function getServerHostOverride(): string | null {
  return localStorage.getItem(HOST_OVERRIDE_KEY);
}
