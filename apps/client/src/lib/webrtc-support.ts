/** Whether this runtime can create a WebRTC peer connection (required for LiveKit publish). */
export function isWebRtcAvailable(): boolean {
  return typeof RTCPeerConnection !== 'undefined';
}

/** Linux WebKitGTK needs GStreamer "bad" plugins for WebRTC; logs warn when they are missing. */
export function linuxWebRtcPackageHint(): string | null {
  if (typeof navigator === 'undefined' || !/Linux/i.test(navigator.userAgent)) {
    return null;
  }

  return 'Install WebRTC support for the desktop app: sudo apt install gstreamer1.0-plugins-bad gstreamer1.0-nice gstreamer1.0-plugins-good — then restart Tandem.';
}

export function describeLiveKitConnectFailure(error: unknown): string {
  if (!isWebRtcAvailable()) {
    const hint = linuxWebRtcPackageHint();
    return hint ?? 'WebRTC is not available in this app. Viewers will not receive video.';
  }

  if (error instanceof Error && error.message) {
    return error.message;
  }

  return 'Could not connect to the media server. Viewers will not receive video.';
}
