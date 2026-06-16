import { DEV_WEB_PORT, PRODUCTION_HOST } from './constants.js';
import { RoomCodeSchema } from './schemas/protocol.js';

export const ROOM_WEB_PATH_PREFIX = '/room/';

export function buildRoomWebPath(roomCode: string, displayName?: string): string {
  const path = `${ROOM_WEB_PATH_PREFIX}${roomCode}`;
  const trimmedName = displayName?.trim();

  if (!trimmedName) {
    return path;
  }

  const params = new URLSearchParams({ name: trimmedName });
  return `${path}?${params.toString()}`;
}

export function buildRoomWebUrl(webOrigin: string, roomCode: string, displayName?: string): string {
  const origin = webOrigin.replace(/\/$/, '');
  return `${origin}${buildRoomWebPath(roomCode, displayName)}`;
}

export function parseRoomWebLocation(
  pathname: string,
  search: string,
): { roomCode: string | null; displayName: string | null } {
  const match = pathname.match(/^\/room\/([A-Za-z2-9]{5})\/?$/);
  if (!match?.[1]) {
    return { roomCode: null, displayName: null };
  }

  const parsed = RoomCodeSchema.safeParse(match[1].toUpperCase());
  if (!parsed.success) {
    return { roomCode: null, displayName: null };
  }

  const name = new URLSearchParams(search).get('name');
  return { roomCode: parsed.data, displayName: name?.trim() || null };
}

export function getDefaultWebOrigin(isDev: boolean, serverUrl?: string): string {
  if (!isDev) {
    return `https://${PRODUCTION_HOST}`;
  }

  if (serverUrl) {
    try {
      const url = new URL(serverUrl);
      if (url.hostname === '127.0.0.1' || url.hostname === '[::1]' || url.hostname === 'localhost') {
        return `http://localhost:${DEV_WEB_PORT}`;
      }

      url.protocol = 'http:';
      url.port = String(DEV_WEB_PORT);
      return url.origin;
    } catch {
      // Fall through to localhost default.
    }
  }

  return `http://localhost:${DEV_WEB_PORT}`;
}
