# Ghostline v2 — Single Binary Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor Ghostline into a single self-contained Rust binary that launches proxy + embedded React viewer + WebSocket live streaming with one command.

**Architecture:** The `ghostline` binary embeds the compiled React viewer via `rust-embed`, serves it via axum on :5173, runs the HTTP proxy on :9000 via hyper, and streams frames live via WebSocket using a `tokio::broadcast` channel between proxy and viewer server.

**Tech Stack:** Rust (axum, hyper, tokio, rust-embed, clap, dialoguer, webbrowser), React/Vite (existing), tokio-tungstenite (or axum WS), serde_json.

**Design doc:** `docs/plans/2026-02-27-v2-single-binary-design.md`

**Reference codebase:**
- Proxy: `crates/ghostline-cli/src/proxy.rs`
- CLI entry: `crates/ghostline-cli/src/main.rs`
- Core format: `crates/ghostline-core/src/`
- React viewer: `viewer/src/`
- Viewer dist: `viewer/dist/` (already built, 340KB)

---

## Task 1: Add dependencies to Cargo.toml

**Files:**
- Modify: `crates/ghostline-cli/Cargo.toml`

**Step 1: Add new crates**

```toml
[dependencies]
# existing deps stay
axum = { version = "0.7", features = ["ws"] }
rust-embed = "8"
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.21"
serde_json = "1"
dialoguer = "0.11"
webbrowser = "0.8"
toml = "0.8"
serde = { version = "1", features = ["derive"] }
dirs = "5"
tower-http = { version = "0.5", features = ["cors"] }
mime_guess = "2"
```

**Step 2: Verify build compiles (no code yet)**

