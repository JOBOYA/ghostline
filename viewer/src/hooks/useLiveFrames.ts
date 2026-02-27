import { useEffect, useRef } from 'react';

export interface LiveFrame {
  index: number;
  timestamp: string;
  request_size: number;
  response_size: number;
  latency_ms: number;
}

export function useLiveFrames(onFrame: (frame: LiveFrame) => void) {
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${protocol}//${location.host}/ws/live`);
    wsRef.current = ws;

    ws.onmessage = (event) => {
      try {
        const frame: LiveFrame = JSON.parse(event.data);
        onFrame(frame);
      } catch (e) {
        console.warn('Failed to parse live frame:', e);
      }
    };

    ws.onerror = (e) => console.warn('WS error:', e);

    return () => {
      ws.close();
    };
  }, []);

  return wsRef;
}
