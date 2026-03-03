# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- Branching support: fork a run at any step (`ghostline fork <file> --at <step>`)
- Header extension for parent lineage tracking (`parent_run_id`, `fork_at_step`)
- LiteLLM provider support (`ghostline.wrap(litellm)` patches `litellm.completion()`)
- Shareable HTML exports (`ghostline export <file> --format html`) — standalone, no server needed
- Semantic search across recordings (`ghostline search <file> <query> --top N`)
- `GhostlineIndex` class for indexing and querying `.ghostline` files by natural language
- Transparent proxy mode (`ghostline proxy`) — zero code changes, works with any LLM client
- `ghostline run` command — sets `ANTHROPIC_BASE_URL` automatically
- Embedded viewer with WebSocket live streaming (feat/v2-single-binary branch)
- Scrub warning on HTML export

### Changed
- Scrubbing enabled by default in `recorder.py` and `context.py`

### Fixed
- Scrub integration restored after rebase (commit 6e78934)
- Security audit findings: 3 MEDIUM patched, 1 LOW patched

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

[Unreleased]: https://github.com/JOBOYA/ghostline/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/JOBOYA/ghostline/releases/tag/v0.1.0
