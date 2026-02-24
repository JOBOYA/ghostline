import { useStore } from '../store/useStore';

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

export function StatusBar() {
  const runs = useStore((s) => s.runs);
  const activeRunId = useStore((s) => s.activeRunId);
  const activeRun = runs.find((r) => r.id === activeRunId);

  const frames = activeRun?.frames ?? [];
  const totalTokens = frames.reduce((s, f) => s + (f.tokens_in ?? 0) + (f.tokens_out ?? 0), 0);
  const totalDuration = frames.reduce((s, f) => s + f.duration_ms, 0);

  return (
    <footer
      style={{
        height: 'var(--statusbar-h)',
        background: 'var(--surface)',
        borderTop: '1px solid var(--border)',
        display: 'flex',
        alignItems: 'center',
        padding: '0 16px',
        gap: 20,
        fontSize: 11,
        color: 'var(--muted)',
        fontFamily: 'var(--font-mono)',
        flexShrink: 0,
      }}
    >
      <span>{frames.length} frames</span>
      <span>{totalTokens.toLocaleString()} tokens</span>
      <span>{(totalDuration / 1000).toFixed(2)}s</span>
      {activeRun && <span>{formatBytes(activeRun.fileSize)}</span>}
    </footer>
  );
}
