import { useStore } from '../store/useStore';
import { DropZone } from './DropZone';

export function SidePanel() {
  const runs = useStore((s) => s.runs);
  const activeRunId = useStore((s) => s.activeRunId);
  const setActiveRun = useStore((s) => s.setActiveRun);
  const removeRun = useStore((s) => s.removeRun);

  return (
    <aside
      style={{
        width: 'var(--side-w)',
        background: 'var(--surface)',
        borderRight: '1px solid var(--border)',
        display: 'flex',
        flexDirection: 'column',
        flexShrink: 0,
        overflow: 'hidden',
      }}
    >
      <div style={{ padding: '12px 12px 8px', fontSize: 11, color: 'var(--muted)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
        Runs
      </div>
      <div style={{ flex: 1, overflowY: 'auto', padding: '0 8px' }}>
        {runs.map((run) => (
          <div
            key={run.id}
            onClick={() => setActiveRun(run.id)}
            style={{
              padding: '8px',
              borderRadius: 6,
              cursor: 'pointer',
              background: run.id === activeRunId ? 'var(--border)' : 'transparent',
              marginBottom: 2,
              display: 'flex',
              alignItems: 'center',
              gap: 8,
            }}
          >
            <span style={{ flex: 1, fontSize: 13, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {run.fileName}
            </span>
            <button
              onClick={(e) => { e.stopPropagation(); removeRun(run.id); }}
              style={{ fontSize: 11, color: 'var(--muted)', padding: '2px 4px' }}
            >
              ✕
            </button>
          </div>
        ))}
        {runs.length === 0 && (
          <div style={{ padding: 12, color: 'var(--muted)', fontSize: 13, textAlign: 'center' }}>
            Drop .ghostline files to begin
          </div>
        )}
      </div>
      <DropZone />
    </aside>
  );
}
