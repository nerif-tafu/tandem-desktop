import { useState } from 'react';

import { invoke } from '@tauri-apps/api/core';

import {
  DEFAULT_STREAM_SLOTS,
  CreateRoomResponseSchema,
  JoinRoomResponseSchema,
  buildRoomWebUrl,
  getDefaultWebOrigin,
  isValidRoomCodeFormat,
  normalizeRoomCode,
} from '@tandem/shared';

import { CaptureGrid } from './components/capture-grid';
import { CreateRoomDialog, type CreateRoomOptions } from './components/create-room-dialog';
import { LandingView } from './components/landing-view';
import { LeaveRoomDialog } from './components/leave-room-dialog';
import { SettingsButton, SettingsDialog } from './components/settings-dialog';
import { SessionLoadingScreen } from './components/session-loading-screen';
import { Toast, useToast } from './components/toast';
import { useDesktopSocket, type DesktopStreamLayout } from './hooks/use-desktop-socket';
import { usePreventBackgroundThrottling } from './hooks/use-prevent-background-throttling';
import { getServerUrl } from './lib/server-url';

type Session = {
  roomCode: string;
  participantId: string;
};

class JoinRoomError extends Error {
  readonly code?: string;

  constructor(message: string, code?: string) {
    super(message);
    this.name = 'JoinRoomError';
    this.code = code;
  }
}

function isPasswordJoinError(code: string | undefined): boolean {
  return code === 'ROOM_PASSWORD_REQUIRED' || code === 'INVALID_ROOM_PASSWORD';
}

