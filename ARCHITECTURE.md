# Ghostline — Architecture

This document describes the internal structure of Ghostline: how the binary format works,
how the components fit together, and the design decisions that shaped each layer.

---

## Overview

Ghostline is a deterministic replay debugger for AI agents. It sits between your code and
the LLM API, records every request/response pair as a binary `.ghostline` file, and lets you
replay any run with zero API calls — returning the exact cached responses.

The system is composed of four independent layers that can be used separately or together:

```
┌─────────────────────────────────────────────────────────────┐
│                        User code                            │
│         (Python, TypeScript, CLI, any HTTP client)          │
└──────────────────────┬──────────────────────────────────────┘
                       │ HTTP / SDK calls
┌──────────────────────▼──────────────────────────────────────┐
│              ghostline-cli  (Rust binary)                    │
│   ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│   │ Proxy server │  │ Replay server│  │  Viewer server   │  │
│   │  (hyper 0.14)│  │  (hyper 0.14)│  │  (axum + embed)  │  │
│   └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘  │
│          │ writes          │ reads              │ WebSocket   │
└──────────┼─────────────────┼────────────────────┼────────────┘
           │                 │                    │
┌──────────▼─────────────────▼────────────────────┼────────────┐
│              ghostline-core  (Rust crate)        │            │
│   GhostlineWriter · GhostlineReader · Frame     │            │
│   Binary format: magic + header + zstd frames   │            │
│   + tail index                                  │            │
└─────────────────────────────────────────────────┼────────────┘
           │                                      │
┌──────────▼──────────────────┐   ┌──────────────▼─────────────┐
│    Python SDK  (sdk/)        │   │  React viewer (viewer/)     │
│  wrap · record · replay      │   │  parser.ts · ReactFlow      │
│  scrub · search · fork        │   │  Timeline · DetailPanel     │
└──────────────────────────────┘   └────────────────────────────┘
```

---

## Binary format — `.ghostline`

The `.ghostline` format is the single source of truth that all components share.
It is designed for:

- **Append-only writes**: frames are written sequentially; no seek during recording.
- **Random-access reads**: a tail index enables O(1) frame lookup by position or hash.
- **Deterministic replay**: each frame stores a SHA-256 hash of its request bytes,
  used as the lookup key during replay.
- **Space efficiency**: frame payload is compressed with zstd level 3.
- **Portability**: little-endian fixed-width integers only; no platform-specific encoding.

### File layout

```
offset 0
│
├── [Header]
│     8 bytes  magic         "GHSTLINE" (0x47 0x48 0x53 0x54 0x4C 0x49 0x4E 0x45)
│     4 bytes  version       u32 LE — currently 1
│     8 bytes  started_at    u64 LE — Unix timestamp in milliseconds
│     1 byte   has_sha       0x00 or 0x01
│    20 bytes  git_sha       present only if has_sha == 1 (raw SHA-1)
│     1 byte   has_fork      0x00 or 0x01
│    32 bytes  parent_run_id present only if has_fork == 1 (SHA-256 of parent lineage)
│     4 bytes  fork_at_step  present only if has_fork == 1 (u32 LE frame index)
│
├── [Frame 0]
│     4 bytes  compressed_len  u32 LE
│     N bytes  compressed_data zstd-compressed MessagePack-encoded Frame
│
├── [Frame 1] ...
│
├── [Frame K-1]
│
├── [Index]
│     For each frame i (0..K-1):
│       32 bytes  request_hash  SHA-256 of frame i's request bytes
│        8 bytes  offset        u64 LE — byte offset of frame i from file start
│     4 bytes  entry_count  u32 LE — number of index entries (= K)
│
└── [Footer]
      8 bytes  index_offset  u64 LE — byte offset where the index begins
```

### Frame payload

Each frame is MessagePack-encoded as a struct with five fields:

| Field          | Type       | Description                                      |
|----------------|------------|--------------------------------------------------|
| `request_hash` | `[u8; 32]` | SHA-256 of `request_bytes`                       |
| `request_bytes`| `Vec<u8>`  | Raw HTTP body sent to the LLM API                |
| `response_bytes`| `Vec<u8>` | Raw HTTP body received from the LLM API          |
| `latency_ms`   | `u64`      | Round-trip time in milliseconds                  |
| `timestamp`    | `u64`      | Unix timestamp in milliseconds when captured     |

