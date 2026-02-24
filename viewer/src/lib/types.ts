export type NodeType = 'llm' | 'tool' | 'error' | 'state';

export interface Frame {
  idx: number;
  type: NodeType;
  name: string;
  timestamp: number;
  duration_ms: number;
  tokens_in?: number;
  tokens_out?: number;
  request?: unknown;
  response?: unknown;
  error?: string;
  meta?: Record<string, unknown>;
}

export interface Run {
  id: string;
  fileName: string;
  version: number;
  startedAt: number;
  sha?: string;
  frames: Frame[];
  fileSize: number;
}
