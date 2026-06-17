const LOOPBACK_HOSTS = new Set(['localhost', '127.0.0.1', '[::1]']);

export function isLoopbackHost(hostname: string): boolean {
  return LOOPBACK_HOSTS.has(hostname.toLowerCase());
}

function coerceWsUrl(url: string): string {
  if (url.startsWith('http://')) {
    return url.replace('http://', 'ws://');
  }

  if (url.startsWith('https://')) {
    return url.replace('https://', 'wss://');
  }

  return url;
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
export function resolveLiveKitUrlForClient(
  serverUrl: string,
  clientHostname?: string,
  publicUrl?: string,
): string {
  if (publicUrl) {
    return normalizeLiveKitUrl(coerceWsUrl(publicUrl));
  }

  if (clientHostname) {
    const coerced = coerceWsUrl(serverUrl);
    const protocol = coerced.startsWith('wss://') ? 'wss:' : 'ws:';
    return `${protocol}//${clientHostname}:7880`;
  }

  return normalizeLiveKitUrl(coerceWsUrl(serverUrl));
}