The `request_hash` is computed before compression and stored both inside the frame
(for self-verification) and in the tail index (for O(1) replay lookup).

### Hash computation

```
request_hash = SHA-256(request_bytes)
```

This is deterministic: identical API calls (same model, messages, parameters) produce
identical hashes regardless of when they are executed, making replay reliable across
sessions, machines, and time.

### Fork lineage

When a run is forked at step N, the new file's header records:

```
parent_run_id = SHA-256(started_at_bytes || first_frame.request_hash)
fork_at_step  = N  (u32)
```

This allows the viewer and tooling to reconstruct the full branching tree from the
files alone, without a separate metadata store.

---

## `ghostline-core` — Rust crate

Location: `crates/ghostline-core/`

The core crate is the single implementation of the binary format. Both the CLI and
third-party tools that embed Ghostline directly use this crate.

### Key types

**`Frame`** (`src/frame.rs`)
- Plain data struct: `request_hash`, `request_bytes`, `response_bytes`, `latency_ms`,
  `timestamp`.
- `Frame::new()` computes `request_hash` automatically via `Frame::hash_request()`.
- `to_msgpack()` / `from_msgpack()` — MessagePack round-trip via `rmp_serde`.

**`GhostlineWriter<W: Write>`** (`src/writer.rs`)
- Streaming, append-only writer. No `Seek` required on the underlying `W`.
- `new(inner, header)` — writes the header immediately.
- `append(frame)` — compresses with zstd level 3, writes `[len: u32][data]`, records
  offset in an in-memory index.
- `finish()` — flushes the tail index and the 8-byte footer. Must be called; dropping
  without calling `finish()` produces a truncated file that readers will reject.

**`GhostlineReader<R: Read + Seek>`** (`src/reader.rs`)
- Random-access reader backed by any `Read + Seek` source.
- `open(path)` — convenience constructor wrapping `BufReader<File>`.
- Construction reads header and tail index; subsequent `get_frame(i)` seeks directly
  to the frame offset without scanning.
- `lookup_by_hash(hash)` — linear scan over the in-memory index, then a single seek
  to the matching frame. Suitable for replay workloads where the frame count is small
  (typical agent runs: tens to low hundreds of frames).
- Exposes fork metadata (`parent_run_id`, `fork_at_step`) for tooling.

### Testing

12 unit tests covering: round-trip write/read, hash determinism, fork metadata, hash
lookup, hash-not-found, per-field byte verification.

---

## `ghostline-cli` — Rust binary

Location: `crates/ghostline-cli/`

The CLI is the primary user-facing artifact. It is a self-contained single binary
that embeds the React viewer at compile time via `rust-embed`.

### Command surface

| Command | Description |
|---------|-------------|
| `ghostline` (default) | Wizard if unconfigured, else launch proxy + viewer |
| `ghostline run <cmd>` | Start proxy + viewer, run `<cmd>` with `ANTHROPIC_BASE_URL` set |
| `ghostline record` | Alias for the default launch |
| `ghostline replay <file>` | Replay proxy — serves cached responses |
| `ghostline viewer` | Start the embedded viewer without proxy |
| `ghostline proxy` | Raw proxy mode (no viewer) |
| `ghostline inspect <file>` | Print header + frame list |
| `ghostline show <file> <n>` | Print frame N with payload preview |
| `ghostline export <file> --format html` | Export standalone HTML viewer |
| `ghostline fork <file> --at <n>` | Fork run at step N |
| `ghostline search <file> <query>` | Semantic search (delegates to Python SDK) |
| `ghostline runs` | List recorded sessions |
| `ghostline doctor` | Health check (config, ports, runs dir) |
| `ghostline config show/set` | Read/write TOML config |

### Modules

**`proxy.rs`** — Transparent recording proxy using `hyper 0.14`.

