import { useEffect } from 'react';

import { logClientLogPath, startClientDiagnostics } from '../lib/diagnostics';

export function useClientDiagnostics(enabled: boolean): void {
  useEffect(() => {
    if (!enabled) {
      return;
    }

    void logClientLogPath();
    return startClientDiagnostics(true);
  }, [enabled]);
}
