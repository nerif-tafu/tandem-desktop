import { invoke } from '@tauri-apps/api/core';
import { MediaTokenResponseSchema, resolveLiveKitUrlForClient, type ParticipantRole } from '@tandem/shared';

import { getServerUrl } from './server-url';

export async function fetchMediaToken(
  roomCode: string,
  participantId: string,
  role: ParticipantRole,
) {
  const baseUrl = await getServerUrl();
  const response = await fetch(`${baseUrl}/api/rooms/${roomCode}/media-token`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ roomCode, participantId, role }),
  });

  if (!response.ok) {
    throw new Error('Failed to fetch LiveKit token');
  }

  const parsed = MediaTokenResponseSchema.parse(await response.json());
  const isDev = await invoke<boolean>('is_dev_mode');

  return {
    ...parsed,
    url: isDev
      ? resolveLiveKitUrlForClient(parsed.url, 'localhost')
      : resolveLiveKitUrlForClient(parsed.url),
  };
}
