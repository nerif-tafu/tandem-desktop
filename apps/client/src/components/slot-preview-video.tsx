import { useEffect, useRef } from 'react';

interface SlotPreviewVideoProps {
  stream: MediaStream;
  label: string;
}

export function SlotPreviewVideo({ stream, label }: SlotPreviewVideoProps) {
  const videoRef = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) {
      return;
    }

    video.srcObject = stream;
    void video.play().catch(() => {
      // Autoplay may be blocked until user interaction; muted previews are usually allowed.
    });

    return () => {
      video.srcObject = null;
    };
  }, [stream]);

  return (
    <video
      ref={videoRef}
      className="block h-full w-full object-contain"
      autoPlay
      muted
      playsInline
      aria-label={label}
    />
  );
}
