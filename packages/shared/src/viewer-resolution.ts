export const VIEWER_RESOLUTION_OPTIONS = ['480p', '720p', '1080p'] as const;

export type ViewerResolution = (typeof VIEWER_RESOLUTION_OPTIONS)[number];

export const DEFAULT_VIEWER_RESOLUTION: ViewerResolution = '720p';

export const VIEWER_RESOLUTION_DIMENSIONS: Record<
  ViewerResolution,
  { width: number; height: number; label: string }
> = {
  '480p': { width: 854, height: 480, label: '480p' },
  '720p': { width: 1280, height: 720, label: '720p' },
  '1080p': { width: 1920, height: 1080, label: '1080p' },
};

export function isViewerResolution(value: string): value is ViewerResolution {
  return (VIEWER_RESOLUTION_OPTIONS as readonly string[]).includes(value);
}
