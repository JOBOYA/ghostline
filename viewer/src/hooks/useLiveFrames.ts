import { useEffect, useRef } from 'react';

export interface LiveFrame {
  index: number;
  run_name: string;
  timestamp: string;
  request_size: number;
  response_size: number;
  latency_ms: number;
}

export function useLiveFrames(onFrame: (frame: LiveFrame) => void) {
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    // Only connect when served from the Ghostline binary (not standalone HTML export)
    if (!location.host || location.protocol === 'file:') return;

    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${protocol}//${location.host}/ws/live`);
    wsRef.current = ws;

    ws.onopen = () => console.info('[ghostline] WS live stream connected');
    ws.onclose = () => console.info('[ghostline] WS live stream disconnected');

    ws.onmessage = (event) => {
      try {
        const frame: LiveFrame = JSON.parse(event.data);
        onFrame(frame);
      } catch (e) {
        console.warn('[ghostline] Failed to parse live frame:', e);
      }
    };

    ws.onerror = () => {
      // Silently fail — viewer may be opened as standalone HTML export
    };

    return () => {
      ws.close();
    };
  }, []);

  return wsRef;
}
