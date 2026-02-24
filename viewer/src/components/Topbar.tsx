import { useStore } from '../store/useStore';

export function Topbar() {
  const activeRunId = useStore((s) => s.activeRunId);
  const runs = useStore((s) => s.runs);
  const activeRun = runs.find((r) => r.id === activeRunId);

  const handleExport = () => {
    if (!activeRun) return;
    const data = JSON.stringify(activeRun.frames, null, 2);
    const blob = new Blob([data], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${activeRun.fileName.replace('.ghostline', '')}_frames.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <header
      style={{
        height: 'var(--topbar-h)',
        background: 'var(--surface)',
        borderBottom: '1px solid var(--border)',
        display: 'flex',
        alignItems: 'center',
        padding: '0 16px',
        gap: 16,
        flexShrink: 0,
      }}
    >
      <span style={{ fontWeight: 600, fontSize: 16, letterSpacing: '-0.02em' }}>
        ghostline
      </span>
      {activeRun && (
        <span style={{ color: 'var(--muted)', fontSize: 13, fontFamily: 'var(--font-mono)' }}>
          {activeRun.fileName} · v{activeRun.version} · {activeRun.frames.length} frames
        </span>
      )}
      <div style={{ flex: 1 }} />
      <button
        onClick={handleExport}
        disabled={!activeRun}
        style={{
          padding: '6px 14px',
          fontSize: 13,
          borderRadius: 6,
          background: activeRun ? 'var(--border)' : 'transparent',
          color: activeRun ? 'var(--text)' : 'var(--muted)',
        }}
      >
        Export JSON
      </button>
    </header>
  );
}
