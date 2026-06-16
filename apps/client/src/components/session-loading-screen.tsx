interface SessionLoadingScreenProps {
  message: string;
}

export function SessionLoadingScreen({ message }: SessionLoadingScreenProps) {
  return (
    <div className="fixed inset-0 z-50 flex flex-col items-center justify-center bg-background px-6">
      <p className="font-mono text-xs uppercase tracking-[0.2em] text-accent">Tandem</p>
      <div
        className="mt-8 h-10 w-10 animate-spin rounded-full border-2 border-border border-t-accent"
        role="status"
        aria-label="Loading"
      />
      <p className="mt-6 text-lg text-muted-foreground">{message}</p>
    </div>
  );
}
