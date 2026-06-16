export async function createWebcamStream(sourceId: string): Promise<{
  stream: MediaStream;
  cleanup: () => void;
}> {
  const index = Number.parseInt(sourceId.replace('webcam:', ''), 10);
  let devices = await navigator.mediaDevices.enumerateDevices();
  let videoInputs = devices.filter((device) => device.kind === 'videoinput');

  if (videoInputs.length === 0 || videoInputs.every((device) => !device.deviceId)) {
    const probe = await navigator.mediaDevices.getUserMedia({ video: true, audio: false });
    probe.getTracks().forEach((track) => track.stop());
    devices = await navigator.mediaDevices.enumerateDevices();
    videoInputs = devices.filter((device) => device.kind === 'videoinput');
  }

  const device = videoInputs[index];

  const stream = await navigator.mediaDevices.getUserMedia({
    video: device?.deviceId
      ? {
          deviceId: { exact: device.deviceId },
          width: { ideal: 3840 },
          height: { ideal: 2160 },
          frameRate: { ideal: 30, max: 60 },
        }
      : {
          width: { ideal: 3840 },
          height: { ideal: 2160 },
          frameRate: { ideal: 30, max: 60 },
        },
    audio: false,
  });

  return {
    stream,
    cleanup: () => {
      stream.getTracks().forEach((track) => track.stop());
    },
  };
}
