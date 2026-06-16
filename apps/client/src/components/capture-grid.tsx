import { useEffect, useRef, useState } from 'react';

import {
  AUX_SLOT_LABEL_MAX_LENGTH,
  AUX_STREAM_SLOTS,
  DEFAULT_STREAM_SLOTS,
  STREAM_SLOT_LABELS,
  resolveSlotLabel,
  type AuxSlotLabels,
  type AuxStreamSlot,
  type DesktopPublisher,
  type StreamSlot,
} from '@tandem/shared';

import { useCaptureSources } from '../hooks/use-capture-sources';
import { useLiveKitPublisher } from '../hooks/use-livekit-publisher';
import { useSlotCapture } from '../hooks/use-slot-capture';
import { useSuspendCaptureWhenInactive } from '../hooks/use-suspend-capture-when-inactive';
import type { DesktopStreamLayout } from '../hooks/use-desktop-socket';
import { SlotPreviewStreamsProvider } from '../contexts/slot-preview-streams';
import { CaptureSlotCard } from './capture-slot-card';
import { PresentationTargetSelect } from './presentation-target-select';
import { PublisherSwitcher } from './publisher-switcher';
import { LeaveRoomButton, ShareRoomButton } from './leave-room-dialog';
import { SettingsButton } from './settings-dialog';
import { Toast, useToast } from './toast';

function isAuxSlot(slot: StreamSlot): slot is AuxStreamSlot {
  return (AUX_STREAM_SLOTS as readonly StreamSlot[]).includes(slot);
}

function gridLayoutClass(slotCount: number): string {
  if (slotCount <= 2) {
    return 'lg:grid-cols-2 lg:grid-rows-1';
  }

  return 'lg:grid-cols-2 lg:grid-rows-2';
}

interface CaptureGridProps {
  roomCode: string;
  participantId: string;
  publishers: DesktopPublisher[];
  activePublisherId: string | null;
  socketRoomReady: boolean;
  onSelectPublisher: (publisherId: string) => void;
  onKickPublisher?: (publisherId: string, displayName: string) => void;
  onStreamLayoutChange?: (layout: DesktopStreamLayout) => void;
  onBootstrapReady?: () => void;
  onShareRoom?: () => void;
  onLeaveRoom?: () => void;
  onOpenSettings?: () => void;
}

export function CaptureGrid({
  roomCode,
  participantId,
  publishers,
  activePublisherId,
  socketRoomReady,
  onSelectPublisher,
  onKickPublisher,
  onStreamLayoutChange,
  onBootstrapReady,
  onShareRoom,
  onLeaveRoom,
  onOpenSettings,
}: CaptureGridProps) {
  return (
    <SlotPreviewStreamsProvider>
      <CaptureGridContent
        roomCode={roomCode}
        participantId={participantId}
        publishers={publishers}
        activePublisherId={activePublisherId}
        socketRoomReady={socketRoomReady}
        onSelectPublisher={onSelectPublisher}
        onKickPublisher={onKickPublisher}
        onStreamLayoutChange={onStreamLayoutChange}
        onBootstrapReady={onBootstrapReady}
        onShareRoom={onShareRoom}
        onLeaveRoom={onLeaveRoom}
        onOpenSettings={onOpenSettings}
      />
    </SlotPreviewStreamsProvider>
  );
}

