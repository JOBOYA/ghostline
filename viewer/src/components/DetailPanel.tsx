import { useState, useCallback } from 'react';
import { useStore } from '../store/useStore';

const REDACT_PATTERNS = [/sk-[A-Za-z0-9_-]+/g, /Bearer\s+\S+/gi, /api[_-]?key["\s:=]+\S+/gi];

function redact(text: string): string {
  let out = text;
  for (const pat of REDACT_PATTERNS) {
    out = out.replace(pat, '[REDACTED]');
  }
  return out;
}

function JsonBlock({ label, data, showRaw }: { label: string; data: unknown; showRaw: boolean }) {
  const text = JSON.stringify(data, null, 2) ?? '';
  const display = showRaw ? text : redact(text);

  const copy = useCallback(() => {
    navigator.clipboard.writeText(display).catch(() => {});
  }, [display]);

  return (
    <div style={{ marginBottom: 16 }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 6 }}>
        <span style={{ fontSize: 11, color: 'var(--muted)', textTransform: 'uppercase' }}>{label}</span>
        <button onClick={copy} style={{ fontSize: 11, color: 'var(--accent-tool)', padding: '2px 6px' }}>
          Copy
        </button>
      </div>
      <pre
        style={{
          background: 'var(--bg)',
          border: '1px solid var(--border)',
          borderRadius: 6,
          padding: 10,
          fontSize: 12,
          fontFamily: 'var(--font-mono)',
          maxHeight: 300,
          overflow: 'auto',
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-all',
        }}
      >
        {display || '—'}
      </pre>
    </div>
  );
}

export function DetailPanel() {
  const detailOpen = useStore((s) => s.detailOpen);
  const selectedFrameIdx = useStore((s) => s.selectedFrameIdx);
  const runs = useStore((s) => s.runs);
  const activeRunId = useStore((s) => s.activeRunId);
  const toggleDetail = useStore((s) => s.toggleDetail);
  const [showRaw, setShowRaw] = useState(false);

  const activeRun = runs.find((r) => r.id === activeRunId);
  const frame = activeRun && selectedFrameIdx !== null ? activeRun.frames[selectedFrameIdx] : null;

  if (!detailOpen || !frame) return null;

  return (
    <aside
      style={{
        width: 'var(--detail-w)',
        background: 'var(--surface)',
        borderLeft: '1px solid var(--border)',
        display: 'flex',
        flexDirection: 'column',
        flexShrink: 0,
        overflow: 'hidden',
      }}
    >
      <div style={{ padding: '12px 16px', borderBottom: '1px solid var(--border)', display: 'flex', alignItems: 'center', gap: 8 }}>
        <span style={{ flex: 1, fontWeight: 600, fontSize: 14 }}>Frame #{frame.idx}</span>
        <button
          onClick={() => setShowRaw(!showRaw)}
          style={{ fontSize: 11, padding: '3px 8px', borderRadius: 4, background: 'var(--border)' }}
        >
          {showRaw ? 'Redact' : 'Show raw'}
        </button>
        <button onClick={toggleDetail} style={{ fontSize: 14, padding: '2px 6px' }}>✕</button>
      </div>
      <div style={{ flex: 1, overflowY: 'auto', padding: 16 }}>
        <div style={{ marginBottom: 16 }}>
          <div style={{ fontSize: 13, marginBottom: 4 }}>
            <strong>{frame.name}</strong>
          </div>
          <div style={{ fontSize: 12, color: 'var(--muted)', fontFamily: 'var(--font-mono)' }}>
            {frame.type} · {frame.duration_ms}ms
            {frame.tokens_in != null && ` · ${frame.tokens_in} in / ${frame.tokens_out ?? 0} out`}
          </div>
          {frame.error && (
            <div style={{ marginTop: 8, padding: 8, background: 'rgba(239,68,68,0.1)', borderRadius: 6, color: 'var(--accent-error)', fontSize: 12, fontFamily: 'var(--font-mono)' }}>
              {frame.error}
            </div>
          )}
        </div>
        {frame.request != null && <JsonBlock label="Request" data={frame.request} showRaw={showRaw} />}
        {frame.response != null && <JsonBlock label="Response" data={frame.response} showRaw={showRaw} />}
        {frame.meta != null && <JsonBlock label="Meta" data={frame.meta} showRaw={showRaw} />}
      </div>
    </aside>
  );
}
