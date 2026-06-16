import { useEffect, useRef, useState } from 'react';
import { ConnectionState, Room, RoomEvent } from 'livekit-client';

import type { StreamSlot } from '@tandem/shared';

import { useSlotPreviewStreams } from '../contexts/slot-preview-streams';
import type { SlotCaptureState } from '../types/capture';
import { fetchMediaToken } from '../lib/media-token';
import { getSlotVideoPublishOptions } from '../lib/livekit-publish-options';

interface SlotPublisher {
  track: MediaStreamTrack;
  stream: MediaStream;
}

export function useLiveKitPublisher(
  roomCode: string | null,
  participantId: string | null,
  slots: SlotCaptureState[],
) {
  const { streams } = useSlotPreviewStreams();
  const roomRef = useRef<Room | null>(null);
  const publishersRef = useRef<Map<string, SlotPublisher>>(new Map());
  const activeSlotsRef = useRef<Set<string>>(new Set());
  const [connectionState, setConnectionState] = useState<'idle' | 'connecting' | 'connected' | 'failed'>(
    'idle',
  );

  useEffect(() => {
    activeSlotsRef.current = new Set(slots.filter((slot) => slot.active).map((slot) => slot.slot));
  }, [slots]);

  useEffect(() => {
    if (!roomCode || !participantId) {
      setConnectionState('idle');
      return;
    }

    let cancelled = false;
    const room = new Room();
    roomRef.current = room;
    setConnectionState('connecting');

    const handleConnectionStateChanged = (state: ConnectionState) => {
      if (state === ConnectionState.Connected) {
        setConnectionState('connected');
      }
    };

    room.on(RoomEvent.ConnectionStateChanged, handleConnectionStateChanged);

    void (async () => {
      try {
        const { token, url } = await fetchMediaToken(roomCode, participantId, 'publisher');
        if (cancelled) {
          return;
        }

        await room.connect(url, token, {
          rtcConfig: {
            iceServers: [{ urls: 'stun:stun.l.google.com:19302' }],
          },
        });
      } catch (error) {
        console.error('LiveKit publisher connect failed', error);
        if (!cancelled) {
          setConnectionState('failed');
        }
      }
    })();

    return () => {
      cancelled = true;
      room.off(RoomEvent.ConnectionStateChanged, handleConnectionStateChanged);

      for (const [slot, publisher] of publishersRef.current.entries()) {
        void room.localParticipant.unpublishTrack(publisher.track);
        publisher.track.stop();
        publisher.stream.getTracks().forEach((track) => track.stop());
        publishersRef.current.delete(slot);
      }

      publishersRef.current.clear();
      void room.disconnect();
      roomRef.current = null;
      setConnectionState('idle');
    };
  }, [roomCode, participantId]);

  useEffect(() => {
    const room = roomRef.current;
    if (!room) {
      return;
    }

    const syncPublishers = () => {
      if (room.state !== ConnectionState.Connected) {
        return;
      }

      for (const slot of publishersRef.current.keys()) {
        const stream = streams[slot as StreamSlot];
        const publisher = publishersRef.current.get(slot);

        if (!activeSlotsRef.current.has(slot) || !stream || publisher?.stream !== stream) {
          if (publisher) {
            void room.localParticipant.unpublishTrack(publisher.track);
            publisher.track.stop();
            publisher.stream.getTracks().forEach((track) => track.stop());
            publishersRef.current.delete(slot);
          }
        }
      }

      for (const [slot, stream] of Object.entries(streams) as [StreamSlot, MediaStream][]) {
        if (!activeSlotsRef.current.has(slot)) {
          continue;
        }

        const publisher = publishersRef.current.get(slot);
        if (publisher?.stream === stream) {
          continue;
        }

        const track = stream.getVideoTracks()[0];
        if (!track) {
          continue;
        }

        publishersRef.current.set(slot, { track, stream });

        void room.localParticipant.publishTrack(track, getSlotVideoPublishOptions(slot));
      }
    };

    syncPublishers();
    room.on(RoomEvent.ConnectionStateChanged, syncPublishers);

    return () => {
      room.off(RoomEvent.ConnectionStateChanged, syncPublishers);
    };
  }, [streams, slots]);

  const livekitReady = connectionState === 'connected' || connectionState === 'failed';

  return { livekitReady };
}
