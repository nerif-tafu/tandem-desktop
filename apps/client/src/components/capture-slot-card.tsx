import { useEffect, useMemo, useState } from 'react';

import { AUX_SLOT_LABEL_MAX_LENGTH, type StreamSlot } from '@tandem/shared';

import { useSlotPreviewStreams } from '../contexts/slot-preview-streams';
import { SlotPreviewVideo } from './slot-preview-video';
import type { CaptureSource, CaptureSourceKind, SlotCaptureState } from '../types/capture';
import { SOURCE_KIND_OPTIONS, kindMatchesFilter } from '../types/capture';

const SLOT_TITLE_CLASS =
  'block h-5 w-full truncate border-b border-transparent text-sm font-medium leading-5';

function SlotTitleInput({
  slot,
  value,
  onChange,
  onCommit,
}: {
  slot: StreamSlot;
  value: string;
  onChange: (value: string) => void;
  onCommit: () => void;
}) {
  return (
    <input
      type="text"
      className={`${SLOT_TITLE_CLASS} cursor-text appearance-none bg-transparent outline-none transition-colors border-dotted border-muted-foreground/35 hover:border-muted-foreground/55 focus:border-solid focus:border-accent/70 focus:bg-muted/30 focus:px-1 focus:-mx-1`}
      value={value}
      maxLength={AUX_SLOT_LABEL_MAX_LENGTH}
      aria-label={`${slot} feed name`}
      onChange={(event) => onChange(event.target.value)}
      onBlur={onCommit}
      onKeyDown={(event) => {
        if (event.key === 'Enter') {
          event.currentTarget.blur();
        }
      }}
    />
  );
}

interface CaptureSlotCardProps {
  slot: StreamSlot;
  state: SlotCaptureState | undefined;
  sources: CaptureSource[];
  className?: string;
  displayLabel?: string;
  removable?: boolean;
  renameable?: boolean;
  onAssign: (slot: StreamSlot, sourceId: string | null) => Promise<unknown>;
  onRefreshSources?: () => Promise<unknown>;
  onLabelChange?: (label: string) => void;
  onRemove?: () => void;
}

