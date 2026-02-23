# ghostline

**Deterministic replay for AI agents. Record once, replay without tokens.**

Debug AI agent runs by time-traveling through any state — without spending another token.

```bash
pip install ghostline
```

```python
import ghostline
from anthropic import Anthropic

client = ghostline.wrap(Anthropic())

# Record a run
with ghostline.record("run.ghostline"):
    result = agent.run("analyze this codebase")

# Replay it deterministically — zero API calls
with ghostline.replay("run.ghostline"):
    result = agent.run("analyze this codebase")  # served from cache, bit-for-bit identical
```

---

## Why

Every time you re-run an agent to reproduce a bug, you spend tokens. The behavior might change. You can't isolate which step went wrong.

Ghostline captures every LLM call in a compact binary format (`.ghostline`). Replays are served from the file — no network, no tokens, no nondeterminism.

> **LangSmith shows you what happened. Ghostline lets you replay it.**

---

## Status

🚧 **Early development** — not yet published on PyPI.

- [x] `.ghostline` binary format (MessagePack + zstd + O(1) index)
- [x] Rust capture engine (`ghostline-core`)
- [ ] Replay CLI (`ghostline replay <file>`)
- [ ] Python SDK (`pip install ghostline`)
- [ ] Timeline viewer (React)

---

## Format

Each `.ghostline` file contains:

```
[Header: magic + version + metadata]
[Frame 0: zstd(MessagePack({request_hash, request_bytes, response_bytes, latency_ms, timestamp}))]
[Frame 1: ...]
...
[Index: (request_hash, offset)[] + entry_count]
[index_offset: u64]  ← last 8 bytes, enables O(1) frame lookup
```

Magic bytes: `GHSTLINE`. Format version: `1`.

---

## Architecture

```
ghostline/
├── crates/
│   ├── ghostline-core/   # Rust: capture engine, .ghostline format, writer/reader
│   └── ghostline-cli/    # Rust: CLI binary (record, replay, export, inspect)
├── sdk/                  # Python: httpx wrapper, pip package
├── viewer/               # React: timeline viewer for .ghostline files
├── format/               # Format spec (SPEC.md)
└── examples/
```

---

## Roadmap

| Milestone | Status |
|-----------|--------|
| Binary format + capture engine | ✅ Done |
| Replay CLI | 🔜 Next |
| Python SDK | 🔜 Soon |
| React viewer | 🔜 Soon |
| 50 GitHub stars | ⭐ Help us get there |

---

## Philosophy

- **Self-hosted** — your traces never leave your machine
- **Zero cost replay** — replays don't spend tokens
- **Open source, MIT** — no lock-in, no SaaS required
- **Rust core, Python SDK** — fast capture, easy integration

---

## License

MIT
