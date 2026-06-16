import { useState } from 'react';

import { ROOM_CODE_LENGTH } from '@tandem/shared';

interface LandingViewProps {
  loading: boolean;
  joinLoading: boolean;
  error: string | null;
  showJoinPassword: boolean;
  onOpenCreateRoom: () => void;
  onJoinRoom: (roomCode: string, password?: string) => void;
  onJoinCodeChange: () => void;
}

export function LandingView({
  loading,
  joinLoading,
  error,
  showJoinPassword,
  onOpenCreateRoom,
  onJoinRoom,
  onJoinCodeChange,
}: LandingViewProps) {
  const [joinCode, setJoinCode] = useState('');
  const [joinPassword, setJoinPassword] = useState('');
  const busy = loading || joinLoading;

  return (
    <div className="flex min-h-screen flex-col items-center justify-center px-6">
      <button
        type="button"
        disabled={busy}
        onClick={onOpenCreateRoom}
        className="group text-center transition-transform duration-200 hover:scale-[1.02] active:scale-[0.98] disabled:opacity-60"
      >
        <p className="font-mono text-xs uppercase tracking-[0.2em] text-accent">Tandem</p>
        <h1 className="mt-4 font-display text-5xl leading-tight md:text-7xl">
          Create a{' '}
          <span className="bg-gradient-to-r from-accent to-accent-secondary bg-clip-text text-transparent">
            room
          </span>
        </h1>
        <p className="mt-4 text-lg text-muted-foreground">
          {loading ? 'Creating room…' : 'Tap to start a session for remote co-presenters'}
        </p>
      </button>

      <div className="mt-12 w-full max-w-sm space-y-3">
        <p className="text-center text-sm font-medium text-muted-foreground">Or join another meeting</p>
        <form
          className="space-y-3"
          onSubmit={(event) => {
            event.preventDefault();
            const normalized = joinCode.trim().toUpperCase();
            if (normalized.length === ROOM_CODE_LENGTH) {
              onJoinRoom(normalized, joinPassword.trim() || undefined);
            }
          }}
        >
          <div className="flex gap-2">
            <input
              type="text"
              className="h-11 flex-1 rounded-xl border border-border bg-card px-4 font-mono uppercase tracking-[0.25em] outline-none focus:ring-2 focus:ring-ring"
              maxLength={ROOM_CODE_LENGTH}
              value={joinCode}
              placeholder="ABCDE"
              disabled={busy}
              aria-label="Room code"
              onChange={(event) => {
                setJoinCode(event.target.value.toUpperCase());
                onJoinCodeChange();
              }}
            />
            <button
              type="submit"
              className="h-11 shrink-0 rounded-xl bg-foreground px-4 text-sm font-medium text-background hover:opacity-90 disabled:opacity-50"
              disabled={busy || joinCode.trim().length !== ROOM_CODE_LENGTH}
            >
              {joinLoading ? 'Joining…' : 'Join'}
            </button>
          </div>
          {showJoinPassword && (
            <input
              type="password"
              className="h-11 w-full rounded-xl border border-border bg-card px-4 outline-none focus:ring-2 focus:ring-ring"
              value={joinPassword}
              placeholder="Room password"
              disabled={busy}
              autoComplete="current-password"
              autoFocus
              onChange={(event) => setJoinPassword(event.target.value)}
            />
          )}
        </form>
      </div>

      {error && (
        <p className="mt-8 max-w-md text-center text-sm text-red-600" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
