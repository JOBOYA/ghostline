# Contributing to Ghostline

Thanks for your interest! Ghostline is early-stage and we welcome contributions.

## What We Need Help With

- **Provider support** — OpenAI, LiteLLM, Cohere, etc. (currently Anthropic only)
- **Python SDK ergonomics** — better decorators, async support, framework integrations
- **Timeline viewer** — React components, accessibility, keyboard navigation
- **Documentation** — examples, guides, troubleshooting
- **Testing** — edge cases, large files, cross-platform

## Getting Started

### Rust (engine + CLI)

```bash
git clone https://github.com/JOBOYA/ghostline.git
cd ghostline
cargo build
cargo test
```

### Python SDK

```bash
cd sdk/python
pip install -e ".[dev]"
pytest
```

### React Viewer

```bash
cd viewer
npm install
npm run dev
```

## Code Style

- **Rust**: `cargo fmt` + `cargo clippy` (zero warnings)
- **Python**: Black + ruff
- **React/TypeScript**: Prettier + ESLint
- **Commits**: conventional commits (`feat:`, `fix:`, `docs:`, `test:`)

## Pull Requests

1. Fork the repo
2. Create a branch (`feat/my-feature` or `fix/my-bug`)
3. Write tests for new functionality
4. Run the test suite (`cargo test` and/or `pytest`)
5. Open a PR with a clear description of what and why

Keep PRs focused — one feature or fix per PR.

## Design System (for viewer contributions)

The viewer uses a strict dark-only palette:

| Token | Value | Usage |
|:------|:------|:------|
| `--bg` | `#09090B` | Background |
| `--surface` | `#18181B` | Panels, cards |
| `--border` | `#27272A` | Borders |
| `--text` | `#FAFAFA` | Primary text |
| `--muted` | `#A1A1AA` | Secondary text |
| `--accent-llm` | `#8B5CF6` | LLM call nodes |
| `--accent-tool` | `#22D3EE` | Tool call nodes |
| `--accent-branch` | `#F59E0B` | Branch/fork points |
| `--accent-error` | `#EF4444` | Errors |

Fonts: **Inter** (UI) + **JetBrains Mono** (code/payloads)

See `specs/ghostline-viewer-design.md` in the collab repo for full component specs.

## Reporting Issues

Open a GitHub issue with:
- What you expected
- What happened
- Steps to reproduce
- `.ghostline` file if relevant (scrub sensitive data first!)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
