import { memo } from 'react';
import { Handle, Position, type NodeProps } from 'reactflow';
import type { Frame } from '../lib/types';

const COLORS: Record<string, string> = {
  llm: 'var(--accent-llm)',
  tool: 'var(--accent-tool)',
  error: 'var(--accent-error)',
  state: 'var(--muted)',
};

function TimelineNodeInner({ data }: NodeProps<{ frame: Frame; selected: boolean }>) {
  const { frame, selected } = data;
  const color = COLORS[frame.type] ?? 'var(--muted)';

  return (
    <div
      style={{
        background: 'var(--surface)',
        border: `2px solid ${selected ? color : 'var(--border)'}`,
        borderRadius: 8,
        padding: '8px 12px',
        minWidth: 120,
        cursor: 'pointer',
        transition: 'border-color 0.15s',
      }}
    >
      <Handle type="target" position={Position.Left} style={{ background: color, width: 8, height: 8 }} />
      <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
        <span
          style={{
            width: 8,
            height: 8,
            borderRadius: '50%',
            background: color,
            flexShrink: 0,
          }}
        />
        <span style={{ fontSize: 11, color: 'var(--muted)', textTransform: 'uppercase' }}>{frame.type}</span>
      </div>
      <div style={{ fontSize: 13, fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', maxWidth: 140 }}>
        {frame.name}
      </div>
      <div style={{ fontSize: 11, color: 'var(--muted)', fontFamily: 'var(--font-mono)', marginTop: 2 }}>
        {frame.duration_ms}ms
        {frame.tokens_in != null && ` · ${frame.tokens_in}→${frame.tokens_out ?? 0}tok`}
      </div>
      <Handle type="source" position={Position.Right} style={{ background: color, width: 8, height: 8 }} />
    </div>
  );
}

export const TimelineNode = memo(TimelineNodeInner);
