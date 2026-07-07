import { useEffect, useRef, useState } from 'react';
import { ConnectionState, Room, RoomEvent } from 'livekit-client';

import type { StreamSlot } from '@tandem/shared';

import { useSlotPreviewStreams } from '../contexts/slot-preview-streams';
import type { SlotCaptureState } from '../types/capture';
import { fetchMediaToken } from '../lib/media-token';
import { getSlotVideoPublishOptions } from '../lib/livekit-publish-options';
import { describeLiveKitConnectFailure, isWebRtcAvailable } from '../lib/webrtc-support';

interface SlotPublisher {
  track: MediaStreamTrack;
  sourceStream: MediaStream;
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
  const [connectionError, setConnectionError] = useState<string | null>(null);

  useEffect(() => {
    activeSlotsRef.current = new Set(slots.filter((slot) => slot.active).map((slot) => slot.slot));
  }, [slots]);

  useEffect(() => {
    if (!roomCode || !participantId) {
      setConnectionState('idle');
      setConnectionError(null);
      return;
    }

    if (!isWebRtcAvailable()) {
      const message = describeLiveKitConnectFailure(null);
      console.error('LiveKit publisher unavailable:', message);
      setConnectionState('failed');
      setConnectionError(message);
      return;
    }

    let cancelled = false;
    const room = new Room();
    roomRef.current = room;
    setConnectionState('connecting');
    setConnectionError(null);

    const connectTimeout = window.setTimeout(() => {
      if (!cancelled && room.state !== ConnectionState.Connected) {
        const message = 'LiveKit publisher connect timed out after 30s';
        console.error(message);
        setConnectionError(message);
        setConnectionState('failed');
        void room.disconnect();
      }
    }, 30_000);

    const handleConnectionStateChanged = (state: ConnectionState) => {
      if (state === ConnectionState.Connected) {
        window.clearTimeout(connectTimeout);
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

        console.info('LiveKit publisher connecting', { roomCode, url });
        await room.connect(url, token, { peerConnectionTimeout: 30_000 });
        if (!cancelled) {
          console.info('LiveKit publisher connected', { roomCode });
        }
      } catch (error) {
        const message = describeLiveKitConnectFailure(error);
        console.error('LiveKit publisher connect failed', error);
        if (!cancelled) {
          setConnectionError(message);
          setConnectionState('failed');
        }
      }
    })();

    return () => {
      cancelled = true;
      window.clearTimeout(connectTimeout);
      room.off(RoomEvent.ConnectionStateChanged, handleConnectionStateChanged);

      for (const [slot, publisher] of publishersRef.current.entries()) {
        void room.localParticipant.unpublishTrack(publisher.track);
        publisher.track.stop();
        publishersRef.current.delete(slot);
      }

      publishersRef.current.clear();
      void room.disconnect();
      roomRef.current = null;
      setConnectionState('idle');
      setConnectionError(null);
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

        if (!activeSlotsRef.current.has(slot) || !stream || publisher?.sourceStream !== stream) {
          if (publisher) {
            void room.localParticipant.unpublishTrack(publisher.track);
            publisher.track.stop();
            publishersRef.current.delete(slot);
          }
        }
      }

      for (const [slot, stream] of Object.entries(streams) as [StreamSlot, MediaStream][]) {
        if (!activeSlotsRef.current.has(slot)) {
          continue;
        }

        const publisher = publishersRef.current.get(slot);
        if (publisher?.sourceStream === stream) {
          continue;
        }

        const sourceTrack = stream.getVideoTracks()[0];
        if (!sourceTrack) {
          continue;
        }

        if (publisher) {
          void room.localParticipant.unpublishTrack(publisher.track);
          publisher.track.stop();
        }

        const publishTrack = sourceTrack.clone();
        publishersRef.current.set(slot, { track: publishTrack, sourceStream: stream });

        void room.localParticipant.publishTrack(publishTrack, getSlotVideoPublishOptions(slot));
      }
    };

    syncPublishers();
    room.on(RoomEvent.ConnectionStateChanged, syncPublishers);

    return () => {
      room.off(RoomEvent.ConnectionStateChanged, syncPublishers);
    };
  }, [streams, slots]);

  const livekitReady = connectionState === 'connected' || connectionState === 'failed';

  return { livekitReady, connectionState, connectionError };
}
