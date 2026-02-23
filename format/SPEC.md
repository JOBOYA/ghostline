# .ghostline Format Specification — v1

## Overview

A `.ghostline` file is a compact, append-optimized binary recording of LLM API calls. It supports O(1) random access to any frame via a trailing index.

## File Layout

```
┌────────────────────────────────────────────────────┐
│ HEADER                                             │
│   magic:       8 bytes  — "GHSTLINE"               │
│   version:     4 bytes  — u32 LE (currently 1)     │
│   started_at:  8 bytes  — u64 LE (unix ms)         │
│   has_git_sha: 1 byte   — 0x00 or 0x01             │
│   git_sha:     20 bytes — present if has_git_sha=1 │
├────────────────────────────────────────────────────┤
│ FRAMES (one per LLM call)                          │
│   frame_len:   4 bytes  — u32 LE (compressed size) │
│   frame_data:  N bytes  — zstd(MessagePack(Frame)) │
├────────────────────────────────────────────────────┤
│ INDEX                                              │
│   entries[]:  40 bytes each                        │
│     request_hash: 32 bytes — SHA-256               │
│     offset:        8 bytes — u64 LE (frame start)  │
│   entry_count: 4 bytes — u32 LE                    │
├────────────────────────────────────────────────────┤
│ INDEX POINTER                                      │
│   index_offset: 8 bytes — u64 LE (last 8 bytes)    │
└────────────────────────────────────────────────────┘
```

## Frame Schema (MessagePack)

```
Frame {
    request_hash:   [u8; 32]  — SHA-256(request_bytes)
    request_bytes:  bytes     — serialized LLM request
    response_bytes: bytes     — serialized LLM response
    latency_ms:     u64       — round-trip latency
    timestamp:      u64       — unix timestamp (ms)
}
```

## Replay Lookup

1. Read last 8 bytes → `index_offset`
2. Seek to `index_offset`, read `entry_count`
3. Binary search or linear scan index for matching `request_hash`
4. Seek to `offset`, read `frame_len`, decompress, deserialize

## Security

- API keys and secrets in `request_bytes` / `response_bytes` are **not** scrubbed automatically in v1
- Treat `.ghostline` files as sensitive — do not commit to public repos
- Future versions will support automatic secret scrubbing via configurable patterns
