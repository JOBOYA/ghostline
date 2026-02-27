# Ghostline v2 — Single Binary Design

**Date:** 2026-02-27  
**Status:** Approved  
**Author:** Architecture brainstorm (CEO + DEV + DESIGN + Joseph)

---

## Vision

One command. Zero dependencies. Everything automatic.

```
$ ghostline
```

That's it. Proxy starts, viewer opens, frames appear live. No Node.js. No Python. No npm. No pip.

---

## Architecture

### Single Binary

```
ghostline (~15-20MB Rust binary)
├── First-run wizard     → ~/.ghostline/config.toml
├── :9000 proxy (hyper)  → forward + capture + broadcast
├── :5173 viewer (axum)  → SPA embedded + WS + REST API
└── tokio broadcast channel (proxy → viewer, unbounded)
```

### Port Strategy

Two ports, clean separation:
- **9000** — HTTP proxy only. Intercepts `ANTHROPIC_BASE_URL` traffic.
- **5173** — Viewer HTTP server (embedded SPA) + WebSocket + REST API.

Mixing proxy + viewer on one port creates routing ambiguity with Anthropic API paths.

### Real-time: WebSocket

`/ws/live` on port 5173. Internal `tokio::sync::broadcast::channel` between proxy and viewer server. Each captured frame is serialized as JSON and pushed to all connected WS clients instantly.

SSE was considered but rejected: WebSocket enables future bidirectionality (pause, seek, remote control).

---

## Embedded Viewer

Build pipeline (CI only, not user-facing):
1. `npm run build` in `viewer/` → generates `viewer/dist/`
2. `cargo build` embeds `viewer/dist/` via `rust-embed` crate

Runtime: axum serves embedded files from memory. No filesystem access needed.

```rust
#[derive(RustEmbed)]
#[folder = "viewer/dist/"]
struct ViewerAssets;
```

The user receives ONE binary. Node.js is a build-time concern only.

---

## Viewer REST API (served by Rust binary)

```
GET  /                       → index.html (React SPA)
GET  /assets/*               → JS/CSS (compiled, embedded)
GET  /api/runs               → list .ghostline files in ~/.ghostline/runs/
GET  /api/runs/:name         → download .ghostline binary
GET  /api/runs/:name/frames  → frames as JSON array
GET  /api/status             → proxy stats (hits, misses, frame count)
WS   /ws/live                → live frame push (JSON per frame)
```

---

## Viewer React Changes

Three additions to the existing React codebase:

### 1. Auto-load runs on startup
```js
const runs = await fetch('/api/runs').then(r => r.json());
for (const run of runs) {
  const data = await fetch(`/api/runs/${run.name}`).then(r => r.arrayBuffer());
  loadRun(await parseGhostline(data, run.name));
}
```

### 2. Live WebSocket stream
```js
const ws = new WebSocket(`ws://${location.host}/ws/live`);
ws.onmessage = (event) => appendLiveFrame(JSON.parse(event.data));
```

### 3. Drag & drop remains as secondary option

---

## Config: `~/.ghostline/config.toml`

```toml
[auth]
claude_token = "..."   # obfuscated on disk — NOT a raw API key

[proxy]
port = 9000
target = "https://api.anthropic.com"

[viewer]
port = 5173
auto_open_browser = true

[recording]
output_dir = "~/.ghostline/runs"
scrub = true
default_model = "claude-3-haiku-20240307"

[display]
colors = true
```

---

## CLI Commands

```
ghostline                    → wizard (first run) OR launch everything (after setup)
ghostline setup-token        → (re)configure Claude Code token
ghostline record [name]      → launch proxy + recording
ghostline replay <name>      → proxy in replay mode
ghostline runs               → list recorded sessions
ghostline runs delete <name> → delete a session
ghostline viewer             → viewer only (no proxy)
ghostline inspect <file>     → .ghostline file details
ghostline export <file>      → JSON export
ghostline doctor             → full health check
ghostline config show        → display config
ghostline config set <k> <v> → edit config
ghostline --version          → version + logo
ghostline --help             → help
```

### `ghostline` with no args behavior

- **No config.toml** → full wizard (token, model, scrubbing prefs) then auto-launch record mode
- **Config exists** → launch proxy :9000 + viewer :5173 + open browser + start recording

---

## First-Run UX

```
$ ghostline

 ██████╗ ██╗  ██╗ ...
 v2.0.0 — Deterministic replay for AI agents.

First time? Let's set up.
? Enter your Claude Code token: ••••••••••••••••
✓ Token verified — connected as joseph@email.com
✓ Config saved to ~/.ghostline/config.toml

Starting Ghostline...
✓ Proxy listening on http://localhost:9000
✓ Viewer serving on http://localhost:5173
✓ Opening browser...

┌─────────────────────────────────────────────────┐
│ Ready! Open a new terminal and run:             │
│                                                 │
│   export ANTHROPIC_BASE_URL=http://localhost:9000│
│   claude                                        │
│                                                 │
│ All API calls will be captured automatically.  │
│ View them live at http://localhost:5173         │
└─────────────────────────────────────────────────┘

[14:30:25] ● FRAME 1 | claude-3-haiku | 342ms | 1.2KB
[14:30:28] ● FRAME 2 | claude-3-haiku | 891ms | 3.4KB
```

---

## Python SDK

**Decision: Keep, reposition.**

Not deprecated. Value remains in 3 specific cases:
1. **Deterministic unit tests** — pytest replay without running a proxy
2. **GhostlineIndex** — semantic search over frames (programmatic)
3. **CI pipelines** — export_html, programmatic frame access

SDK = "Advanced Usage" section in docs. Proxy = primary UX.

---

## Distribution Strategy

### Phase 1 (Show HN)
- GitHub Releases: pre-built binaries (Linux x86_64, macOS arm64/x86_64, Windows x64) via `cargo-dist`
- `curl -fsSL https://ghostline.dev/install.sh | sh`
- `cargo install ghostline`

### Phase 2 (post-traction)
- `pip install ghostline` (like ruff — best ROI for Python/AI audience)
- `brew install ghostline`
- `winget install ghostline`

---

## Build Pipeline Changes

`build.rs` must:
1. Run `npm run build` in `viewer/` (or fail if dist/ is stale)
2. Allow skipping with `GHOSTLINE_SKIP_VIEWER_BUILD=1` for fast iteration

CI matrix: build viewer once, then compile Rust for all targets.

---

## What Stays the Same

- `.ghostline` binary format (backward compatible)
- `ghostline fork`, `ghostline search`, `ghostline export --format html` (CLI commands)
- Python SDK (unchanged, optional)
- All existing Rust tests (9/9)

---

## What Changes

| Component | v1 | v2 |
|---|---|---|
| Entry point | Multiple commands | `ghostline` does everything |
| Viewer | Separate `npm run dev` | Embedded in binary |
| Live frames | Manual file reload | WebSocket push |
| Runs storage | Current dir | `~/.ghostline/runs/` |
| Config | None | `~/.ghostline/config.toml` |
| Token setup | Manual env var | First-run wizard |
| Distribution | cargo/pip only | curl install + releases |
