interface LeaveRoomDialogProps {
  open: boolean;
  roomCode: string;
  onClose: () => void;
  onConfirm: () => void;
}

export function LeaveRoomDialog({ open, roomCode, onClose, onConfirm }: LeaveRoomDialogProps) {
  if (!open) {
    return null;
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-foreground/40 p-6 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="relative w-full max-w-md rounded-2xl border border-border bg-card p-6 shadow-xl"
        role="dialog"
        aria-labelledby="leave-room-title"
        onClick={(event) => event.stopPropagation()}
      >
        <h2 id="leave-room-title" className="text-lg font-semibold">
          Leave room?
        </h2>
        <p className="mt-2 text-sm text-muted-foreground">
          You will leave room <span className="font-mono font-medium text-foreground">{roomCode}</span> and
          return to the home screen. Remote co-presenters will be disconnected from your session.
        </p>

        <div className="mt-6 flex gap-3">
          <button
            type="button"
            className="h-11 flex-1 rounded-xl border border-border px-4 text-sm font-medium hover:bg-muted"
            onClick={onClose}
          >
            Cancel
          </button>
          <button
            type="button"
            className="h-11 flex-1 rounded-xl bg-foreground px-4 text-sm font-medium text-background hover:opacity-90"
            onClick={onConfirm}
          >
            Leave room
          </button>
        </div>
      </div>
    </div>
  );
}

export function ShareRoomButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      aria-label="Share room link"
      className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-border bg-background text-foreground transition-colors hover:bg-muted"
      onClick={onClick}
    >
      <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.75">
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"
        />
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"
        />
      </svg>
    </button>
  );
}

export function LeaveRoomButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      aria-label="Leave room"
      className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-red-200 bg-background text-red-600 transition-colors hover:border-red-300 hover:bg-red-50 hover:text-red-700"
      onClick={onClick}
    >
      <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.75">
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4M16 17l5-5-5-5M21 12H9"
        />
      </svg>
    </button>
  );
}
