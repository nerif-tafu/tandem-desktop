import type { DesktopPublisher } from '@tandem/shared';

interface PublisherSwitcherProps {
  publishers: DesktopPublisher[];
  activePublisherId: string | null;
  currentParticipantId: string;
  onSelect: (publisherId: string) => void;
  onKick?: (publisherId: string, displayName: string) => void;
  className?: string;
}

export function PublisherSwitcher({
  publishers,
  activePublisherId,
  currentParticipantId,
  onSelect,
  onKick,
  className = '',
}: PublisherSwitcherProps) {
  return (
    <div className={`min-w-0 space-y-1.5 ${className}`}>
      <p className="text-xs font-medium text-muted-foreground">Live feed to viewers</p>
      <div className="flex flex-wrap gap-1.5">
        {publishers.map((publisher) => {
          const isActive = publisher.participantId === activePublisherId;
          const isSelf = publisher.participantId === currentParticipantId;
          const canKick = !isSelf && onKick;

          return (
            <div
              key={publisher.participantId}
              className={`flex items-stretch overflow-hidden rounded-lg border transition-colors ${
                isActive
                  ? 'border-accent bg-accent/10'
                  : 'border-border bg-card hover:border-accent/30 hover:bg-muted'
              }`}
            >
              <button
                type="button"
                className={`px-2.5 py-1.5 text-sm font-medium transition-colors ${
                  isActive ? 'text-accent' : 'text-foreground'
                }`}
                onClick={() => onSelect(publisher.participantId)}
              >
                {publisher.displayName}
                {isSelf ? ' (you)' : ''}
              </button>
              {canKick ? (
                <button
                  type="button"
                  className="border-l border-border px-1.5 text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
                  aria-label={`Remove ${publisher.displayName} from room`}
                  onClick={() => onKick(publisher.participantId, publisher.displayName)}
                >
                  <svg
                    viewBox="0 0 24 24"
                    className="h-4 w-4"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    aria-hidden="true"
                  >
                    <path d="M18 6L6 18M6 6l12 12" />
                  </svg>
                </button>
              ) : null}
            </div>
          );
        })}
      </div>
    </div>
  );
}
