import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ConnectionState, Room, RoomEvent } from 'livekit-client';

import type { StreamSlot } from '@tandem/shared';

import { useSlotPreviewStreams } from '../contexts/slot-preview-streams';
import type { SlotCaptureState } from '../types/capture';
import { fetchMediaToken } from '../lib/media-token';
import { appendClientLog } from '../lib/client-log';
import { getSlotVideoPublishOptions } from '../lib/livekit-publish-options';
import { isLinuxDesktop } from '../lib/platform';
import { describeLiveKitConnectFailure, isWebRtcAvailable } from '../lib/webrtc-support';

interface SlotPublisher {
  track: MediaStreamTrack;
  sourceStream: MediaStream;
}

interface LinuxSidecarSlot {
  slot: string;
  wsUrl: string;
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
  const useSidecarRef = useRef(isLinuxDesktop());
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

    if (useSidecarRef.current) {
      let cancelled = false;
      setConnectionState('connecting');
      setConnectionError(null);

      const connectTimeout = window.setTimeout(() => {
        if (!cancelled) {
          const message = 'LiveKit publisher connect timed out after 30s';
          console.error(message);
          setConnectionError(message);
          setConnectionState('failed');
          void invoke('stop_linux_livekit_publisher').catch(() => undefined);
        }
      }, 30_000);

      void (async () => {
        try {
          const { token, url } = await fetchMediaToken(roomCode, participantId, 'publisher');
          if (cancelled) {
            return;
          }

          console.info('LiveKit sidecar publisher connecting', { roomCode, url });
          void appendClientLog(`[livekit] sidecar connecting room=${roomCode} url=${url}`);
          await invoke('start_linux_livekit_publisher', { url, token });
          if (!cancelled) {
            window.clearTimeout(connectTimeout);
            console.info('LiveKit sidecar publisher started', { roomCode });
            void appendClientLog(`[livekit] sidecar started room=${roomCode}`);
            setConnectionState('connected');
          }
        } catch (error) {
          const message = describeLiveKitConnectFailure(error);
          console.error('LiveKit sidecar publisher connect failed', error);
          void appendClientLog(
            `[livekit] sidecar connect failed room=${roomCode} error=${message}`,
          );
          if (!cancelled) {
            setConnectionError(message);
            setConnectionState('failed');
          }
        }
      })();

      return () => {
        cancelled = true;
        window.clearTimeout(connectTimeout);
        void invoke('stop_linux_livekit_publisher').catch(() => undefined);
        setConnectionState('idle');
        setConnectionError(null);
      };
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
        void appendClientLog(`[livekit] publisher connecting room=${roomCode} url=${url}`);
        await room.connect(url, token, { peerConnectionTimeout: 30_000 });
        if (!cancelled) {
          console.info('LiveKit publisher connected', { roomCode });
          void appendClientLog(`[livekit] publisher connected room=${roomCode}`);
        }
      } catch (error) {
        const message = describeLiveKitConnectFailure(error);
        console.error('LiveKit publisher connect failed', error);
        void appendClientLog(
          `[livekit] publisher connect failed room=${roomCode} error=${message}`,
        );
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
    if (useSidecarRef.current) {
      if (connectionState !== 'connected') {
        return;
      }

      const syncSidecarSlots = async () => {
        try {
          const port = await invoke<number>('get_video_server_port');
          const sidecarSlots: LinuxSidecarSlot[] = slots
            .filter((slot) => slot.active)
            .map((slot) => ({
              slot: slot.slot,
              wsUrl: `ws://127.0.0.1:${port}/ws/${slot.slot}`,
            }));

          await invoke('sync_linux_livekit_publisher_slots', { slots: sidecarSlots });
        } catch (error) {
          console.error('LiveKit sidecar slot sync failed', error);
          void appendClientLog(
            `[livekit] sidecar slot sync failed error=${
              error instanceof Error ? error.message : String(error)
            }`,
          );
        }
      };

      void syncSidecarSlots();
      return;
    }

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
  }, [connectionState, streams, slots]);

  const livekitReady = connectionState === 'connected' || connectionState === 'failed';

  return { livekitReady, connectionState, connectionError };
}