export function CaptureSlotCard({
  slot,
  state,
  sources,
  className = '',
  displayLabel,
  removable = false,
  renameable = false,
  onAssign,
  onRefreshSources,
  onLabelChange,
  onRemove,
}: CaptureSlotCardProps) {
  const [kindFilter, setKindFilter] = useState<CaptureSourceKind>('screen');
  const [selectedId, setSelectedId] = useState('');
  const [busy, setBusy] = useState(false);
  const [draftLabel, setDraftLabel] = useState(displayLabel ?? state?.label ?? slot);
  const { streams, captureErrors } = useSlotPreviewStreams();
  const previewStream = state?.active ? streams[slot] : undefined;
  const captureError = state?.active ? captureErrors[slot] : undefined;
  const headingLabel = displayLabel ?? state?.label ?? slot;

  const filteredSources = useMemo(
    () => sources.filter((source) => kindMatchesFilter(source.kind, kindFilter)),
    [sources, kindFilter],
  );

  useEffect(() => {
    setSelectedId(state?.source?.id ?? '');
  }, [state?.source?.id]);

  useEffect(() => {
    if (state?.source) {
      setKindFilter(state.source.kind);
    }
  }, [state?.source]);

  useEffect(() => {
    setDraftLabel(headingLabel);
  }, [headingLabel]);

  function commitLabel(): void {
    if (!renameable || !onLabelChange) {
      return;
    }

    onLabelChange(draftLabel);
  }

  function refreshSourcesOnOpen(): void {
    void onRefreshSources?.();
  }

  async function applySource(): Promise<void> {
    setBusy(true);
    try {
      await onAssign(slot, selectedId || null);
    } finally {
      setBusy(false);
    }
  }

  async function clearSource(): Promise<void> {
    setBusy(true);
    try {
      setSelectedId('');
      await onAssign(slot, null);
    } finally {
      setBusy(false);
    }
  }

  const appliedSourceId = state?.source?.id ?? '';
  const selectionMatchesApplied = Boolean(selectedId) && selectedId === appliedSourceId;

  const previewLabel = `${headingLabel} preview`;

  const previewContent = previewStream ? (
    <SlotPreviewVideo stream={previewStream} label={previewLabel} />
  ) : captureError ? (
    <div className="flex h-full items-center justify-center px-4 text-center text-sm text-red-600" role="alert">
      {captureError}
    </div>
  ) : (
    <div className="flex h-full items-center justify-center px-4 text-center text-sm text-muted-foreground">
      {state?.active ? 'Starting preview…' : 'Select a source below'}
    </div>
  );

  return (
    <article
      className={`flex min-h-0 flex-col overflow-hidden rounded-2xl border border-border bg-card shadow-md ${className}`}
    >
      <header className="flex shrink-0 items-start justify-between border-b border-border px-4 py-2.5">
        <div className="min-w-0 flex-1 pr-3">
          {renameable ? (
            <SlotTitleInput
              slot={slot}
              value={draftLabel}
              onChange={setDraftLabel}
              onCommit={commitLabel}
            />
          ) : (
            <h2 className={SLOT_TITLE_CLASS}>{headingLabel}</h2>
          )}
          <p className="text-xs text-muted-foreground">{state?.active ? 'Capturing' : 'No source'}</p>
        </div>
        <div className="flex shrink-0 items-center gap-3 self-start">
          {removable && onRemove && (
            <button
              type="button"
              aria-label="Remove auxiliary feed"
              className="flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-red-50 hover:text-red-600 disabled:opacity-50"
              disabled={busy}
              onClick={onRemove}
            >
              <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="2">
                <path strokeLinecap="round" strokeLinejoin="round" d="M18 6 6 18M6 6l12 12" />
              </svg>
            </button>
          )}
        </div>
      </header>

      <div className="preview-aspect-container hidden lg:flex">
        <div className="preview-aspect-box h-full w-full">{previewContent}</div>
      </div>

      <div className="relative aspect-video shrink-0 bg-muted lg:hidden">{previewContent}</div>

      <div className="flex shrink-0 flex-col gap-2 p-3">
        <div
          className="grid gap-1 rounded-xl bg-muted p-1"
          style={{ gridTemplateColumns: `repeat(${SOURCE_KIND_OPTIONS.length}, minmax(0, 1fr))` }}
        >
          {SOURCE_KIND_OPTIONS.map((option) => (
            <button
              key={option.kind}
              type="button"
              className={`rounded-lg px-1.5 py-1.5 text-xs font-medium transition-colors ${
                kindFilter === option.kind
                  ? 'bg-card text-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
              onClick={() => {
                setKindFilter(option.kind);
                setSelectedId('');
                void onRefreshSources?.();
              }}
            >
              {option.label}
            </button>
          ))}
        </div>

        <select
          className="h-9 w-full rounded-xl border border-border bg-background px-3 text-sm outline-none focus:ring-2 focus:ring-ring"
          value={selectedId}
          onMouseDown={refreshSourcesOnOpen}
          onChange={(event) => setSelectedId(event.target.value)}
        >
          <option value="">Choose a source…</option>
          {filteredSources.map((source) => (
            <option key={source.id} value={source.id}>
              {source.label}
            </option>
          ))}
        </select>

        {filteredSources.length === 0 && (
          <p className="text-xs text-muted-foreground">
            {kindFilter === 'ndi'
              ? 'No NDI sources found on the network.'
              : 'No sources available for this type.'}
          </p>
        )}

        <div className="flex gap-2">
          <button
            type="button"
            className="h-9 flex-1 rounded-xl border border-border text-sm font-medium hover:bg-muted disabled:opacity-50"
            disabled={busy || !state?.active}
            onClick={() => void clearSource()}
          >
            Clear
          </button>
          <button
            type="button"
            className="h-9 flex-1 rounded-xl bg-gradient-to-r from-accent to-accent-secondary text-sm font-medium text-white disabled:opacity-50"
            disabled={busy || !selectedId || selectionMatchesApplied}
            onClick={() => void applySource()}
          >
            {busy ? 'Applying…' : 'Apply'}
          </button>
        </div>
      </div>
    </article>
  );
}
