import { decompress } from 'fzstd';
import { decode } from '@msgpack/msgpack';
import type { Frame, Run, NodeType } from './types';

// GHSTLINE in ASCII
const MAGIC = new Uint8Array([0x47, 0x48, 0x53, 0x54, 0x4C, 0x49, 0x4E, 0x45]);

function readU32LE(view: DataView, offset: number): number {
  return view.getUint32(offset, true);
}

function readU64LE(view: DataView, offset: number): number {
  const lo = view.getUint32(offset, true);
  const hi = view.getUint32(offset + 4, true);
  return hi * 0x100000000 + lo;
}

function bytesToString(b: unknown): string {
  if (b instanceof Uint8Array) return new TextDecoder().decode(b);
  if (typeof b === 'string') return b;
  return '';
}

function tryParseJson(raw: string): unknown {
  try { return JSON.parse(raw); } catch { return raw; }
}

function classifyFrame(requestStr: string): NodeType {
  const lower = requestStr.toLowerCase();
  if (lower.includes('tool') || lower.includes('function_call')) return 'tool';
  if (lower.includes('model') || lower.includes('messages') || lower.includes('prompt')) return 'llm';
  return 'state';
}

function extractFrameData(decoded: unknown, idx: number): Frame {
  let requestBytes: Uint8Array = new Uint8Array();
  let responseBytes: Uint8Array = new Uint8Array();
  let latencyMs = 0;
  let timestamp = 0;

  if (Array.isArray(decoded)) {
    // Rust rmp_serde array format: [request_hash, request_bytes, response_bytes, latency_ms, timestamp]
    requestBytes  = decoded[1] instanceof Uint8Array ? decoded[1] : new Uint8Array();
    responseBytes = decoded[2] instanceof Uint8Array ? decoded[2] : new Uint8Array();
    latencyMs     = Number(decoded[3] ?? 0);
    timestamp     = Number(decoded[4] ?? 0);
  } else if (decoded && typeof decoded === 'object') {
    // Python SDK map format: {request_bytes, response_bytes, latency_ms, timestamp, request_hash}
    const m = decoded as Record<string, unknown>;
    requestBytes  = m['request_bytes']  instanceof Uint8Array ? m['request_bytes']  : new Uint8Array();
    responseBytes = m['response_bytes'] instanceof Uint8Array ? m['response_bytes'] : new Uint8Array();
    latencyMs     = Number(m['latency_ms']  ?? 0);
    timestamp     = Number(m['timestamp']   ?? 0);
  }

  const requestStr  = bytesToString(requestBytes);
  const responseStr = bytesToString(responseBytes);

  return {
    idx,
    type: classifyFrame(requestStr),
    name: `frame_${idx}`,
    timestamp,
    duration_ms: latencyMs,
    request:  tryParseJson(requestStr),
    response: tryParseJson(responseStr),
    meta: {},
  };
}

export async function parseGhostline(buffer: ArrayBuffer, fileName: string): Promise<Run> {
  const bytes = new Uint8Array(buffer);
  const view  = new DataView(buffer);

  // Validate magic
  for (let i = 0; i < 8; i++) {
    if (bytes[i] !== MAGIC[i]) {
      throw new Error(`Invalid .ghostline file: bad magic (got ${bytes.slice(0, 8)})`);
    }
  }

  let offset = 8;
  const version   = readU32LE(view, offset); offset += 4;
  const startedAt = readU64LE(view, offset); offset += 8;
  const hasSha    = bytes[offset];           offset += 1;

  let sha: string | undefined;
  if (hasSha) {
    sha = Array.from(bytes.slice(offset, offset + 20))
      .map(b => b.toString(16).padStart(2, '0'))
      .join('');
    offset += 20;
  }

  const frames: Frame[] = [];
  let idx = 0;
  // Stop before last 8 bytes (index_offset) and leave room for index entries
  const stopAt = buffer.byteLength - 8;

  while (offset + 4 < stopAt) {
    const frameLen = readU32LE(view, offset);
    if (frameLen === 0 || offset + 4 + frameLen > stopAt) break;
    offset += 4;

    const compressed = bytes.slice(offset, offset + frameLen);
    offset += frameLen;

    try {
      const decompressed = decompress(compressed);
      const decoded      = decode(decompressed);
      frames.push(extractFrameData(decoded, idx++));
    } catch {
      // Reached index area — stop
      break;
    }
  }

  return {
    id: crypto.randomUUID(),
    fileName,
    version,
    startedAt,
    sha,
    frames,
    fileSize: buffer.byteLength,
  };
}
