import { useCallback, useEffect, useRef, useState } from 'react';

const TOAST_DURATION_MS = 2000;
const TOAST_FADE_MS = 200;

export function useToast() {
  const [message, setMessage] = useState<string | null>(null);
  const [visible, setVisible] = useState(false);
  const hideTimeoutRef = useRef<number | null>(null);
  const clearTimeoutRef = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (hideTimeoutRef.current !== null) {
        window.clearTimeout(hideTimeoutRef.current);
      }
      if (clearTimeoutRef.current !== null) {
        window.clearTimeout(clearTimeoutRef.current);
      }
    };
  }, []);

  const showToast = useCallback((nextMessage: string) => {
    if (hideTimeoutRef.current !== null) {
      window.clearTimeout(hideTimeoutRef.current);
    }
    if (clearTimeoutRef.current !== null) {
      window.clearTimeout(clearTimeoutRef.current);
    }

    setMessage(nextMessage);
    setVisible(false);

    requestAnimationFrame(() => {
      requestAnimationFrame(() => setVisible(true));
    });

    hideTimeoutRef.current = window.setTimeout(() => {
      setVisible(false);

      clearTimeoutRef.current = window.setTimeout(() => {
        setMessage(null);
        clearTimeoutRef.current = null;
      }, TOAST_FADE_MS);

      hideTimeoutRef.current = null;
    }, TOAST_DURATION_MS);
  }, []);

  return { message, visible, showToast };
}

export function Toast({ message, visible }: { message: string | null; visible: boolean }) {
  if (!message) {
    return null;
  }

  return (
    <div
      className={`pointer-events-none fixed bottom-6 left-1/2 z-50 -translate-x-1/2 rounded-xl border border-border bg-foreground px-4 py-3 text-sm font-medium text-background shadow-lg transition-opacity duration-200 ease-out motion-reduce:transition-none ${
        visible ? 'opacity-100' : 'opacity-0'
      }`}
      role="status"
      aria-live="polite"
    >
      {message}
    </div>
  );
}