Listens on `127.0.0.1:<port>` (never `0.0.0.0`). For each incoming request:
1. Strip hop-by-hop headers (`host`, `connection`, `transfer-encoding`).
2. Forward to the configured target (default: `https://api.anthropic.com`).
3. On response: create a `Frame`, append it to the `GhostlineWriter` under a
   `tokio::sync::Mutex`, broadcast a JSON summary to the WebSocket channel,
   then forward the response to the caller.
4. On `Ctrl-C`: graceful shutdown calls `writer.finish()`.

The proxy adds `x-ghostline-proxy: true` to every forwarded response so callers
can detect they are being recorded.

**`replay.rs`** — Deterministic replay server.

Loads all frames from a `.ghostline` file into a `HashMap<[u8; 32], Frame>` at
startup. Serves the same `hyper` service loop as the proxy, but instead of
forwarding:
1. Hash the incoming request body with SHA-256.
2. Look up the hash in the map.
3. Return the cached `response_bytes` with HTTP 200, or a 404 with a JSON error
   if not found.

This means any run recorded against a specific set of inputs can be replayed
offline with zero API calls and zero latency variance.

**`viewer_server.rs`** — Embedded HTTP + WebSocket server using `axum`.

Routes:
- `GET /` and `GET /assets/*` — serve the embedded React build (via `rust-embed`).
- `GET /api/runs` — list `.ghostline` files in the runs directory.
- `GET /api/runs/:name` — parse and return a run as JSON (header + frame list).
- `GET /api/runs/:name/frames` — return all frames as JSON.
- `GET /api/status` — current frame count (atomic integer, updated by proxy).
- `GET /ws/live` — WebSocket upgrade; broadcasts frame events as they arrive.

CORS is restricted to `localhost` origins only. The viewer is designed to run
locally; it must not be exposed on a network interface.

**`wizard.rs`** — Interactive first-run wizard.

Prompts for a Claude Code token and writes `~/.config/ghostline/config.toml`.
Runs automatically on first launch if no config is found.

**`config.rs`** — TOML config with serde.

```toml
[proxy]
port   = 9000
target = "https://api.anthropic.com"

[viewer]
port              = 5173
auto_open_browser = true

[recording]
scrub = true

[display]
colors = true
```

**`viewer_assets.rs`** — `rust-embed` statics.

The entire `viewer/dist/` directory is embedded at compile time. The binary
ships with no external dependencies for the viewer.

### Concurrency model

The proxy and viewer server run as two `tokio::spawn` tasks under a single
`tokio::runtime::Runtime`. The shared state is:

- `Arc<Mutex<ProxyState>>` — the writer and frame count, guarded per-request.
- `Arc<AtomicUsize>` — frame count exposed to the viewer's `/api/status` route
  without locking.
- `broadcast::Sender<String>` — zero-copy fan-out of frame events to all open
  WebSocket connections.

---

## Python SDK — `sdk/`

The SDK provides a Pythonic interface over the same binary format.

### `format.py` — Binary format in Python

Implements `GhostlineWriter` and `GhostlineReader` in pure Python, mirroring the
Rust implementation byte-for-byte. Uses `struct.pack` / `struct.unpack` for
fixed-width fields, `msgpack` for frame serialization, and `zstd` for compression.

Cross-compatibility is verified by tests: a file written by the Rust CLI is readable
by the Python reader, and vice versa.

The `fork(path, at_step, output)` function replicates the CLI's `fork` command.

### `recorder.py` — Recording context

`GhostlineRecorder` wraps an `anthropic.Anthropic` or `openai.OpenAI` client,
monkey-patching its `.messages.create()` / `.chat.completions.create()` to
intercept calls and write frames to disk.

`scrub=True` (the default) applies the scrubbing layer to both request and response
bytes before writing.

### `replayer.py` — Replay context

`GhostlineReplayer` loads all frames into a `dict[bytes, Frame]` and monkey-patches
the same client methods to return cached responses. If a request hash is not found
in the cache, it raises `KeyError` rather than falling through to a live API call —
replay is strict by design.

### `wrapper.py` — Zero-code-change wrapping

```python
import anthropic, ghostline

client = ghostline.wrap(anthropic.Anthropic())
# All subsequent calls are recorded transparently
```

