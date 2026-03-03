mod banner;
mod config;
mod proxy;
mod replay;
mod viewer_assets;
mod viewer_server;
mod wizard;

use base64::Engine;
use clap::{Parser, Subcommand};
use config::Config;
use ghostline_core::{GhostlineReader, MAGIC};
use std::path::PathBuf;
use std::sync::Arc;

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
    /// Launch proxy + recording (default behavior)
    Record {
        /// Session name
        name: Option<String>,
    },
    /// Launch proxy in replay mode
    Replay {
        /// Path to the .ghostline file
        file: String,
        /// Port for the replay proxy
        #[arg(short, long, default_value = "8384")]
        port: u16,
    },
    /// List recorded sessions
    Runs {
        #[command(subcommand)]
        action: Option<RunsCommand>,
    },
    /// Launch viewer only (no proxy)
    Viewer,
    /// Inspect a .ghostline file
    Inspect {
        /// Path to the .ghostline file
        file: String,
    },
    /// Export a .ghostline file
    Export {
        /// Path to the .ghostline file
        file: String,
        /// Output path
        #[arg(short, long)]
        output: Option<String>,
        /// Frame index (JSON only)
        #[arg(long)]
        frame: Option<usize>,
        /// Output format: json or html
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Show a single frame in detail
    Show {
        file: String,
        index: usize,
    },
    /// Fork a run at a specific step
    Fork {
        file: String,
        #[arg(long)]
        at: usize,
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Search frames (requires Python SDK)
    Search {
        file: String,
        query: String,
        #[arg(short, long, default_value = "5")]
        top: usize,
    },
    /// Run a transparent recording proxy only
    Proxy {
        #[arg(short, long, default_value = "9000")]
        port: u16,
        #[arg(short, long, default_value = "./ghostline-runs/")]
        out: PathBuf,
        #[arg(short, long, default_value = "https://api.anthropic.com")]
        target: String,
    },
    /// Run a command with ANTHROPIC_BASE_URL set automatically
    Run {
        /// Command to run (e.g., "claude" or "python agent.py")
        #[arg(trailing_var_arg = true, num_args = 1..)]
        cmd: Vec<String>,
    },
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
    /// Delete a recorded session
    Delete { name: String },
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Show current config
    Show,
    /// Set a config value
    Set { key: String, value: String },
}

fn fmt_ts(ms: u64) -> String {
    chrono::DateTime::from_timestamp_millis(ms as i64)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| format!("{}", ms))
}

fn print_data_preview(data: &[u8], label: &str) {
    println!("\n--- {} ({} bytes) ---", label, data.len());
    match std::str::from_utf8(data) {
        Ok(s) if s.len() <= 2000 => println!("{}", s),
        Ok(s) => println!("{}...", &s[..2000]),
        Err(_) => {
            if let Ok(val) = rmp_serde::from_slice::<serde_json::Value>(data) {
                if let Ok(json) = serde_json::to_string_pretty(&val) {
                    let truncated = if json.len() > 2000 { &json[..2000] } else { &json };
                    println!("{}", truncated);
                    return;
                }
            }
            let hex_preview: String = data
                .iter()
                .take(64)
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" ");
            println!(
                "{}{}",
                hex_preview,
                if data.len() > 64 { "..." } else { "" }
            );
        }
    }
}

