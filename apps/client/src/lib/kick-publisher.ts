import { KickPublisherResponseSchema } from '@tandem/shared';

import { getServerUrl } from './server-url';

export async function kickPublisher(
  roomCode: string,
  participantId: string,
  targetPublisherId: string,
) {
  const baseUrl = await getServerUrl();
  const response = await fetch(`${baseUrl}/api/rooms/${roomCode}/kick`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ participantId, targetPublisherId }),
  });

  const payload = await response.json();
  if (!response.ok) {
    throw new Error(payload.error?.message ?? 'Failed to remove presenter');
  }

  return KickPublisherResponseSchema.parse(payload);
}
