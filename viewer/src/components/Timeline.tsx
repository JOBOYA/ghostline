import { useMemo, useCallback } from 'react';
import ReactFlow, {
  Background,
  Controls,
  type Node,
  type Edge,
  type NodeTypes,
} from 'reactflow';
import 'reactflow/dist/style.css';
import { useStore } from '../store/useStore';
import { TimelineNode } from './TimelineNode';
import ghostSvg from '../assets/ghost-empty.svg';

const nodeTypes: NodeTypes = {
  ghostFrame: TimelineNode,
};

export function Timeline() {
  const runs = useStore((s) => s.runs);
  const activeRunId = useStore((s) => s.activeRunId);
  const selectedFrameIdx = useStore((s) => s.selectedFrameIdx);
  const selectFrame = useStore((s) => s.selectFrame);
  const zoom = useStore((s) => s.zoom);

  const activeRun = runs.find((r) => r.id === activeRunId);
  const frames = activeRun?.frames ?? [];

  const nodes: Node[] = useMemo(
    () =>
      frames.map((frame, i) => ({
        id: `f-${i}`,
        type: 'ghostFrame',
        position: { x: i * 200 * zoom, y: 0 },
        data: { frame, selected: i === selectedFrameIdx },
      })),
    [frames, selectedFrameIdx, zoom],
  );

  const edges: Edge[] = useMemo(
    () =>
      frames.slice(1).map((_, i) => ({
        id: `e-${i}`,
        source: `f-${i}`,
        target: `f-${i + 1}`,
        style: { stroke: 'var(--border)', strokeWidth: 2 },
        type: 'smoothstep',
      })),
    [frames],
  );

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      const idx = parseInt(node.id.split('-')[1], 10);
      selectFrame(idx);
    },
    [selectFrame],
  );

  if (!activeRun) {
    return (
      <div
        style={{
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 16,
          color: 'var(--muted)',
          fontSize: 15,
        }}
      >
        <img src={ghostSvg} alt="" width={96} height={96} style={{ opacity: 0.6 }} />
        <div>Drop a <code style={{ fontFamily: 'var(--font-mono)', color: 'var(--accent-llm)' }}>.ghostline</code> file to view its timeline</div>
      </div>
    );
  }

  return (
    <div style={{ flex: 1, background: 'var(--bg)' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodeClick={onNodeClick}
        fitView
        proOptions={{ hideAttribution: true }}
        style={{ background: 'var(--bg)' }}
      >
        <Background color="var(--border)" gap={24} size={1} />
        <Controls
          style={{ background: 'var(--surface)', borderColor: 'var(--border)' }}
        />
      </ReactFlow>
    </div>
  );
}