function CaptureGridContent({
  roomCode,
  participantId,
  publishers,
  activePublisherId,
  socketRoomReady,
  onSelectPublisher,
  onKickPublisher,
  onStreamLayoutChange,
  onBootstrapReady,
  onShareRoom,
  onLeaveRoom,
  onOpenSettings,
}: CaptureGridProps) {
  const { sources, slots, loading: sourcesLoading, error, assignSource, refreshSources } =
    useCaptureSources(true);
  const captureSuspended = useSuspendCaptureWhenInactive(true);
  const [captureResumeNonce, setCaptureResumeNonce] = useState(0);
  const captureSuspendedRef = useRef(captureSuspended);

  useEffect(() => {
    if (captureSuspendedRef.current && !captureSuspended) {
      setCaptureResumeNonce((value) => value + 1);
    }

    captureSuspendedRef.current = captureSuspended;
  }, [captureSuspended]);

  useSlotCapture(slots, captureSuspended, captureResumeNonce);
  const { livekitReady } = useLiveKitPublisher(roomCode, participantId, slots);
  const bootstrapReportedRef = useRef(false);
  const [enabledAuxSlots, setEnabledAuxSlots] = useState<AuxStreamSlot[]>([]);
  const [auxLabels, setAuxLabels] = useState<AuxSlotLabels>({});
  const { message: toastMessage, visible: toastVisible, showToast } = useToast();

  const slotMap = new Map(slots.map((slot) => [slot.slot, slot]));
  const visibleSlots: StreamSlot[] = [...DEFAULT_STREAM_SLOTS, ...enabledAuxSlots];
  const nextAuxSlot = AUX_STREAM_SLOTS.find((slot) => !enabledAuxSlots.includes(slot));

  useEffect(() => {
    onStreamLayoutChange?.({ visibleSlots, auxLabels });
  }, [onStreamLayoutChange, visibleSlots, auxLabels]);

  useEffect(() => {
    if (bootstrapReportedRef.current) {
      return;
    }

    if (!socketRoomReady || sourcesLoading || !livekitReady) {
      return;
    }

    bootstrapReportedRef.current = true;
    onBootstrapReady?.();
  }, [socketRoomReady, sourcesLoading, livekitReady, onBootstrapReady]);

  async function copyRoomCode(): Promise<void> {
    try {
      await navigator.clipboard.writeText(roomCode);
      showToast('Copied to clipboard');
    } catch {
      // Clipboard unavailable
    }
  }

  function addAuxSlot(): void {
    if (!nextAuxSlot) {
      return;
    }

    setEnabledAuxSlots((current) => [...current, nextAuxSlot]);
  }

  async function removeAuxSlot(slot: AuxStreamSlot): Promise<void> {
    await assignSource(slot, null);
    setEnabledAuxSlots((current) => current.filter((entry) => entry !== slot));
    setAuxLabels((current) => {
      const next = { ...current };
      delete next[slot];
      return next;
    });
  }

  function updateAuxLabel(slot: AuxStreamSlot, label: string): void {
    setAuxLabels((current) => {
      const trimmed = label.trim().slice(0, AUX_SLOT_LABEL_MAX_LENGTH);
      if (!trimmed || trimmed === STREAM_SLOT_LABELS[slot]) {
        const next = { ...current };
        delete next[slot];
        return next;
      }

      return { ...current, [slot]: trimmed };
    });
  }

  return (
    <div className="flex h-screen flex-col overflow-hidden">
      <Toast message={toastMessage} visible={toastVisible} />
      <header className="shrink-0 border-b border-border bg-muted/30 px-4 py-3">
        <div className="flex items-stretch gap-3">
          <div className="flex shrink-0 flex-col justify-center border-r border-border pr-4">
            <p className="text-xs font-medium text-muted-foreground">Room</p>
            <button
              type="button"
              onClick={() => void copyRoomCode()}
              className="group text-left"
              aria-label={`Copy room code ${roomCode}`}
              title="Click to copy room code"
            >
              <span className="font-display text-2xl leading-tight transition-colors group-hover:text-accent">
                {roomCode}
              </span>
            </button>
          </div>

          <div className="flex min-w-0 flex-1 flex-col rounded-xl border border-border bg-card lg:flex-row lg:items-stretch">
            <PublisherSwitcher
              className="flex flex-col justify-center px-3 py-2.5 lg:h-full lg:flex-1 lg:pr-4"
              publishers={publishers}
              activePublisherId={activePublisherId}
              currentParticipantId={participantId}
              onSelect={onSelectPublisher}
              onKick={onKickPublisher}
            />
            <div
              className="h-px shrink-0 bg-border lg:h-auto lg:w-px"
              aria-hidden="true"
            />
            <PresentationTargetSelect className="flex flex-col justify-center px-3 py-2.5 lg:h-full lg:flex-1 lg:pl-4" />
            <div
              className="h-px shrink-0 bg-border lg:h-auto lg:w-px"
              aria-hidden="true"
            />
            <div
              className="flex items-center justify-center gap-1.5 self-stretch px-2 py-2.5 lg:px-3"
              role="toolbar"
              aria-label="Session actions"
            >
              {onShareRoom && <ShareRoomButton onClick={onShareRoom} />}
              {onOpenSettings && <SettingsButton onClick={onOpenSettings} />}
              {onLeaveRoom && (
                <>
                  <div className="h-6 w-px shrink-0 bg-border" aria-hidden="true" />
                  <LeaveRoomButton onClick={onLeaveRoom} />
                </>
              )}
            </div>
          </div>
        </div>
        {error && (
          <p className="mt-2 text-sm text-red-600" role="alert">
            {error}
          </p>
        )}
      </header>

      <div
        className={`min-h-0 flex-1 gap-3 p-3 max-lg:overflow-y-auto lg:grid lg:overflow-hidden ${gridLayoutClass(visibleSlots.length)}`}
      >
        {visibleSlots.map((slot) => (
          <CaptureSlotCard
            key={slot}
            slot={slot}
            state={slotMap.get(slot)}
            sources={sources}
            className="max-lg:shrink-0 lg:h-full lg:min-h-0"
            displayLabel={resolveSlotLabel(slot, auxLabels)}
            removable={isAuxSlot(slot)}
            renameable={isAuxSlot(slot)}
            onAssign={assignSource}
            onRefreshSources={refreshSources}
            onLabelChange={
              isAuxSlot(slot) ? (label) => updateAuxLabel(slot, label) : undefined
            }
            onRemove={isAuxSlot(slot) ? () => void removeAuxSlot(slot) : undefined}
          />
        ))}
      </div>

      {nextAuxSlot && (
        <div className="shrink-0 border-t border-border px-3 py-3">
          <button
            type="button"
            className="h-10 w-full rounded-xl border border-dashed border-border text-sm font-medium text-muted-foreground transition-colors hover:border-accent/30 hover:bg-muted hover:text-foreground"
            onClick={addAuxSlot}
          >
            + Add auxiliary feed
          </button>
        </div>
      )}
    </div>
  );
}
