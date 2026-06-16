export const ROOM_CODE_LENGTH = 5 as const;

export const ROOM_CODE_CHARSET = 'ABCDEFGHJKMNPQRSTUVWXYZ23456789' as const;

export const ROOM_TTL_SECONDS_DEFAULT = 28_800;

export const DEV_SERVER_PORT = 3841 as const;
export const DEV_WEB_PORT = 5173 as const;

export const PRODUCTION_HOST = 'tandem.tafu.casa' as const;
export const DEVELOPMENT_HOST = '127.0.0.1' as const;

export const STREAM_SLOTS = ['main', 'notes', 'aux1', 'aux2'] as const;
export type StreamSlot = (typeof STREAM_SLOTS)[number];

export const DEFAULT_STREAM_SLOTS = ['main', 'notes'] as const satisfies readonly StreamSlot[];
export const AUX_STREAM_SLOTS = ['aux1', 'aux2'] as const satisfies readonly StreamSlot[];

export const STREAM_SLOT_LABELS: Record<StreamSlot, string> = {
  main: 'Main presentation',
  notes: 'Presenter notes',
  aux1: 'Auxiliary 1',
  aux2: 'Auxiliary 2',
};

export type AuxStreamSlot = (typeof AUX_STREAM_SLOTS)[number];

/** Custom display names for auxiliary feeds, set by the desktop publisher. */
export type AuxSlotLabels = Partial<Record<AuxStreamSlot, string>>;

export const AUX_SLOT_LABEL_MAX_LENGTH = 64;

export function normalizeAuxSlotLabels(labels: AuxSlotLabels | undefined): AuxSlotLabels {
  if (!labels) {
    return {};
  }

  const next: AuxSlotLabels = {};

  for (const slot of AUX_STREAM_SLOTS) {
    const raw = labels[slot];
    if (typeof raw !== 'string') {
      continue;
    }

    const trimmed = raw.trim().slice(0, AUX_SLOT_LABEL_MAX_LENGTH);
    if (trimmed) {
      next[slot] = trimmed;
    }
  }

  return next;
}

export function resolveSlotLabel(slot: StreamSlot, auxLabels?: AuxSlotLabels): string {
  if ((AUX_STREAM_SLOTS as readonly StreamSlot[]).includes(slot)) {
    const custom = auxLabels?.[slot as AuxStreamSlot];
    if (custom) {
      return custom;
    }
  }

  return STREAM_SLOT_LABELS[slot];
}

export function normalizeVisibleSlots(slots: readonly StreamSlot[]): StreamSlot[] {
  const next: StreamSlot[] = [...DEFAULT_STREAM_SLOTS];

  for (const aux of AUX_STREAM_SLOTS) {
    if (slots.includes(aux)) {
      next.push(aux);
    }
  }

  return next;
}

export const PARTICIPANT_ROLES = ['publisher', 'viewer'] as const;
export type ParticipantRole = (typeof PARTICIPANT_ROLES)[number];

export const CLIENT_TYPES = ['desktop', 'web'] as const;
export type ClientType = (typeof CLIENT_TYPES)[number];

export function getDefaultServerUrl(isDev: boolean): string {
  if (isDev) {
    return `http://${DEVELOPMENT_HOST}:${DEV_SERVER_PORT}`;
  }
  return `https://${PRODUCTION_HOST}`;
}