export function App() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [leaveOpen, setLeaveOpen] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);
  const [session, setSession] = useState<Session | null>(null);
  const [streamLayout, setStreamLayout] = useState<DesktopStreamLayout>({
    visibleSlots: [...DEFAULT_STREAM_SLOTS],
    auxLabels: {},
  });
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [joinLoading, setJoinLoading] = useState(false);
  const [joinPasswordRequired, setJoinPasswordRequired] = useState(false);
  const [sessionBootstrapping, setSessionBootstrapping] = useState(false);
  const { message: toastMessage, visible: toastVisible, showToast } = useToast();
  const {
    error: socketError,
    roomStateReady,
    leaveRoom,
    activePublisherId,
    publishers,
    setActivePublisher,
    kickPublisher,
  } = useDesktopSocket(
    session?.roomCode ?? null,
    session?.participantId ?? null,
    session ? streamLayout : null,
    {
      onKicked: () => {
        setSession(null);
        setSessionBootstrapping(false);
        setStreamLayout({
          visibleSlots: [...DEFAULT_STREAM_SLOTS],
          auxLabels: {},
        });
        showToast('You were removed from the room');
      },
    },
  );
  usePreventBackgroundThrottling(Boolean(session));

  async function joinRoom(code: string, password?: string): Promise<void> {
    const baseUrl = await getServerUrl();
    const normalizedCode = normalizeRoomCode(code);

    if (!isValidRoomCodeFormat(normalizedCode)) {
      throw new Error('Enter a valid 5-character room code.');
    }

    const joinResponse = await fetch(`${baseUrl}/api/rooms/${normalizedCode}/join`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        displayName: 'Presenter',
        clientType: 'desktop',
        password,
      }),
    }).catch(() => {
      throw new Error(`Could not reach server at ${baseUrl}. Check Settings for the server host.`);
    });

    const joinPayload = await joinResponse.json();
    if (!joinResponse.ok) {
      throw new JoinRoomError(
        joinPayload.error?.message ?? 'Failed to join room',
        joinPayload.error?.code,
      );
    }

    const parsed = JoinRoomResponseSchema.parse(joinPayload);

    setSession({
      roomCode: parsed.room.code,
      participantId: parsed.participant.id,
    });
  }

  async function createAndJoinRoom(options: CreateRoomOptions): Promise<void> {
    setLoading(true);
    setSessionBootstrapping(true);
    setError(null);

    try {
      const baseUrl = await getServerUrl();
      const createResponse = await fetch(`${baseUrl}/api/rooms`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          code: options.code,
          password: options.password,
        }),
      }).catch(() => {
        throw new Error(`Could not reach server at ${baseUrl}. Check Settings for the server host.`);
      });

      let createPayload: { room?: { code?: string }; error?: { message?: string } };
      try {
        createPayload = await createResponse.json();
      } catch {
        throw new Error(`Could not reach server at ${baseUrl}. Start it with pnpm dev:server.`);
      }

      if (!createResponse.ok) {
        throw new Error(createPayload.error?.message ?? 'Failed to create room');
      }

      const parsed = CreateRoomResponseSchema.parse(createPayload);
      await joinRoom(parsed.room.code, options.password);
      setCreateOpen(false);
    } catch (joinError) {
      setSessionBootstrapping(false);
      if (joinError instanceof Error && joinError.name === 'ZodError') {
        setError('Server returned an unexpected response. Is the API server up to date?');
      } else if (joinError instanceof Error) {
        setError(joinError.message);
      } else if (typeof joinError === 'string') {
        setError(joinError);
      } else {
        setError('Failed to start session');
      }
    } finally {
      setLoading(false);
    }
  }

  async function joinExistingRoom(code: string, password?: string): Promise<void> {
    setJoinLoading(true);
    setSessionBootstrapping(true);
    setError(null);

    try {
      await joinRoom(code, password);
      setJoinPasswordRequired(false);
    } catch (joinError) {
      setSessionBootstrapping(false);
      if (joinError instanceof JoinRoomError) {
        if (isPasswordJoinError(joinError.code)) {
          setJoinPasswordRequired(true);
        }
        setError(joinError.message);
      } else if (joinError instanceof Error && joinError.name === 'ZodError') {
        setError('Server returned an unexpected response. Is the API server up to date?');
      } else if (joinError instanceof Error) {
        setError(joinError.message);
      } else {
        setError('Failed to join room');
      }
    } finally {
      setJoinLoading(false);
    }
  }

  async function shareRoomLink(): Promise<void> {
    if (!session) {
      return;
    }

    try {
      const isDev = await invoke<boolean>('is_dev_mode');
      const serverUrl = await getServerUrl();
      const webOrigin = getDefaultWebOrigin(isDev, serverUrl);
      const inviteUrl = buildRoomWebUrl(webOrigin, session.roomCode);

      await navigator.clipboard.writeText(inviteUrl);
      showToast('Invite link copied');
    } catch {
      showToast('Could not copy link');
    }
  }

  function confirmLeaveRoom(): void {
    leaveRoom();
    setSession(null);
    setSessionBootstrapping(false);
    setStreamLayout({
      visibleSlots: [...DEFAULT_STREAM_SLOTS],
      auxLabels: {},
    });
    setLeaveOpen(false);
  }

  function confirmKickPublisher(publisherId: string, displayName: string): void {
    if (!window.confirm(`Remove ${displayName} from this room?`)) {
      return;
    }

    kickPublisher(publisherId);
  }

  const displayError = error ?? socketError;
  const showSessionLoading = loading || joinLoading || sessionBootstrapping;
  const loadingMessage = loading
    ? 'Creating room…'
    : joinLoading
      ? 'Joining room…'
      : 'Connecting to room…';

  return (
    <div className="relative h-screen overflow-hidden bg-background">
      <Toast message={toastMessage} visible={toastVisible} />
      {showSessionLoading && <SessionLoadingScreen message={loadingMessage} />}
      {!session && (
        <div className="absolute right-4 top-4 z-40">
          <SettingsButton onClick={() => setSettingsOpen(true)} />
        </div>
      )}

      <SettingsDialog open={settingsOpen} onClose={() => setSettingsOpen(false)} />

      <CreateRoomDialog
        open={createOpen}
        loading={loading}
        onClose={() => setCreateOpen(false)}
        onCreate={(options) => void createAndJoinRoom(options)}
      />

      {session && (
        <LeaveRoomDialog
          open={leaveOpen}
          roomCode={session.roomCode}
          onClose={() => setLeaveOpen(false)}
          onConfirm={confirmLeaveRoom}
        />
      )}

      {session ? (
        <div className={sessionBootstrapping ? 'invisible' : undefined}>
          <CaptureGrid
            roomCode={session.roomCode}
            participantId={session.participantId}
            publishers={publishers}
            activePublisherId={activePublisherId}
            socketRoomReady={roomStateReady}
            onSelectPublisher={setActivePublisher}
            onKickPublisher={confirmKickPublisher}
            onStreamLayoutChange={setStreamLayout}
            onBootstrapReady={() => setSessionBootstrapping(false)}
            onShareRoom={() => void shareRoomLink()}
            onLeaveRoom={() => setLeaveOpen(true)}
            onOpenSettings={() => setSettingsOpen(true)}
          />
        </div>
      ) : (
        <LandingView
          loading={loading}
          joinLoading={joinLoading}
          error={displayError}
          showJoinPassword={joinPasswordRequired}
          onOpenCreateRoom={() => {
            setError(null);
            setCreateOpen(true);
          }}
          onJoinRoom={(code, password) => void joinExistingRoom(code, password)}
          onJoinCodeChange={() => {
            setJoinPasswordRequired(false);
            setError(null);
          }}
        />
      )}
    </div>
  );
}
