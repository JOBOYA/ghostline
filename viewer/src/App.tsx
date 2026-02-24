import { useEffect } from 'react';
import { useStore } from './store/useStore';
import { Topbar } from './components/Topbar';
import { SidePanel } from './components/SidePanel';
import { Timeline } from './components/Timeline';
import { DetailPanel } from './components/DetailPanel';
import { StatusBar } from './components/StatusBar';

export default function App() {
  const selectFrame = useStore((s) => s.selectFrame);
  const selectedFrameIdx = useStore((s) => s.selectedFrameIdx);
  const runs = useStore((s) => s.runs);
  const activeRunId = useStore((s) => s.activeRunId);
  const toggleDetail = useStore((s) => s.toggleDetail);
  const detailOpen = useStore((s) => s.detailOpen);
  const setZoom = useStore((s) => s.setZoom);

  const activeRun = runs.find((r) => r.id === activeRunId);
  const frameCount = activeRun?.frames.length ?? 0;

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'j' || e.key === 'ArrowRight') {
        const next = Math.min((selectedFrameIdx ?? -1) + 1, frameCount - 1);
        if (frameCount > 0) selectFrame(next);
      } else if (e.key === 'k' || e.key === 'ArrowLeft') {
        const prev = Math.max((selectedFrameIdx ?? 1) - 1, 0);
        if (frameCount > 0) selectFrame(prev);
      } else if (e.key === 'Enter') {
        if (selectedFrameIdx !== null && !detailOpen) toggleDetail();
      } else if (e.key === 'Escape') {
        if (detailOpen) toggleDetail();
      } else if (e.key === '0') {
        setZoom(1);
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [selectFrame, selectedFrameIdx, frameCount, toggleDetail, detailOpen, setZoom]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh' }}>
      <Topbar />
      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        <SidePanel />
        <Timeline />
        <DetailPanel />
      </div>
      <StatusBar />
    </div>
  );
}
