# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.2.0] - 2026-03-03

### Added
- **Single-binary mode** — `ghostline` (no subcommand) launches embedded viewer + capture proxy in one command
- **Transparent proxy** (`ghostline proxy`) — zero code changes required; works with any LLM client, including Claude Code, Cursor, LangChain, LiteLLM
- **`ghostline run <command>`** — wraps any command, sets `ANTHROPIC_BASE_URL` automatically
- **WebSocket live streaming** — frames appear in the viewer in real time as they are captured
- **Auto-load runs** — viewer lists and opens `.ghostline` files from the run directory automatically
- **Setup wizard** — first-run wizard for token configuration
- **Branching** — fork any run at step N (`ghostline fork <file> --at <step>`); tracks `parent_run_id` and `fork_at_step` in file header
- **LiteLLM provider support** — `ghostline.wrap(litellm)` patches `litellm.completion()` for record/replay
- **Semantic search** — search frames by natural language (`ghostline search <file> <query> --top N`); uses Zvec when available, numpy cosine fallback on Python 3.13+
- **Shareable HTML exports** — `ghostline export <file> --format html` produces a standalone `.html` with embedded data; no server required
- **Ghost favicon** — viewer and exported HTML include the ghost SVG favicon
- **Live indicator** — pulsing LIVE badge in the viewer status bar during active capture
- **Install script** — `curl -fsSL https://ghostline.dev/install.sh | sh`
- **GitHub Actions release workflow** — cross-platform binaries (Linux, macOS arm64/x86, Windows) published on `v*` tag

### Changed
- Scrubbing enabled by default in `recorder.py` and `context.py` (previously opt-in)
- CORS restricted to localhost origins in viewer server (security hardening)

### Fixed
- Scrub integration lost during remote rebase — fully restored
- HTML export now shows a warning when scrub mode was not enabled at record time
- Security audit #18: 3 MEDIUM findings patched (path traversal guard, WS auth, rate limit)

## [0.1.0] - 2026-02-25

### Added
- `.ghostline` binary format: MessagePack frames + zstd compression + O(1) hash index
- Rust capture engine (`ghostline-core`): `Frame`, `Writer`, `Reader`
- CLI commands: `inspect`, `show`, `export`, `replay`
- Replay proxy server: HTTP intercept, SHA-256 hash matching, cached responses
- Python SDK: `ghostline.wrap(client)` for Anthropic and OpenAI
- `record()` and `replay()` context managers
- API key scrubbing: 15+ built-in regex patterns (Anthropic, OpenAI, Stripe, AWS, GitHub, Bearer, emails)
- Configurable scrubbing via `ScrubConfig`
- React timeline viewer: horizontal layout, per-frame detail panel, drag & drop, dark theme
- Cross-compatibility: Python writes ↔ Rust reads
- Format spec: `format/SPEC.md`

[Unreleased]: https://github.com/JOBOYA/ghostline/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/JOBOYA/ghostline/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/JOBOYA/ghostline/releases/tag/v0.1.0