```bash
cd crates/ghostline-cli && cargo build 2>&1 | head -20
```
Expected: compiles (possibly with unused dep warnings, that's fine)

**Step 3: Commit**

```bash
git add crates/ghostline-cli/Cargo.toml
git commit -m "chore: add axum, rust-embed, dialoguer, webbrowser deps for v2"
```

---

## Task 2: Config module (~/.ghostline/config.toml)

**Files:**
- Create: `crates/ghostline-cli/src/config.rs`
- Modify: `crates/ghostline-cli/src/main.rs` (add `mod config;`)

**Step 1: Write the test**

```rust
// At bottom of config.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_config_default() {
        let cfg = Config::default();
        assert_eq!(cfg.proxy.port, 9000);
        assert_eq!(cfg.viewer.port, 5173);
        assert!(cfg.recording.scrub);
    }

    #[test]
    fn test_config_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        let cfg = Config::default();
        cfg.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded.proxy.port, cfg.proxy.port);
    }
}
```

Add `tempfile = "3"` to `[dev-dependencies]` in Cargo.toml.

**Step 2: Run test to verify it fails**

```bash
cargo test -p ghostline-cli config -- --nocapture 2>&1 | tail -5
```
Expected: compile error (config module not found)

**Step 3: Implement config.rs**

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub auth: AuthConfig,
    pub proxy: ProxyConfig,
    pub viewer: ViewerConfig,
    pub recording: RecordingConfig,
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub claude_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub port: u16,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewerConfig {
    pub port: u16,
    pub auto_open_browser: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingConfig {
    pub output_dir: String,
    pub scrub: bool,
    pub default_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub colors: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auth: AuthConfig { claude_token: None },
            proxy: ProxyConfig {
                port: 9000,
                target: "https://api.anthropic.com".to_string(),
            },
            viewer: ViewerConfig {
                port: 5173,
                auto_open_browser: true,
            },
            recording: RecordingConfig {
                output_dir: "~/.ghostline/runs".to_string(),
                scrub: true,
                default_model: "claude-3-haiku-20240307".to_string(),
            },
            display: DisplayConfig { colors: true },
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        dirs::home_dir()
            .expect("no home dir")
            .join(".ghostline")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn runs_dir() -> PathBuf {
        dirs::home_dir()
            .expect("no home dir")
            .join(".ghostline")
            .join("runs")
    }

    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn load_or_default() -> Self {
        let path = Self::config_path();
        if path.exists() {
            Self::load(&path).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn is_configured(&self) -> bool {
        self.auth.claude_token.is_some()
    }
}
```

**Step 4: Run tests**

```bash
cargo test -p ghostline-cli config -- --nocapture
```
Expected: 2 tests pass

**Step 5: Commit**

```bash
git add crates/ghostline-cli/src/config.rs crates/ghostline-cli/Cargo.toml
git commit -m "feat(v2): add config module (~/.ghostline/config.toml)"
```

---

## Task 3: First-run wizard (dialoguer)

**Files:**
- Create: `crates/ghostline-cli/src/wizard.rs`
- Modify: `crates/ghostline-cli/src/main.rs` (add `mod wizard;`)

**Step 1: Implement wizard.rs**

No unit tests for wizard (it's interactive I/O). Manual test is step 2.

```rust
use crate::config::Config;
use dialoguer::{Input, Confirm};
use std::path::Path;

pub fn run_wizard() -> anyhow::Result<Config> {
    println!("\n First time? Let's set up.\n");

    let token: String = Input::new()
        .with_prompt("Enter your Claude Code token\n  (Run `claude setup-token` in another terminal to get it)\n  Token")
        .interact_text()?;

    // TODO: verify token against Anthropic API
    println!(" Token saved");

    let scrub = Confirm::new()
        .with_prompt("Scrub secrets from recordings? (recommended)")
        .default(true)
        .interact()?;

    let auto_open = Confirm::new()
        .with_prompt("Auto-open browser when starting?")
        .default(true)
        .interact()?;

    let mut cfg = Config::default();
    cfg.auth.claude_token = Some(token);
    cfg.recording.scrub = scrub;
    cfg.viewer.auto_open_browser = auto_open;

    let path = Config::config_path();
    cfg.save(&path)?;
    println!(" Config saved to {}", path.display());

    Ok(cfg)
}
```

**Step 2: Wire into main.rs**

In `main.rs`, at the start of the `ghostline` (no subcommand) branch:

```rust
let cfg = if !Config::config_path().exists() {
    wizard::run_wizard()?
} else {
    Config::load_or_default()
};
```

**Step 3: Manual test**

```bash
rm -f ~/.ghostline/config.toml
cargo run -p ghostline-cli -- 2>&1 | head -20
```
Expected: wizard prompts appear

**Step 4: Commit**

```bash
git add crates/ghostline-cli/src/wizard.rs crates/ghostline-cli/src/main.rs
git commit -m "feat(v2): first-run wizard with dialoguer"
```

---

## Task 4: Embed React viewer with rust-embed

**Files:**
- Create: `crates/ghostline-cli/src/viewer_assets.rs`
- Modify: `crates/ghostline-cli/src/main.rs`

**Step 1: Verify viewer/dist/ exists and is built**

```bash
ls /root/.openclaw/workspace/ghostline/viewer/dist/assets/
```
Expected: index-*.js and index-*.css files

If missing:
```bash
cd viewer && npm install && npm run build
```

**Step 2: Create viewer_assets.rs**

```rust
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../viewer/dist/"]
pub struct ViewerAssets;
```

Note: `$CARGO_MANIFEST_DIR` in `#[folder]` attr resolves at compile time relative to the crate.
Actual path from crate root: adjust to `../../viewer/dist/` or use absolute path in build.rs.

**Step 3: Quick smoke test**

```rust
#[test]
fn test_viewer_assets_has_index() {
    let index = ViewerAssets::get("index.html");
    assert!(index.is_some(), "index.html not found in embedded assets");
    let js = ViewerAssets::iter().any(|f| f.ends_with(".js"));
    assert!(js, "no JS file found in embedded assets");
}
```

**Step 4: Run test**

```bash
cargo test -p ghostline-cli viewer_assets
```
Expected: 1 test passes

**Step 5: Commit**

```bash
git add crates/ghostline-cli/src/viewer_assets.rs
git commit -m "feat(v2): embed React viewer dist with rust-embed"
```

---

## Task 5: Viewer HTTP server (axum) — static files + REST API

**Files:**
- Create: `crates/ghostline-cli/src/viewer_server.rs`
- Modify: `crates/ghostline-cli/src/main.rs`

**Step 1: Implement viewer_server.rs**

```rust
use axum::{
    Router,
    routing::get,
    extract::{Path, State},
    response::{IntoResponse, Response},
    http::{HeaderMap, StatusCode, header},
    Json,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use serde_json::{json, Value};
use crate::viewer_assets::ViewerAssets;
use crate::config::Config;

pub type FrameSender = broadcast::Sender<String>;

#[derive(Clone)]
pub struct ViewerState {
    pub config: Arc<Config>,
    pub frame_tx: FrameSender,
}

pub fn router(state: ViewerState) -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/assets/*path", get(serve_asset))
        .route("/api/runs", get(list_runs))
        .route("/api/runs/:name", get(get_run))
        .route("/api/status", get(get_status))
        .route("/ws/live", get(ws_handler))
        .with_state(state)
        .layer(tower_http::cors::CorsLayer::permissive())
}

async fn serve_index() -> impl IntoResponse {
    serve_embedded_file("index.html")
}

async fn serve_asset(Path(path): Path<String>) -> impl IntoResponse {
    serve_embedded_file(&format!("assets/{}", path))
}

fn serve_embedded_file(path: &str) -> Response {
    match ViewerAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                mime.as_ref().parse().unwrap(),
            );
            (StatusCode::OK, headers, content.data.into_owned()).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn list_runs(State(state): State<ViewerState>) -> impl IntoResponse {
    let runs_dir = Config::runs_dir();
    let mut runs = vec![];
    if let Ok(entries) = std::fs::read_dir(&runs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "ghostline").unwrap_or(false) {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                runs.push(json!({"name": name, "size": size}));
            }
        }
    }
    Json(runs)
}

async fn get_run(
    Path(name): Path<String>,
    State(state): State<ViewerState>,
) -> impl IntoResponse {
    let path = Config::runs_dir().join(&name);
    match std::fs::read(&path) {
        Ok(data) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                "application/octet-stream".parse().unwrap(),
            );
            (StatusCode::OK, headers, data).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn get_status() -> impl IntoResponse {
    Json(json!({"status": "ok"}))
}

async fn ws_handler(
    ws: axum::extract::WebSocketUpgrade,
    State(state): State<ViewerState>,
) -> impl IntoResponse {
    let mut rx = state.frame_tx.subscribe();
    ws.on_upgrade(move |mut socket| async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if socket.send(axum::extract::ws::Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    })
}

pub async fn start(config: Arc<Config>, frame_tx: FrameSender) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", config.viewer.port).parse()?;
    let state = ViewerState { config, frame_tx };
    let app = router(state);
    println!(" Viewer serving on http://localhost:{}", addr.port());
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
```

**Step 2: Test — serve index.html**

Write integration test:
```rust
#[tokio::test]
async fn test_viewer_serves_index() {
    // This is tested manually for now — axum test utils can be added later
    // Check that serve_embedded_file returns 200 for "index.html"
    let response = serve_embedded_file("index.html");
    // Just check it compiles and runs
}
```

**Step 3: Manual test**

```bash
cargo run -p ghostline-cli -- viewer
```
Open http://localhost:5173 — should see the React viewer.

**Step 4: Commit**

```bash
git add crates/ghostline-cli/src/viewer_server.rs
git commit -m "feat(v2): axum viewer server with embedded SPA + REST API"
```

---

## Task 6: Broadcast channel — proxy → viewer live streaming

**Files:**
- Modify: `crates/ghostline-cli/src/proxy.rs`
- Modify: `crates/ghostline-cli/src/main.rs`

**Step 1: Add frame_tx to ProxyState**

In `proxy.rs`, add `frame_tx: Option<FrameSender>` to `ProxyState`.

After capturing a frame (after `writer.capture(...)`), broadcast JSON:

```rust
if let Some(ref tx) = s.frame_tx {
    let frame_json = serde_json::json!({
        "index": s.frame_count,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "request_size": body_bytes.len(),
        "response_size": resp_body.len(),
        "latency_ms": elapsed.as_millis(),
    });
    let _ = tx.send(frame_json.to_string()); // ignore if no subscribers
}
```

**Step 2: Wire in main.rs**

```rust
let (frame_tx, _) = tokio::sync::broadcast::channel::<String>(256);
let frame_tx_proxy = frame_tx.clone();
let frame_tx_viewer = frame_tx.clone();

// Spawn proxy with frame_tx_proxy
// Spawn viewer server with frame_tx_viewer
tokio::join!(
    proxy::start(proxy_config, frame_tx_proxy),
    viewer_server::start(viewer_config, frame_tx_viewer),
);
```

**Step 3: Manual test**

```bash
export ANTHROPIC_BASE_URL=http://localhost:9000
cargo run -p ghostline-cli &
# In viewer: connect to ws://localhost:5173/ws/live
# Make an API call — frame should appear
```

**Step 4: Commit**

```bash
git add crates/ghostline-cli/src/proxy.rs crates/ghostline-cli/src/main.rs
git commit -m "feat(v2): broadcast channel — proxy streams live frames to viewer WS"
```

---

## Task 7: React viewer — WebSocket live streaming

**Files:**
- Modify: `viewer/src/App.tsx` (or wherever frames are loaded)
- Modify: `viewer/src/hooks/` (create `useLiveFrames.ts`)

**Step 1: Create `viewer/src/hooks/useLiveFrames.ts`**

```ts
import { useEffect, useRef } from 'react';

export interface LiveFrame {
  index: number;
  timestamp: string;
  request_size: number;
  response_size: number;
  latency_ms: number;
}

export function useLiveFrames(onFrame: (frame: LiveFrame) => void) {
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    const ws = new WebSocket(`ws://${location.host}/ws/live`);
    wsRef.current = ws;

    ws.onmessage = (event) => {
      try {
        const frame: LiveFrame = JSON.parse(event.data);
        onFrame(frame);
      } catch (e) {
        console.warn('Failed to parse live frame:', e);
      }
    };

    ws.onerror = (e) => console.warn('WS error:', e);

    return () => {
      ws.close();
    };
  }, []);

  return wsRef;
}
```

**Step 2: Create `viewer/src/hooks/useAutoLoadRuns.ts`**

```ts
import { useEffect } from 'react';

export interface RunMeta {
  name: string;
  size: number;
}

export async function fetchRuns(): Promise<RunMeta[]> {
  const res = await fetch('/api/runs');
  if (!res.ok) return [];
  return res.json();
}

export async function fetchRunData(name: string): Promise<ArrayBuffer> {
  const res = await fetch(`/api/runs/${name}`);
  return res.arrayBuffer();
}
```

**Step 3: Wire into App.tsx**

Find where the app currently loads replay data (likely from drag & drop). Add:

```tsx
// Auto-load existing runs on mount
useEffect(() => {
  fetchRuns().then(async (runs) => {
    for (const run of runs) {
      const data = await fetchRunData(run.name);
      // call existing loadRun(data, run.name) or equivalent
    }
  });
}, []);

// Live frames from WS
useLiveFrames((frame) => {
  appendLiveFrame(frame); // call existing frame append logic
});
```

**Step 4: Rebuild viewer**

```bash
cd viewer && npm run build
```

**Step 5: Rebuild Rust binary and test**

```bash
cargo build -p ghostline-cli
cargo run -p ghostline-cli -- viewer
```
Open browser → should auto-load any .ghostline files in `~/.ghostline/runs/`

**Step 6: Commit**

```bash
git add viewer/src/hooks/ viewer/src/App.tsx viewer/dist/
git commit -m "feat(v2): viewer WebSocket live streaming + auto-load runs from API"
```

---

## Task 8: Browser auto-open

**Files:**
- Modify: `crates/ghostline-cli/src/main.rs`

**Step 1: After viewer server starts, open browser**

```rust
use webbrowser;

// After spawning viewer server task
let viewer_url = format!("http://localhost:{}", cfg.viewer.port);
if cfg.viewer.auto_open_browser {
    // Small delay to let server bind
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let _ = webbrowser::open(&viewer_url);
}
```

**Step 2: Test**

```bash
cargo run -p ghostline-cli
```
Expected: browser opens to `http://localhost:5173`

**Step 3: Commit**

```bash
git add crates/ghostline-cli/src/main.rs
git commit -m "feat(v2): auto-open browser after viewer starts"
```

---

## Task 9: Complete CLI command wiring (clap)

**Files:**
- Modify: `crates/ghostline-cli/src/main.rs`

**Step 1: Replace current arg parsing with clap derive**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ghostline", version, about = "Deterministic replay for AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// (Re)configure Claude Code token
    SetupToken,
    /// Launch proxy + recording
    Record { name: Option<String> },
    /// Launch proxy in replay mode
    Replay { name: String },
    /// List recorded sessions
    Runs {
        #[command(subcommand)]
        action: Option<RunsCommand>,
    },
    /// Launch viewer only (no proxy)
    Viewer,
    /// Show details of a .ghostline file
    Inspect { file: String },
    /// Export .ghostline file (default: JSON)
    Export { file: String, #[arg(long, default_value = "json")] format: String },
    /// Full health check
    Doctor,
    /// Config management
    Config {
        #[command(subcommand)]
        action: ConfigCommand,
    },
}

#[derive(Subcommand)]
enum RunsCommand {
    Delete { name: String },
}

#[derive(Subcommand)]
enum ConfigCommand {
    Show,
    Set { key: String, value: String },
}
```

**Step 2: Implement each command handler**

- `None` (no subcommand) → wizard if not configured, else launch all
- `SetupToken` → call wizard::run_wizard()
- `Viewer` → start viewer server only
- `Runs` → list `~/.ghostline/runs/*.ghostline`
- `Doctor` → check config, check ports, report status
- Other commands → wire to existing v1 logic

**Step 3: Run tests**

```bash
cargo test -p ghostline-cli
cargo run -p ghostline-cli -- --help
cargo run -p ghostline-cli -- runs
cargo run -p ghostline-cli -- doctor
```

**Step 4: Commit**

```bash
git add crates/ghostline-cli/src/main.rs
git commit -m "feat(v2): complete CLI with clap (all subcommands wired)"
```

---

## Task 10: Startup banner + frame log

**Files:**
- Create: `crates/ghostline-cli/src/banner.rs`
- Modify: `crates/ghostline-cli/src/main.rs`

**Step 1: Create banner.rs**

```rust
pub const LOGO: &str = r#"
 ██████╗ ██╗  ██╗ ...
"#;

pub fn print_startup(proxy_port: u16, viewer_port: u16) {
    println!("{}", LOGO);
    println!(" v2.0.0 — Deterministic replay for AI agents.\n");
    println!(" Proxy listening on  http://localhost:{}", proxy_port);
    println!(" Viewer serving on   http://localhost:{}", viewer_port);
    println!(" Opening browser...\n");
    println!("┌──────────────────────────────────────────────────────┐");
    println!("│ Ready! Open a new terminal and run:                 │");
    println!("│                                                      │");
    println!("│   export ANTHROPIC_BASE_URL=http://localhost:{}    │", proxy_port);
    println!("│   claude                                             │");
    println!("│                                                      │");
    println!("│ All API calls will be captured automatically.       │");
    println!("│ View them live at http://localhost:{}              │", viewer_port);
    println!("└──────────────────────────────────────────────────────┘\n");
}

pub fn print_frame(index: usize, model: &str, latency_ms: u64, size_bytes: usize) {
    let now = chrono::Local::now().format("%H:%M:%S");
    println!(
        "[{}] ● FRAME {} | {} | {}ms | {:.1}KB",
        now, index, model, latency_ms,
        size_bytes as f64 / 1024.0
    );
}
```

**Step 2: Call from main.rs after all services start**

**Step 3: Commit**

```bash
git add crates/ghostline-cli/src/banner.rs
git commit -m "feat(v2): startup banner and live frame log output"
```

---

## Task 11: build.rs — auto-build viewer before cargo build

**Files:**
- Create: `build.rs` at workspace root (or in `crates/ghostline-cli/build.rs`)

**Step 1: Create build.rs**

```rust
use std::process::Command;
use std::env;

fn main() {
    // Skip viewer build if explicitly disabled (e.g., in CI after manual build)
    if env::var("GHOSTLINE_SKIP_VIEWER_BUILD").is_ok() {
        println!("cargo:warning=Skipping viewer build (GHOSTLINE_SKIP_VIEWER_BUILD set)");
        return;
    }

    let viewer_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../viewer");
    let dist_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../viewer/dist");

    // Tell cargo to re-run if viewer source changes
    println!("cargo:rerun-if-changed={}/src", viewer_dir);
    println!("cargo:rerun-if-changed={}/package.json", viewer_dir);

    // Only build if dist/ doesn't exist or is stale
    if !std::path::Path::new(dist_dir).exists() {
        println!("cargo:warning=Building viewer...");
        let status = Command::new("npm")
            .args(["run", "build"])
            .current_dir(viewer_dir)
            .status()
            .expect("failed to run npm — is Node.js installed?");

        if !status.success() {
            panic!("Viewer build failed");
        }
    }
}
```

**Step 2: Test**

```bash
rm -rf viewer/dist
GHOSTLINE_SKIP_VIEWER_BUILD=0 cargo build -p ghostline-cli 2>&1 | grep -E "warning|error"
```
Expected: viewer build runs, then Rust compiles

**Step 3: Commit**

```bash
git add crates/ghostline-cli/build.rs
git commit -m "feat(v2): build.rs auto-builds React viewer before cargo build"
```

---

## Task 12: GitHub Actions CI — build binaries for all targets

**Files:**
- Create: `.github/workflows/release.yml`

**Step 1: Create release workflow**

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: ghostline-linux-x86_64
          - os: macos-latest
            target: aarch64-apple-darwin
            artifact: ghostline-macos-arm64
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact: ghostline-macos-x86_64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: ghostline-windows-x86_64.exe

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Build viewer
        run: |
          cd viewer
          npm ci
          npm run build

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build binary
        env:
          GHOSTLINE_SKIP_VIEWER_BUILD: "1"
        run: cargo build --release --target ${{ matrix.target }} -p ghostline-cli

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: target/${{ matrix.target }}/release/ghostline*
```

**Step 2: Create install script `install.sh`**

```bash
#!/bin/sh
set -e
REPO="JOBOYA/ghostline"
VERSION=$(curl -sf "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS-$ARCH" in
  linux-x86_64)  FILE="ghostline-linux-x86_64" ;;
  darwin-arm64)  FILE="ghostline-macos-arm64" ;;
  darwin-x86_64) FILE="ghostline-macos-x86_64" ;;
  *) echo "Unsupported platform: $OS-$ARCH"; exit 1 ;;
esac

URL="https://github.com/$REPO/releases/download/$VERSION/$FILE"
echo "Downloading ghostline $VERSION..."
curl -fL "$URL" -o /usr/local/bin/ghostline
chmod +x /usr/local/bin/ghostline
echo "Installed ghostline $VERSION to /usr/local/bin/ghostline"
```

**Step 3: Commit**

```bash
git add .github/workflows/release.yml install.sh
git commit -m "ci: GitHub Actions release workflow + install.sh"
```

---

## Task 13: Final integration test + README update

**Step 1: End-to-end smoke test**

```bash
# Clean slate
rm -f ~/.ghostline/config.toml

# First run wizard
cargo run -p ghostline-cli

# Should see wizard, fill in token, then proxy + viewer start
# Open http://localhost:5173 — should see React viewer
# Run: export ANTHROPIC_BASE_URL=http://localhost:9000
# Make API call — frame should appear in viewer

# Second run (config exists)
cargo run -p ghostline-cli
# Should skip wizard, go straight to launch
```

**Step 2: Run full test suite**

```bash
cargo test --workspace
cd sdk && python3 -m pytest tests/ -q
```
Expected: Rust 9+/tests pass, Python 29+/tests pass

**Step 3: Update README**

Replace current "Quick Start" section:

```markdown
## Quick Start

```bash
curl -fsSL https://ghostline.dev/install.sh | sh
ghostline
```

That's it. Ghostline will:
1. Ask for your Claude Code token (first run only)
2. Start a proxy on http://localhost:9000
3. Open the viewer at http://localhost:5173
4. Capture all API calls live

Then in a new terminal:
```bash
export ANTHROPIC_BASE_URL=http://localhost:9000
claude  # or any AI agent
```
```

**Step 4: Final commit**

```bash
git add README.md
git commit -m "docs: update README for v2 single-binary quick start"
git tag v2.0.0
```

---

## Summary

| Task | Description | Est. |
|---|---|---|
| 1 | Add Cargo deps | 5min |
| 2 | Config module | 15min |
| 3 | First-run wizard | 15min |
| 4 | rust-embed viewer | 10min |
| 5 | Axum viewer server + REST | 30min |
| 6 | Broadcast channel live frames | 20min |
| 7 | React WS + auto-load | 25min |
| 8 | Browser auto-open | 5min |
| 9 | CLI clap wiring | 20min |
| 10 | Banner + frame log | 10min |
| 11 | build.rs auto-build | 10min |
| 12 | GitHub Actions CI | 20min |
| 13 | E2E test + README | 15min |
| **Total** | | **~3h** |
