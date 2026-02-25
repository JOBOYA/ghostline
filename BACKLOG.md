# Ghostline Backlog

Post-MVP work items, ordered by priority.

## Security (gate for public launch)

- [x] Bind replay proxy to 127.0.0.1 (already done)
- [x] Scrubbing layer: redact API keys, PII, tokens before writing frames
- [ ] Threat model document (`specs/ghostline-threat-model.md`)
- [ ] AWS secret key pattern (added, pending release)

## SDK

- [x] PyPI publish: `pip install ghostline` (v0.1.0 live)
- [ ] npm publish viewer or deploy to ghostline.dev
- [ ] Multi-provider: LiteLLM support
- [ ] Configurable scrubbing patterns via `.ghostlinerc`
- [ ] Branching: fork at step N, new `.ghostline` with `parent_run_id`

## Viewer

- [ ] Zoom: semantic grouping (phases when zoomed out, steps when zoomed in)
- [ ] Keyboard shortcut B: fork from selected step
- [ ] Export: PNG/SVG timeline snapshot

## Distribution

- [ ] Show HN (gated on threat model + scrubbing validation)
- [ ] PR to awesome-llm-apps, awesome-ai-agents, awesome-rust
- [ ] Demo video: real run → replay → show zero tokens used

## Future

- [ ] Streaming support (SSE/WebSocket frame capture)
- [ ] Cost tracking per frame (token counts + pricing)
- [ ] VS Code extension: inline replay viewer
