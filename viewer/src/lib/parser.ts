import { decompress } from 'fflate';
import { decode } from '@msgpack/msgpack';
import type { Frame, Run, NodeType } from './types';

const MAGIC = new Uint8Array([0x47, 0x48, 0x4F, 0x53, 0x54, 0x4C, 0x4E, 0x00]); // GHOSTLN\0

function readU32LE(view: DataView, offset: number): number {
  return view.getUint32(offset, true);
}

function readU64LE(view: DataView, offset: number): number {
  const lo = view.getUint32(offset, true);
  const hi = view.getUint32(offset + 4, true);
  return hi * 0x100000000 + lo;
}

function classifyFrame(raw: Record<string, unknown>): NodeType {
  if (raw['error']) return 'error';
  const t = String(raw['type'] ?? raw['kind'] ?? '').toLowerCase();
  if (t.includes('tool') || t.includes('function')) return 'tool';
  if (t.includes('llm') || t.includes('chat') || t.includes('completion')) return 'llm';
  return 'state';
}

function decompressAsync(data: Uint8Array): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    decompress(data, (err, result) => {
      if (err) reject(err);
      else resolve(result);
    });
  });
}

export async function parseGhostline(buffer: ArrayBuffer, fileName: string): Promise<Run> {
  const bytes = new Uint8Array(buffer);
  const view = new DataView(buffer);

  // Validate magic
  for (let i = 0; i < 8; i++) {
    if (bytes[i] !== MAGIC[i]) {
      throw new Error('Invalid .ghostline file: bad magic bytes');
    }
  }

  let offset = 8;
  const version = readU32LE(view, offset); offset += 4;
  const startedAt = readU64LE(view, offset); offset += 8;
  const hasSha = bytes[offset]; offset += 1;

  let sha: string | undefined;
  if (hasSha) {
    sha = Array.from(bytes.slice(offset, offset + 20))
      .map(b => b.toString(16).padStart(2, '0'))
      .join('');
    offset += 20;
  }

  // Read frames until we hit the index offset region
  const indexOffsetPos = buffer.byteLength - 8;
  const frames: Frame[] = [];
  let idx = 0;

  while (offset < indexOffsetPos - 4) {
    if (offset + 4 > buffer.byteLength) break;
    const frameLen = readU32LE(view, offset); offset += 4;
    if (frameLen === 0 || offset + frameLen > buffer.byteLength) break;

    const compressed = bytes.slice(offset, offset + frameLen);
    offset += frameLen;

    try {
      const decompressed = await decompressAsync(compressed);
      const raw = decode(decompressed) as Record<string, unknown>;

      frames.push({
        idx,
        type: classifyFrame(raw),
        name: String(raw['name'] ?? raw['type'] ?? raw['kind'] ?? `frame_${idx}`),
        timestamp: Number(raw['timestamp'] ?? raw['ts'] ?? 0),
        duration_ms: Number(raw['duration_ms'] ?? raw['duration'] ?? 0),
        tokens_in: raw['tokens_in'] != null ? Number(raw['tokens_in']) : undefined,
        tokens_out: raw['tokens_out'] != null ? Number(raw['tokens_out']) : undefined,
        request: raw['request'] ?? raw['input'],
        response: raw['response'] ?? raw['output'],
        error: raw['error'] != null ? String(raw['error']) : undefined,
        meta: raw['meta'] as Record<string, unknown> | undefined,
      });
    } catch {
      // If decompression fails, the remaining bytes might be index data
      break;
    }

    idx++;
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
