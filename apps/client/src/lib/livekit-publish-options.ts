import { Track, VideoPreset, VideoPresets, type TrackPublishOptions } from 'livekit-client';

/** Simulcast layer: lower FPS, enough bitrate for crisp text at this tier. */
const SIMULCAST_480P = new VideoPreset(854, 480, 400_000, 12);
const SIMULCAST_720P = new VideoPreset(1280, 720, 1_000_000, 15);

export function getSlotVideoPublishOptions(slot: string): TrackPublishOptions {
  return {
    name: slot,
    source: Track.Source.Unknown,
    simulcast: true,
    degradationPreference: 'maintain-resolution',
    videoEncoding: {
      ...VideoPresets.h1080.encoding,
      maxBitrate: 3_000_000,
      maxFramerate: 20,
    },
    videoSimulcastLayers: [SIMULCAST_480P, SIMULCAST_720P],
  };
}
