import { useEffect } from 'react';

import type { PresentationExtensionStatus } from '../types/presentation';

interface ExtensionSetupDialogProps {
  status: PresentationExtensionStatus | null;
}

export function ExtensionSetupDialog({ status }: ExtensionSetupDialogProps) {
  useEffect(() => {
    function blockEscape(event: KeyboardEvent): void {
      if (event.key === 'Escape') {
        event.preventDefault();
        event.stopPropagation();
      }
    }

    window.addEventListener('keydown', blockEscape, true);
    return () => window.removeEventListener('keydown', blockEscape, true);
  }, []);

  if (!status?.applicable || status.active) {
    return null;
  }

  return (
    <div
      className="extension-setup-overlay fixed inset-0 z-[60] flex items-center justify-center bg-foreground/40 p-6 backdrop-blur-sm"
      role="presentation"
    >
      <div
        className="extension-setup-card relative w-full max-w-md rounded-2xl border border-border bg-card p-6 shadow-xl"
        role="dialog"
        aria-modal="true"
        aria-labelledby="extension-setup-title"
      >
        <h2 id="extension-setup-title" className="text-lg font-semibold">
          Restart required
        </h2>

        <p className="mt-3 text-sm text-muted-foreground">
          In order for this program to function, a reboot of the machine is required. Please sign
          out and reopen Tandem.
        </p>
      </div>
    </div>
  );
}