/// Launch proxy + viewer + browser (the main "ghostline" experience)
async fn launch_all(cfg: &Config) -> anyhow::Result<()> {
    let (frame_tx, _) = tokio::sync::broadcast::channel::<String>(256);
    let frame_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let cfg = Arc::new(cfg.clone());

    // Ensure runs dir exists
    std::fs::create_dir_all(Config::runs_dir())?;

    banner::print_startup(cfg.proxy.port, cfg.viewer.port);

    // Spawn viewer server
    let viewer_cfg = cfg.clone();
    let viewer_tx = frame_tx.clone();
    let viewer_fc = frame_count.clone();
    let viewer_handle = tokio::spawn(async move {
        if let Err(e) = viewer_server::start(viewer_cfg, viewer_tx, viewer_fc).await {
            eprintln!("Viewer error: {}", e);
        }
    });

    // Auto-open browser
    if cfg.viewer.auto_open_browser {
        let url = format!("http://localhost:{}", cfg.viewer.port);
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let _ = open::that(&url);
    }

    // Run proxy (blocking on main task)
    let out = Config::runs_dir();
    let target = cfg.proxy.target.clone();
    proxy::run_proxy(cfg.proxy.port, out, target, Some(frame_tx), frame_count).await?;

    viewer_handle.abort();
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => {
            // Default: wizard if not configured, else launch all
            let cfg = if !Config::config_path().exists() {
                wizard::run_wizard()?
            } else {
                Config::load_or_default()
            };
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(launch_all(&cfg))?;
        }
        Some(Commands::SetupToken) => {
            wizard::run_wizard()?;
        }
        Some(Commands::Record { name: _ }) => {
            let cfg = Config::load_or_default();
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(launch_all(&cfg))?;
        }
        Some(Commands::Viewer) => {
            let cfg = Config::load_or_default();
            let (frame_tx, _) = tokio::sync::broadcast::channel::<String>(256);
            let frame_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(viewer_server::start(Arc::new(cfg), frame_tx, frame_count))?;
        }
        Some(Commands::Runs { action }) => match action {
            None => {
                let runs_dir = Config::runs_dir();
                if !runs_dir.exists() {
                    println!("No runs directory found at {}", runs_dir.display());
                    return Ok(());
                }
                let mut entries: Vec<_> = std::fs::read_dir(&runs_dir)?
                    .flatten()
                    .filter(|e| {
                        e.path()
                            .extension()
                            .map(|ext| ext == "ghostline")
                            .unwrap_or(false)
                    })
                    .collect();
                entries.sort_by_key(|e| e.file_name());
                if entries.is_empty() {
                    println!("No recorded sessions found.");
                } else {
                    for entry in entries {
                        let path = entry.path();
                        let name = path.file_name().unwrap().to_string_lossy();
                        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                        println!("  {} ({:.1} KB)", name, size as f64 / 1024.0);
                    }
                }
            }
            Some(RunsCommand::Delete { name }) => {
                let path = Config::runs_dir().join(&name);
                if path.exists() {
                    std::fs::remove_file(&path)?;
                    println!("Deleted: {}", name);
                } else {
                    println!("Not found: {}", name);
                }
            }
        },
        Some(Commands::Run { cmd }) => {
            if cmd.is_empty() {
                anyhow::bail!("Usage: ghostline run <command> [args...]");
            }
            let cfg = Config::load_or_default();
            let proxy_url = format!("http://localhost:{}", cfg.proxy.port);

            // Check if proxy is already running by trying to connect
            let proxy_running = std::net::TcpStream::connect(format!("127.0.0.1:{}", cfg.proxy.port)).is_ok();

            if !proxy_running {
                // Start proxy + viewer in background, then run command
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    let (frame_tx, _) = tokio::sync::broadcast::channel::<String>(256);
                    let frame_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
                    let cfg = Arc::new(cfg.clone());
                    let proxy_port = cfg.proxy.port;
                    let viewer_port = cfg.viewer.port;
                    let auto_open = cfg.viewer.auto_open_browser;

                    std::fs::create_dir_all(Config::runs_dir())?;
                    banner::print_startup(proxy_port, viewer_port);

                    // Spawn viewer
                    let vcfg = cfg.clone();
                    let vtx = frame_tx.clone();
                    let vfc = frame_count.clone();
                    tokio::spawn(async move {
                        let _ = viewer_server::start(vcfg, vtx, vfc).await;
                    });

                    // Spawn proxy
                    let out = Config::runs_dir();
                    let target = cfg.proxy.target.clone();
                    let ptx = frame_tx.clone();
                    let pfc = frame_count.clone();
                    tokio::spawn(async move {
                        let _ = proxy::run_proxy(proxy_port, out, target, Some(ptx), pfc).await;
                    });

                    // Wait for proxy to be ready
                    for _ in 0..50 {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        if std::net::TcpStream::connect(format!("127.0.0.1:{}", proxy_port)).is_ok() {
                            break;
                        }
                    }

                    // Auto-open browser
                    if auto_open {
                        let url = format!("http://localhost:{}", viewer_port);
                        let _ = open::that(&url);
                    }

                    // Run the user's command
                    eprintln!("\n Running: {} (with ANTHROPIC_BASE_URL={})\n", cmd.join(" "), proxy_url);
                    let status = tokio::process::Command::new(&cmd[0])
                        .args(&cmd[1..])
                        .env("ANTHROPIC_BASE_URL", &proxy_url)
                        .status()
                        .await?;

                    if !status.success() {
                        std::process::exit(status.code().unwrap_or(1));
                    }
                    Ok::<_, anyhow::Error>(())
                })?;
            } else {
                // Proxy already running, just exec the command
                eprintln!("Running: {} (with ANTHROPIC_BASE_URL={})", cmd.join(" "), proxy_url);
                let status = std::process::Command::new(&cmd[0])
                    .args(&cmd[1..])
                    .env("ANTHROPIC_BASE_URL", &proxy_url)
                    .status()?;
                if !status.success() {
                    std::process::exit(status.code().unwrap_or(1));
                }
            }
        }
        Some(Commands::Doctor) => {
            println!("Ghostline Doctor\n");

            // Config
            let cfg_path = Config::config_path();
            if cfg_path.exists() {
                println!("  ✓ Config found at {}", cfg_path.display());
                let cfg = Config::load_or_default();
                if cfg.is_configured() {
                    println!("  ✓ Token configured");
                } else {
                    println!("  ✗ No token configured (run: ghostline setup-token)");
                }
            } else {
                println!("  ✗ No config found (run: ghostline)");
            }

            // Runs dir
            let runs_dir = Config::runs_dir();
            if runs_dir.exists() {
                let count = std::fs::read_dir(&runs_dir)
                    .map(|d| d.flatten().filter(|e| e.path().extension().map(|x| x == "ghostline").unwrap_or(false)).count())
                    .unwrap_or(0);
                println!("  ✓ Runs directory: {} ({} sessions)", runs_dir.display(), count);
            } else {
                println!("  - Runs directory not yet created");
            }

            // Ports
            let proxy_ok = std::net::TcpStream::connect("127.0.0.1:9000").is_ok();
            let viewer_ok = std::net::TcpStream::connect("127.0.0.1:5173").is_ok();
            println!("  {} Proxy port 9000", if proxy_ok { "● (running)" } else { "○ (free)" });
            println!("  {} Viewer port 5173", if viewer_ok { "● (running)" } else { "○ (free)" });

            println!("\n  All checks passed.");
        }
        Some(Commands::Config { action }) => match action {
            ConfigCommand::Show => {
                let cfg = Config::load_or_default();
                let toml_str = toml::to_string_pretty(&cfg)?;
                println!("{}", toml_str);
            }
            ConfigCommand::Set { key, value } => {
                let mut cfg = Config::load_or_default();
                match key.as_str() {
                    "proxy.port" => cfg.proxy.port = value.parse()?,
                    "viewer.port" => cfg.viewer.port = value.parse()?,
                    "viewer.auto_open_browser" => cfg.viewer.auto_open_browser = value.parse()?,
                    "recording.scrub" => cfg.recording.scrub = value.parse()?,
                    "display.colors" => cfg.display.colors = value.parse()?,
                    _ => anyhow::bail!("Unknown config key: {}", key),
                }
                cfg.save(&Config::config_path())?;
                println!("Set {} = {}", key, value);
            }
        },
        // Legacy commands preserved from v1
        Some(Commands::Inspect { file }) => {
            let reader = GhostlineReader::open(&file)?;
            let magic = std::str::from_utf8(MAGIC).unwrap_or("?");
            println!("Magic:       {}", magic);
            println!("Version:     {}", reader.version);
            println!("Frames:      {}", reader.frame_count());
            println!("Started at:  {}", fmt_ts(reader.started_at));
            if let Some(sha) = &reader.git_sha {
                println!("Git SHA:     {}", hex::encode(sha));
            }
            if let Some(run_id) = &reader.parent_run_id {
                println!("Parent run:  {}", hex::encode(run_id));
                if let Some(step) = reader.fork_at_step {
                    println!("Forked at:   step {}", step);
                }
            }
            for i in 0..reader.frame_count() {
                println!("  [{}]", i);
            }
        }
        Some(Commands::Export {
            file,
            output,
            frame: frame_idx,
            format,
        }) => {
            if format == "html" {
                let raw = std::fs::read(&file)?;
                let data_b64 = base64::engine::general_purpose::STANDARD.encode(&raw);
                let filename = std::path::Path::new(&file)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                let viewer_dir = std::env::var("GHOSTLINE_VIEWER_DIST")
                    .unwrap_or_else(|_| "viewer/dist".to_string());
                let assets_dir = format!("{}/assets", viewer_dir);
                let mut js_content = String::new();
                let mut css_content = String::new();
                for entry in std::fs::read_dir(&assets_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    match path.extension().and_then(|e| e.to_str()) {
                        Some("js") => js_content = std::fs::read_to_string(&path)?,
                        Some("css") => css_content = std::fs::read_to_string(&path)?,
                        _ => {}
                    }
                }
                if js_content.is_empty() || css_content.is_empty() {
                    anyhow::bail!(
                        "viewer assets not found in {}. Run 'cd viewer && npm run build' or set GHOSTLINE_VIEWER_DIST",
                        assets_dir
                    );
                }
                let html = format!(
                    r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Ghostline — {filename}</title>
  <style>{css_content}</style>
</head>
<body>
  <div id="root"></div>
  <script id="ghostline-data" type="application/octet-stream" data-filename="{filename}">{data_b64}</script>
  <script type="module">{js_content}</script>
</body>
</html>"#
                );
                let out_path = output.unwrap_or_else(|| {
                    file.trim_end_matches(".ghostline").to_string() + ".html"
                });
                std::fs::write(&out_path, &html)?;
                println!("Exported → {} ({:.1} KB)", out_path, html.len() as f64 / 1024.0);
            } else {
                let mut reader = GhostlineReader::open(&file)?;
                let b64 = base64::engine::general_purpose::STANDARD;
                let range: Vec<usize> = match frame_idx {
                    Some(idx) => vec![idx],
                    None => (0..reader.frame_count()).collect(),
                };
                let mut frames = Vec::new();
                for i in range {
                    let frame = reader.get_frame(i)?;
                    let obj = serde_json::json!({
                        "frame_index": i,
                        "request_hash": hex::encode(frame.request_hash),
                        "latency_ms": frame.latency_ms,
                        "timestamp": frame.timestamp,
                        "request_b64": b64.encode(&frame.request_bytes),
                        "response_b64": b64.encode(&frame.response_bytes),
                    });
                    frames.push(obj);
                }
                let json = serde_json::to_string_pretty(&frames)?;
                match output {
                    Some(path) => std::fs::write(&path, &json)?,
                    None => println!("{}", json),
                }
            }
        }
        Some(Commands::Show { file, index }) => {
            let mut reader = GhostlineReader::open(&file)?;
            let frame = reader.get_frame(index)?;
            println!("Frame [{}]", index);
            println!("  Hash:      {}", hex::encode(frame.request_hash));
            println!("  Timestamp: {}", fmt_ts(frame.timestamp));
            println!("  Latency:   {}ms", frame.latency_ms);
            println!("  Request:   {} bytes", frame.request_bytes.len());
            println!("  Response:  {} bytes", frame.response_bytes.len());
            print_data_preview(&frame.request_bytes, "Request");
            print_data_preview(&frame.response_bytes, "Response");
        }
        Some(Commands::Fork { file, at, output }) => {
            use ghostline_core::{GhostlineWriter, Header};
            use sha2::{Digest, Sha256};

            let mut reader = GhostlineReader::open(&file)?;
            let frame_count = reader.frame_count();
            if at >= frame_count {
                anyhow::bail!(
                    "step {} out of range — file has {} frames (0..{})",
                    at,
                    frame_count,
                    frame_count - 1
                );
            }
            let first_frame = reader.get_frame(0)?;
            let mut hasher = Sha256::new();
            hasher.update(reader.started_at.to_le_bytes());
            hasher.update(first_frame.request_hash);
            let parent_run_id: [u8; 32] = hasher.finalize().into();
            let out_path = output.unwrap_or_else(|| {
                let stem = file.trim_end_matches(".ghostline");
                format!("{}-fork-{}.ghostline", stem, at)
            });
            let out_file = std::fs::File::create(&out_path)?;
            let mut buf_writer = std::io::BufWriter::new(out_file);
            let header = Header {
                started_at: reader.started_at,
                git_sha: reader.git_sha,
                parent_run_id: Some(parent_run_id),
                fork_at_step: Some(at as u32),
            };
            let mut writer = GhostlineWriter::new(&mut buf_writer, &header)?;
            for i in 0..=at {
                let frame = reader.get_frame(i)?;
                writer.append(&frame)?;
            }
            writer.finish()?;
            println!("Forked {} frames (0..={}) → {}", at + 1, at, out_path);
            println!("Parent run: {}", hex::encode(parent_run_id));
        }
        Some(Commands::Search { file, query, top }) => {
            let script = format!(
                r#"import sys
sys.path.insert(0, 'sdk')
from ghostline.search import GhostlineIndex
idx = GhostlineIndex()
n = idx.add_file("{file}")
print(f"Indexed {{n}} frames")
results = idx.search("{query}", k={top})
for r in results:
    fi = r["frame_idx"]
    sc = r["score"]
    print(f"  [{{fi}}] score={{sc:.3f}}")
if not results:
    print("  No results found.")
backend = "zvec" if idx.using_zvec else "numpy"
print(f"Backend: {{backend}}")
"#
            );
            let tmp = std::env::temp_dir().join("ghostline_search.py");
            std::fs::write(&tmp, &script)?;
            let status = std::process::Command::new("python3").arg(&tmp).status()?;
            let _ = std::fs::remove_file(&tmp);
            if !status.success() {
                anyhow::bail!("search requires Python SDK: pip install ghostline");
            }
        }
        Some(Commands::Replay { file, port }) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(replay::run_replay_server(&file, port))?;
        }
        Some(Commands::Proxy { port, out, target }) => {
            let frame_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(proxy::run_proxy(port, out, target, None, frame_count))?;
        }
    }

    Ok(())
}
