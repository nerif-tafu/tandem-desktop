import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useState } from 'react';

async function shouldSuspendCapture(): Promise<boolean> {
  if (document.visibilityState === 'hidden') {
    return true;
  }

  const window = getCurrentWindow();
  const [minimized, visible] = await Promise.all([window.isMinimized(), window.isVisible()]);
  return minimized || !visible;
}

/**
 * Stops native capture while the window is minimized or hidden so DXGI / WebView
 * pipelines are not left running against a suspended renderer.
 */
export function useSuspendCaptureWhenInactive(enabled: boolean): boolean {
  const [suspended, setSuspended] = useState(false);

  useEffect(() => {
    if (!enabled) {
      setSuspended(false);
      return;
    }

    let cancelled = false;

    const sync = async (): Promise<void> => {
      if (cancelled) {
        return;
      }

      setSuspended(await shouldSuspendCapture());
    };

    void sync();

    const onVisibilityChange = (): void => {
      void sync();
    };

    document.addEventListener('visibilitychange', onVisibilityChange);

    let unlistenFocus: (() => void) | undefined;

    void getCurrentWindow()
      .onFocusChanged(() => {
        void sync();
      })
      .then((unlisten) => {
        if (cancelled) {
          void unlisten();
          return;
        }

        unlistenFocus = unlisten;
      })
      .catch(() => {
        // Non-Tauri environments fall back to visibility only.
      });

    const interval = window.setInterval(() => {
      void sync();
    }, 30_000);

    return () => {
      cancelled = true;
      document.removeEventListener('visibilitychange', onVisibilityChange);
      unlistenFocus?.();
      window.clearInterval(interval);
    };
  }, [enabled]);

  return suspended;
}