`wrap()` detects the client type (`anthropic` or `litellm`) and applies the
appropriate monkey-patch. The wrapped client is returned; the caller's code
needs no other changes.

### `context.py` — Context managers

```python
with ghostline.record("run.ghostline"):
    response = client.messages.create(...)

with ghostline.replay("run.ghostline"):
    response = client.messages.create(...)  # returns cached response
```

Both managers wrap/unwrap the default global client automatically.

### `scrub.py` — Sensitive data redaction

`ScrubConfig` holds a list of `(regex, replacement)` pairs. Default patterns cover:

- Anthropic, OpenAI, Stripe, and generic `sk-*` API keys
- AWS access key IDs and secret access keys
- GitHub personal access tokens (`ghp_`, `gho_`, `github_pat_`)
- Generic `Bearer <token>` authorization headers
- Email addresses
- Base64-encoded secrets in common key/value patterns

`scrub(text, config)` applies all patterns in order. The function is called on
the UTF-8 decoded request and response bytes before they reach the writer.

### `search.py` — Semantic search

`GhostlineIndex` indexes one or more `.ghostline` files for natural-language
search across frame content.

**Embedding strategy:**
- Default: hash-based bag-of-tokens projected into a 256-dimensional vector
  (feature hashing, zero dependencies, works offline).
- Custom: any `embed_fn: str → np.ndarray` can be passed at construction time
  to use a real embedding model (e.g., `sentence-transformers`).

**Backend selection (automatic):**
1. `zvec` (alibaba/zvec, Python 3.10-3.12) — uses a native vector store if
   available.
2. NumPy fallback — stacks all vectors into a matrix and computes cosine
   similarity in one `@` operation. Used on Python 3.13+ where `zvec` is
   not yet available.

### `export_html.py`

`export_html(path, output)` reads a `.ghostline` file, base64-encodes it, and
inlines it into a standalone HTML file alongside the compiled viewer JS and CSS.
The resulting file opens in any browser with no server required.

---

## React viewer — `viewer/`

Location: `viewer/`  
Stack: React 18, TypeScript, ReactFlow, Zustand, Vite

The viewer is a single-page application that renders a `.ghostline` run as an
interactive timeline. It operates in two modes:

1. **File drop**: the user drags a `.ghostline` file onto the dropzone; `parser.ts`
   parses it client-side.
2. **Live mode**: when served by `ghostline-cli`, the viewer connects to
   `ws://localhost:<port>/ws/live` and receives frame events in real time, appending
   nodes to the timeline as they arrive.
3. **Server mode**: when served by `ghostline-cli`, the viewer fetches
   `/api/runs` and `/api/runs/:name/frames` to load past sessions without file drag.

### `lib/parser.ts` — Binary format in TypeScript

Implements the same parsing logic as `ghostline-core` and `format.py`, using:
- Manual `DataView` reads for the header (little-endian integers).
- `fzstd` for zstd decompression.
- `@msgpack/msgpack` for MessagePack desoding.

The parser handles both frame encodings transparently:
- **Rust array format**: `rmp_serde` serializes structs as arrays by default →
  `[request_hash, request_bytes, response_bytes, latency_ms, timestamp]`.
- **Python map format**: Python `msgpack` serializes dicts as maps →
  `{request_bytes: ..., response_bytes: ..., ...}`.

Frame classification (`classifyFrame`) inspects the request text to assign a
`NodeType` (`llm`, `tool`, or `state`) used by ReactFlow for node styling.

### Component tree

```
App
├── Topbar           — title, file name, run selector
├── DropZone         — drag-and-drop overlay, active when no run is loaded
├── Timeline         — ReactFlow canvas, one node per frame
│   └── TimelineNode — styled node (color by NodeType, latency badge)
├── SidePanel        — frame list with timestamps and latency
└── DetailPanel      — selected frame: request + response JSON, latency, hash
    └── SecretsWarning — banner shown when redaction patterns are detected
StatusBar            — frame count, file size, connection status
```

### State management

A single Zustand store (`store/useStore.ts`) holds:
- The active `Run` (parsed header + frames array)
- The selected frame index
- Live mode connection state
- The list of available runs from the server

### Live frame hook

