import { z } from 'zod';

import { CLIENT_TYPES, PARTICIPANT_ROLES, STREAM_SLOTS } from '../constants.js';
import { AuxSlotLabelsSchema } from './aux-labels.js';

export const RoomCodeSchema = z
  .string()
  .length(5)
  .regex(/^[A-Z2-9]{5}$/);

export const ParticipantSchema = z.object({
  id: z.string().uuid(),
  displayName: z.string().min(1).max(64),
  role: z.enum(PARTICIPANT_ROLES),
  clientType: z.enum(CLIENT_TYPES),
  joinedAt: z.string().datetime(),
});

export const RoomSchema = z.object({
  code: RoomCodeSchema,
  id: z.string().uuid(),
  createdAt: z.string().datetime(),
  expiresAt: z.string().datetime(),
  participantCount: z.number().int().nonnegative(),
  hasPublisher: z.boolean(),
  passwordProtected: z.boolean(),
});

export const RoomPasswordSchema = z.string().min(1).max(128);

export const CreateRoomRequestSchema = z.object({
  code: RoomCodeSchema.optional(),
  password: RoomPasswordSchema.optional(),
});

export const CreateRoomResponseSchema = z.object({
  room: RoomSchema,
});

export const JoinRoomRequestSchema = z.object({
  displayName: z.string().min(1).max(64),
  clientType: z.enum(CLIENT_TYPES),
  password: RoomPasswordSchema.optional(),
});

export const JoinRoomResponseSchema = z.object({
  room: RoomSchema,
  participant: ParticipantSchema,
});

export const SlideCommandSchema = z.object({
  roomCode: RoomCodeSchema,
  participantId: z.string().uuid(),
});

export const SlideStateSchema = z.object({
  roomCode: RoomCodeSchema,
  index: z.number().int().nonnegative(),
  updatedBy: z.string().uuid(),
  updatedAt: z.string().datetime(),
});

export const MediaTokenRequestSchema = z.object({
  roomCode: RoomCodeSchema,
  participantId: z.string().uuid(),
  role: z.enum(PARTICIPANT_ROLES),
});

export const MediaTokenResponseSchema = z.object({
  token: z.string(),
  url: z.string().url(),
});

export const SocketRoomJoinSchema = z.object({
  roomCode: RoomCodeSchema,
  participantId: z.string().uuid(),
  displayName: z.string().min(1).max(64),
  clientType: z.enum(CLIENT_TYPES),
});

export const SocketRoomLeaveSchema = z.object({
  roomCode: RoomCodeSchema,
  participantId: z.string().uuid(),
});

export const StreamSlotSchema = z.enum(STREAM_SLOTS);

export const StreamLayoutUpdateSchema = z.object({
  roomCode: RoomCodeSchema,
  participantId: z.string().uuid(),
  visibleSlots: z.array(StreamSlotSchema),
  auxLabels: AuxSlotLabelsSchema.optional(),
});

export const StreamLayoutStateSchema = z.object({
  roomCode: RoomCodeSchema,
  visibleSlots: z.array(StreamSlotSchema),
  auxLabels: AuxSlotLabelsSchema.optional(),
  updatedAt: z.string().datetime(),
});

export const DesktopPublisherSchema = z.object({
  participantId: z.string().uuid(),
  displayName: z.string().min(1).max(64),
});

export const ActivePublisherUpdateSchema = z.object({
  roomCode: RoomCodeSchema,
  participantId: z.string().uuid(),
  activePublisherId: z.string().uuid(),
});

export const ActivePublisherStateSchema = z.object({
  roomCode: RoomCodeSchema,
  activePublisherId: z.string().uuid().nullable(),
  publishers: z.array(DesktopPublisherSchema),
  updatedAt: z.string().datetime(),
});

export const PublisherKickSchema = z.object({
  roomCode: RoomCodeSchema,
  participantId: z.string().uuid(),
  targetPublisherId: z.string().uuid(),
});

export const PublisherKickedSchema = z.object({
  roomCode: RoomCodeSchema,
});

export const KickPublisherRequestSchema = z.object({
  participantId: z.string().uuid(),
  targetPublisherId: z.string().uuid(),
});

export const KickPublisherResponseSchema = z.object({
  room: RoomSchema.nullable(),
  participants: z.array(ParticipantSchema),
  activePublisherState: ActivePublisherStateSchema,
});

export type Participant = z.infer<typeof ParticipantSchema>;
export type Room = z.infer<typeof RoomSchema>;
export type CreateRoomRequest = z.infer<typeof CreateRoomRequestSchema>;
export type JoinRoomRequest = z.infer<typeof JoinRoomRequestSchema>;
export type SlideCommand = z.infer<typeof SlideCommandSchema>;
export type SlideState = z.infer<typeof SlideStateSchema>;
export type StreamLayoutUpdate = z.infer<typeof StreamLayoutUpdateSchema>;
export type StreamLayoutState = z.infer<typeof StreamLayoutStateSchema>;
export type DesktopPublisher = z.infer<typeof DesktopPublisherSchema>;
export type ActivePublisherUpdate = z.infer<typeof ActivePublisherUpdateSchema>;
export type ActivePublisherState = z.infer<typeof ActivePublisherStateSchema>;
export type PublisherKick = z.infer<typeof PublisherKickSchema>;
export type PublisherKicked = z.infer<typeof PublisherKickedSchema>;
export type KickPublisherRequest = z.infer<typeof KickPublisherRequestSchema>;
export type KickPublisherResponse = z.infer<typeof KickPublisherResponseSchema>;
export type MediaTokenRequest = z.infer<typeof MediaTokenRequestSchema>;

export const SOCKET_EVENTS = {
  ROOM_JOIN: 'room.join',
  ROOM_LEAVE: 'room.leave',
  ROOM_STATE: 'room.state',
  SLIDE_FORWARD: 'slide.forward',
  SLIDE_BACK: 'slide.back',
  SLIDE_STATE: 'slide.state',
  STREAM_LAYOUT_UPDATE: 'streams.layout.update',
  STREAM_LAYOUT_STATE: 'streams.layout.state',
  ACTIVE_PUBLISHER_UPDATE: 'publisher.active.update',
  ACTIVE_PUBLISHER_STATE: 'publisher.active.state',
  PUBLISHER_KICK: 'publisher.kick',
  PUBLISHER_KICKED: 'publisher.kicked',
  MEDIA_TOKEN: 'media.token',
  ERROR: 'error',
} as const;
