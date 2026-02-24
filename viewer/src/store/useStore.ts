import { create } from 'zustand';
import type { Run } from '../lib/types';

interface State {
  runs: Run[];
  activeRunId: string | null;
  selectedFrameIdx: number | null;
  zoom: number;
  detailOpen: boolean;
  loadRun: (run: Run) => void;
  setActiveRun: (id: string) => void;
  selectFrame: (idx: number | null) => void;
  setZoom: (z: number) => void;
  toggleDetail: () => void;
  removeRun: (id: string) => void;
}

export const useStore = create<State>((set, get) => ({
  runs: [],
  activeRunId: null,
  selectedFrameIdx: null,
  zoom: 1,
  detailOpen: false,

  loadRun: (run) =>
    set((s) => ({
      runs: [...s.runs, run],
      activeRunId: run.id,
      selectedFrameIdx: null,
      detailOpen: false,
    })),

  setActiveRun: (id) =>
    set({ activeRunId: id, selectedFrameIdx: null, detailOpen: false }),

  selectFrame: (idx) => {
    const s = get();
    set({ selectedFrameIdx: idx, detailOpen: idx !== null ? true : s.detailOpen });
  },

  setZoom: (z) => set({ zoom: Math.max(0.25, Math.min(3, z)) }),

  toggleDetail: () => set((s) => ({ detailOpen: !s.detailOpen })),

  removeRun: (id) =>
    set((s) => {
      const runs = s.runs.filter((r) => r.id !== id);
      return {
        runs,
        activeRunId: s.activeRunId === id ? (runs[0]?.id ?? null) : s.activeRunId,
        selectedFrameIdx: s.activeRunId === id ? null : s.selectedFrameIdx,
      };
    }),
}));