`hooks/useLiveFrames.ts` opens a `WebSocket` to `/ws/live` and appends incoming
frame summaries to the store. Full frame data is fetched lazily from
`/api/runs/:name/frames` when the user selects a frame.

`hooks/useAutoLoadRuns.ts` polls `/api/runs` once on mount and when the
WebSocket receives a new frame, keeping the run selector in sync.

---

## Data flow — recording

```
User code
  │
  │  ANTHROPIC_BASE_URL=http://localhost:9000
  │
  ▼
ghostline proxy (127.0.0.1:9000)
  │
  │  1. Receive request body
  │  2. Forward to https://api.anthropic.com
  │  3. Wait for response
  │
  ▼
Frame::new(request_bytes, response_bytes, latency_ms, timestamp)
  │
  │  SHA-256(request_bytes) → request_hash
  │
  ├──► GhostlineWriter.append(frame)
  │       zstd::compress(msgpack(frame)) → [len: u32][data: N bytes]
  │       record offset in index
  │
  └──► broadcast::Sender.send(frame_summary_json)
           ▼
       WebSocket clients (viewer)
```

On `Ctrl-C`, `writer.finish()` flushes the tail index and the 8-byte footer.

---

## Data flow — replay

```
User code
  │
  │  ANTHROPIC_BASE_URL=http://localhost:8384
  │
  ▼
ghostline replay proxy (127.0.0.1:8384)
  │
  │  1. Receive request body
  │  2. SHA-256(request_bytes) → hash
  │  3. Look up hash in HashMap<[u8;32], Frame>
  │
  ├── HIT  → return frame.response_bytes with HTTP 200 (0ms network latency)
  └── MISS → return HTTP 404 {"error":"frame not found","hash":"..."}
```

The replay proxy never makes a live API call. If a request was not part of the
original recorded run, the replay will fail explicitly rather than silently
producing a live response mixed with cached data.

---

## Design decisions

### Why a binary format instead of JSON?

Frame payloads are raw HTTP bodies — often MessagePack-encoded LLM requests and
responses. Storing them in JSON would require double-encoding (JSON inside JSON)
or base64, both of which inflate size and complicate streaming writes. The binary
format is append-only, which makes it safe to record long agent runs without ever
seeking backwards.

### Why a tail index instead of a prefix index?

A prefix index would require knowing the number of frames before writing begins,
which is impossible for streaming captures. The tail index is written once at
`finish()` time and enables the same O(1) lookup. The file is unusable until
`finish()` is called, which is the correct semantic: a partial recording is not
a valid recording.

### Why zstd over gzip?

zstd level 3 gives better compression ratios than gzip at higher throughput.
LLM request/response bodies contain repetitive JSON structures that compress
well. At typical frame sizes (1–50 KB), zstd decompresses fast enough that
random frame access remains interactive in the viewer.

### Why SHA-256 for the request hash?

The hash is the replay key. It must be collision-resistant (two different requests
must not produce the same key) and deterministic (same request always produces the
same key). SHA-256 satisfies both. The hash is computed over the raw request bytes
before any scrubbing, so scrubbed recordings can still be replayed correctly
(the replay proxy receives the original request bytes from the live client).

### Why hyper 0.14 for the proxy?

The proxy needs low-level control over header forwarding and body streaming.
`hyper 0.14` gives direct access to request/response internals without the
abstraction overhead of higher-level frameworks. The viewer server uses `axum`
(built on hyper 1.x), which is appropriate for route-based API serving.

### Why embed the viewer in the binary?

The primary install path is `cargo install ghostline` or a single binary download.
Requiring a separate `npm install && npm run build` step would significantly raise
the barrier to first use. Embedding the compiled viewer trades binary size (~325 KB
gzip) for a zero-step install experience.

### Why cross-compile the binary format across three languages?

The format is implemented three times (Rust, Python, TypeScript) because each
layer has a different deployment context:
- Rust: performance-critical recording/replay path, distributed as a binary.
- Python: integration with the Python LLM ecosystem (anthropic, litellm, langchain).
- TypeScript: browser-side parsing — cannot call native Rust from a web page.

