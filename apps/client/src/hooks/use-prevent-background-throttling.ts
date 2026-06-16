import { useEffect } from 'react';

/**
 * WebView2 throttles timers and can suspend background tabs when the window is
 * minimized. A held Web Lock keeps the JS frame pipeline alive for streaming.
 */
export function usePreventBackgroundThrottling(enabled: boolean): void {
  useEffect(() => {
    if (!enabled || !('locks' in navigator)) {
      return;
    }

    let cancelled = false;
    let releaseLock: (() => void) | null = null;

    const holdLock = (): void => {
      void navigator.locks.request('tandem-streaming', { mode: 'shared' }, async () => {
        await new Promise<void>((resolve) => {
          releaseLock = resolve;
          if (cancelled) {
            resolve();
          }
        });
      });
    };

    holdLock();

    const onVisibilityChange = (): void => {
      if (document.visibilityState === 'hidden') {
        holdLock();
      }
    };

    document.addEventListener('visibilitychange', onVisibilityChange);

    return () => {
      cancelled = true;
      releaseLock?.();
      document.removeEventListener('visibilitychange', onVisibilityChange);
    };
  }, [enabled]);
}
