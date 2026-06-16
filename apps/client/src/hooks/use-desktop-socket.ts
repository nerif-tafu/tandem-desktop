import { useCallback, useEffect, useRef, useState } from 'react';
import { io, type Socket } from 'socket.io-client';

import {
  SOCKET_EVENTS,
  type ActivePublisherState,
  type AuxSlotLabels,
  type DesktopPublisher,
  type StreamSlot,
} from '@tandem/shared';

import { getServerUrl } from '../lib/server-url';
import { kickPublisher as kickPublisherViaApi } from '../lib/kick-publisher';

export interface DesktopStreamLayout {
  visibleSlots: StreamSlot[];
  auxLabels: AuxSlotLabels;
}

export function useDesktopSocket(
  roomCode: string | null,
  participantId: string | null,
  streamLayout: DesktopStreamLayout | null = null,
  options?: { onKicked?: () => void },
) {
  const [socket, setSocket] = useState<Socket | null>(null);
  const [connected, setConnected] = useState(false);
  const [roomStateReady, setRoomStateReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activePublisherId, setActivePublisherId] = useState<string | null>(null);
  const [publishers, setPublishers] = useState<DesktopPublisher[]>([]);
  const streamLayoutRef = useRef(streamLayout);
  const activePublisherIdRef = useRef<string | null>(null);
  const onKickedRef = useRef(options?.onKicked);

  streamLayoutRef.current = streamLayout;
  activePublisherIdRef.current = activePublisherId;
  onKickedRef.current = options?.onKicked;

  const publishStreamLayout = useCallback(
    (
      nextSocket: Socket,
      nextRoomCode: string,
      nextParticipantId: string,
      layout: DesktopStreamLayout,
    ) => {
      nextSocket.emit(SOCKET_EVENTS.STREAM_LAYOUT_UPDATE, {
        roomCode: nextRoomCode,
        participantId: nextParticipantId,
        visibleSlots: layout.visibleSlots,
        auxLabels: layout.auxLabels,
      });
    },
    [],
  );

  const setActivePublisher = useCallback(
    (targetPublisherId: string) => {
      if (!socket || !roomCode || !participantId) {
        return;
      }

      socket.emit(SOCKET_EVENTS.ACTIVE_PUBLISHER_UPDATE, {
        roomCode,
        participantId,
        activePublisherId: targetPublisherId,
      });
    },
    [socket, roomCode, participantId],
  );

  const kickPublisher = useCallback(
    (targetPublisherId: string) => {
      if (!roomCode || !participantId) {
        return;
      }

      void kickPublisherViaApi(roomCode, participantId, targetPublisherId).catch((kickError: unknown) => {
        const message = kickError instanceof Error ? kickError.message : 'Failed to remove presenter';
        setError(message);
      });
    },
    [roomCode, participantId],
  );

  useEffect(() => {
    if (!roomCode || !participantId) {
      return;
    }

    let activeSocket: Socket | null = null;
    let cancelled = false;

    void getServerUrl().then((serverUrl) => {
      if (cancelled) {
        return;
      }

      activeSocket = io(serverUrl, {
        transports: ['websocket', 'polling'],
        reconnection: true,
        reconnectionDelayMax: 5000,
      });

      setSocket(activeSocket);

      activeSocket.on('connect', () => {
        setConnected(true);

        activeSocket?.emit(SOCKET_EVENTS.ROOM_JOIN, {
          roomCode,
          participantId,
          displayName: 'Presenter',
          clientType: 'desktop',
        });

        const layout = streamLayoutRef.current;
        if (layout) {
          publishStreamLayout(activeSocket!, roomCode, participantId, layout);
        }
      });

      activeSocket.on('disconnect', () => {
        setConnected(false);
        setRoomStateReady(false);
      });

      activeSocket.on(SOCKET_EVENTS.ACTIVE_PUBLISHER_STATE, (payload: ActivePublisherState) => {
        setRoomStateReady(true);
        setActivePublisherId(payload.activePublisherId);
        setPublishers(payload.publishers);
      });

      activeSocket.on(SOCKET_EVENTS.SLIDE_FORWARD, () => {
        if (activePublisherIdRef.current !== participantId) {
          return;
        }

        void import('@tauri-apps/api/core').then(({ invoke }) =>
          invoke('presentation_forward').catch((error: unknown) => {
            console.error('presentation_forward failed', error);
          }),
        );
      });

      activeSocket.on(SOCKET_EVENTS.SLIDE_BACK, () => {
        if (activePublisherIdRef.current !== participantId) {
          return;
        }

        void import('@tauri-apps/api/core').then(({ invoke }) =>
          invoke('presentation_back').catch((error: unknown) => {
            console.error('presentation_back failed', error);
          }),
        );
      });

      activeSocket.on(SOCKET_EVENTS.ERROR, (payload: { message: string }) => {
        setError(payload.message);
      });

      activeSocket.on(SOCKET_EVENTS.PUBLISHER_KICKED, () => {
        activeSocket?.disconnect();
        setSocket(null);
        setConnected(false);
        setRoomStateReady(false);
        setActivePublisherId(null);
        setPublishers([]);
        onKickedRef.current?.();
      });
    });

    return () => {
      cancelled = true;
      activeSocket?.disconnect();
      setSocket(null);
      setConnected(false);
      setRoomStateReady(false);
      setActivePublisherId(null);
      setPublishers([]);
    };
  }, [roomCode, participantId, publishStreamLayout]);

  useEffect(() => {
    if (!socket || !roomCode || !participantId || !streamLayout || !socket.connected) {
      return;
    }

    publishStreamLayout(socket, roomCode, participantId, streamLayout);
  }, [socket, roomCode, participantId, streamLayout, publishStreamLayout]);

  const leaveRoom = useCallback(() => {
    if (!socket || !roomCode || !participantId) {
      return;
    }

    socket.emit(SOCKET_EVENTS.ROOM_LEAVE, { roomCode, participantId });
    socket.disconnect();
    setSocket(null);
  }, [socket, roomCode, participantId]);

  const isActivePublisher = activePublisherId === participantId;

  return {
    error,
    connected,
    roomStateReady,
    leaveRoom,
    activePublisherId,
    publishers,
    isActivePublisher,
    setActivePublisher,
    kickPublisher,
  };
}
