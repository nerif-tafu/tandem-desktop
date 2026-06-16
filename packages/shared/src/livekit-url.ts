const LOOPBACK_HOSTS = new Set(['localhost', '127.0.0.1', '[::1]']);

export function isLoopbackHost(hostname: string): boolean {
  return LOOPBACK_HOSTS.has(hostname.toLowerCase());
}

/** Normalize loopback LiveKit URLs for browser WebRTC (Firefox is strict about localhost vs 127.0.0.1). */
export function normalizeLiveKitUrl(url: string): string {
  try {
    const parsed = new URL(url);
    if (parsed.hostname === '127.0.0.1' || parsed.hostname === '[::1]') {
      parsed.hostname = 'localhost';
    }

    return parsed.href.replace(/\/$/, '');
  } catch {
    return url;
  }
}

/** Pick the LiveKit WS URL that matches how the user opened this client (critical for ICE). */
export function resolveLiveKitUrlForClient(serverUrl: string, clientHostname?: string): string {
  if (clientHostname) {
    const protocol =
      serverUrl.startsWith('wss://') || serverUrl.startsWith('https://') ? 'wss:' : 'ws:';
    return `${protocol}//${clientHostname}:7880`;
  }

  return normalizeLiveKitUrl(serverUrl);
}