All three implementations are tested for cross-compatibility. The spec is in
`format/SPEC.md`.

---

## Repository structure

```
ghostline/
├── Cargo.toml                    # workspace manifest
├── crates/
│   ├── ghostline-core/           # binary format: Frame, Writer, Reader
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── frame.rs
│   │       ├── writer.rs
│   │       └── reader.rs
│   └── ghostline-cli/            # CLI binary + embedded servers
│       └── src/
│           ├── main.rs           # command dispatch
│           ├── proxy.rs          # recording proxy (hyper)
│           ├── replay.rs         # replay proxy (hyper)
│           ├── viewer_server.rs  # viewer API + WebSocket (axum)
│           ├── viewer_assets.rs  # rust-embed statics
│           ├── wizard.rs         # first-run setup
│           ├── config.rs         # TOML config
│           └── banner.rs         # terminal output
├── sdk/                          # Python SDK
│   ├── ghostline/
│   │   ├── __init__.py
│   │   ├── format.py             # binary format (Python)
│   │   ├── recorder.py           # recording wrapper
│   │   ├── replayer.py           # replay wrapper
│   │   ├── wrapper.py            # zero-code-change wrap()
│   │   ├── context.py            # record()/replay() context managers
│   │   ├── scrub.py              # sensitive data redaction
│   │   ├── search.py             # semantic search (zvec / numpy)
│   │   └── export_html.py        # standalone HTML export
│   └── tests/
│       ├── test_format.py
│       ├── test_record_replay.py
│       ├── test_scrub.py
│       └── test_search.py
├── viewer/                       # React SPA
│   └── src/
│       ├── App.tsx
│       ├── lib/
│       │   ├── parser.ts         # binary format (TypeScript)
│       │   └── types.ts
│       ├── components/
│       │   ├── Timeline.tsx      # ReactFlow canvas
│       │   ├── TimelineNode.tsx
│       │   ├── SidePanel.tsx
│       │   ├── DetailPanel.tsx
│       │   ├── SecretsWarning.tsx
│       │   ├── Topbar.tsx
│       │   ├── DropZone.tsx
│       │   └── StatusBar.tsx
│       ├── hooks/
│       │   ├── useLiveFrames.ts
│       │   └── useAutoLoadRuns.ts
│       └── store/
│           └── useStore.ts
├── format/
│   └── SPEC.md                   # canonical format specification
├── docs/
│   └── assets/                   # screenshots, OG image
├── examples/
│   └── proxy-test.py
└── .github/
    └── workflows/
        └── release.yml           # multi-platform release CI (SHA-pinned actions)
```

---

## Testing strategy

| Layer | Test suite | Count | Location |
|-------|-----------|-------|----------|
| Rust core | `cargo test` | 9 (core) + 3 (cli) = 12 | `crates/*/src/*.rs` |
| Python SDK | `pytest` | 29 | `sdk/tests/` |
| TypeScript | (manual + visual) | — | viewer dev server |

Rust tests are unit tests colocated with the source using `#[cfg(test)]` modules.
Python tests use `pytest` with no test framework mocking — they write real
`.ghostline` files to a temp directory and read them back.

Cross-compatibility is verified explicitly: `test_format.py` writes a file in
Python and reads it back, while `test_record_replay.py` records a live call
(or a patched one) and replays it, asserting byte-for-byte response equality.

---

## Security considerations

- The proxy binds to `127.0.0.1` only. It must not be exposed on a network interface.
- The viewer server applies CORS restrictions to `localhost` origins.
- The scrubbing layer (`scrub=True` by default) redacts known API key formats before
  writing to disk. Scrubbing is best-effort; it does not guarantee that all secrets
  are removed. Users handling sensitive data should review the `ScrubConfig` patterns.
- The `export --format html` command embeds the full frame payload (possibly including
  unscrubbed data) in a standalone file. A warning is displayed at export time.
- GitHub Actions in `release.yml` are pinned to exact commit SHAs to prevent
  supply-chain attacks via mutable tags.
- The `install.sh` script downloads a `SHA256SUMS` file and verifies the binary
  checksum before installation.

---

*Last updated: 2026-03-08. Reflects Ghostline v0.2.0.*
