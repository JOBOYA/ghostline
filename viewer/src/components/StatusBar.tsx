import { useStore } from '../store/useStore';

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

const liveKeyframes = `
@keyframes ghostline-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
`;

export function StatusBar() {
  const runs = useStore((s) => s.runs);
  const activeRunId = useStore((s) => s.activeRunId);
  const liveRunName = useStore((s) => s.liveRunName);
  const activeRun = runs.find((r) => r.id === activeRunId);

  const frames = activeRun?.frames ?? [];
  const totalTokens = frames.reduce((s, f) => s + (f.tokens_in ?? 0) + (f.tokens_out ?? 0), 0);
  const totalDuration = frames.reduce((s, f) => s + f.duration_ms, 0);

  const isLive = liveRunName != null;

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
      {isLive && (
        <>
          <style>{liveKeyframes}</style>
          <span style={{ display: 'flex', alignItems: 'center', gap: 5, color: '#22C55E' }}>
            <span
              style={{
                width: 6,
                height: 6,
                borderRadius: '50%',
                background: '#22C55E',
                animation: 'ghostline-pulse 1.5s ease-in-out infinite',
              }}
            />
            LIVE
          </span>
        </>
      )}
      <span>{frames.length} frames</span>
      <span>{totalTokens.toLocaleString()} tokens</span>
      <span>{(totalDuration / 1000).toFixed(2)}s</span>
      {activeRun && <span>{formatBytes(activeRun.fileSize)}</span>}
    </footer>
  );
}
