import type { StreamSlot } from '@tandem/shared';

export type CaptureSourceKind = 'screen' | 'webcam' | 'ndi';

export interface CaptureSource {
  id: string;
  kind: CaptureSourceKind;
  label: string;
}

export interface SlotCaptureState {
  slot: StreamSlot;
  label: string;
  source: CaptureSource | null;
  preview: string | null;
  active: boolean;
}

export const SLOT_LABELS: Record<StreamSlot, string> = {
  main: 'Main presentation',
  notes: 'Presenter notes',
  aux1: 'Auxiliary 1',
  aux2: 'Auxiliary 2',
};

export const SOURCE_KIND_OPTIONS: { kind: CaptureSourceKind; label: string }[] = [
  { kind: 'screen', label: 'Screen' },
  { kind: 'webcam', label: 'Webcam' },
  { kind: 'ndi', label: 'NDI' },
];

export function kindMatchesFilter(kind: CaptureSourceKind, filter: CaptureSourceKind): boolean {
  return kind === filter;
}
