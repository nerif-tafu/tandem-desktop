import { useState } from 'react';

import { ROOM_CODE_LENGTH, isValidRoomCodeFormat, normalizeRoomCode } from '@tandem/shared';

export interface CreateRoomOptions {
  code?: string;
  password?: string;
}

interface CreateRoomDialogProps {
  open: boolean;
  loading: boolean;
  onClose: () => void;
  onCreate: (options: CreateRoomOptions) => void;
}

export function CreateRoomDialog({ open, loading, onClose, onCreate }: CreateRoomDialogProps) {
  const [roomCode, setRoomCode] = useState('');
  const [password, setPassword] = useState('');
  const [validationError, setValidationError] = useState<string | null>(null);

  if (!open) {
    return null;
  }

  function handleSubmit(event: React.FormEvent): void {
    event.preventDefault();

    const trimmedCode = roomCode.trim();
    if (trimmedCode) {
      const normalized = normalizeRoomCode(trimmedCode);
      if (!isValidRoomCodeFormat(normalized)) {
        setValidationError('Room code must be 5 characters (A–Z, 2–9).');
        return;
      }
    }

    setValidationError(null);
    onCreate({
      code: trimmedCode ? normalizeRoomCode(trimmedCode) : undefined,
      password: password.trim() || undefined,
    });
  }

  function handleClose(): void {
    if (loading) {
      return;
    }

    setRoomCode('');
    setPassword('');
    setValidationError(null);
    onClose();
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-foreground/40 p-6 backdrop-blur-sm"
      onClick={handleClose}
    >
      <div
        className="relative w-full max-w-md rounded-2xl border border-border bg-card p-6 shadow-xl"
        role="dialog"
        aria-labelledby="create-room-title"
        onClick={(event) => event.stopPropagation()}
      >
        <h2 id="create-room-title" className="text-lg font-semibold">
          Create a room
        </h2>
        <p className="mt-2 text-sm text-muted-foreground">
          Leave the room code blank for a random code, or choose your own. Add a password to restrict
          who can join.
        </p>

        <form className="mt-5 space-y-4" onSubmit={handleSubmit}>
          <label className="block space-y-2">
            <span className="text-sm font-medium">Room code (optional)</span>
            <input
              type="text"
              className="h-11 w-full rounded-xl border border-border bg-background px-4 font-mono uppercase tracking-[0.25em] outline-none focus:ring-2 focus:ring-ring"
              maxLength={ROOM_CODE_LENGTH}
              value={roomCode}
              placeholder="Random"
              disabled={loading}
              onChange={(event) => setRoomCode(event.target.value.toUpperCase())}
            />
          </label>

          <label className="block space-y-2">
            <span className="text-sm font-medium">Password (optional)</span>
            <input
              type="password"
              className="h-11 w-full rounded-xl border border-border bg-background px-4 outline-none focus:ring-2 focus:ring-ring"
              value={password}
              placeholder="No password"
              disabled={loading}
              autoComplete="new-password"
              onChange={(event) => setPassword(event.target.value)}
            />
          </label>

          {validationError && (
            <p className="text-sm text-red-600" role="alert">
              {validationError}
            </p>
          )}

          <div className="flex gap-3 pt-2">
            <button
              type="button"
              className="h-11 flex-1 rounded-xl border border-border px-4 text-sm font-medium hover:bg-muted disabled:opacity-50"
              disabled={loading}
              onClick={handleClose}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="h-11 flex-1 rounded-xl bg-foreground px-4 text-sm font-medium text-background hover:opacity-90 disabled:opacity-50"
              disabled={loading}
            >
              {loading ? 'Creating…' : 'Create room'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
