mod proxy;
mod replay;

use base64::Engine;
use clap::{Parser, Subcommand};
use ghostline_core::{GhostlineReader, MAGIC};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ghostline", version)]
#[command(about = "Deterministic replay for AI agents", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inspect a .ghostline file — print header and frame count
    Inspect {
        /// Path to the .ghostline file
        file: String,
    },
    /// Export a .ghostline file to JSON or standalone HTML
    Export {
        /// Path to the .ghostline file
        file: String,
        /// Output path (default: stdout for JSON, <file>.html for HTML)
        #[arg(short, long)]
        output: Option<String>,
        /// Export only frame at this index (JSON only)
        #[arg(long)]
        frame: Option<usize>,
        /// Output format: json (default) or html (standalone viewer)
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Show a single frame in detail (request/response preview)
    Show {
        /// Path to the .ghostline file
        file: String,
        /// Frame index (0-based)
        index: usize,
    },
    /// Replay a recorded run deterministically (serves cached responses)
    Replay {
        /// Path to the .ghostline file
        file: String,
        /// Port for the replay proxy
        #[arg(short, long, default_value = "8384")]
        port: u16,
    },
    /// Fork a run at a specific step — creates a new .ghostline with frames 0..=step
    Fork {
        /// Path to the source .ghostline file
        file: String,
        /// Step index to fork at (inclusive — frames 0..=at are copied)
        #[arg(long)]
        at: usize,
        /// Output path (default: <file>-fork-<step>.ghostline)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Run a transparent recording proxy — forwards requests and captures exchanges
    Proxy {
        /// Port for the proxy server
        #[arg(short, long, default_value = "9000")]
        port: u16,
        /// Output directory for .ghostline files
        #[arg(short, long, default_value = "./ghostline-runs/")]
        out: PathBuf,
        /// Target API base URL
        #[arg(short, long, default_value = "https://api.anthropic.com")]
        target: String,
    },
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
            // Try msgpack → JSON
            if let Ok(val) = rmp_serde::from_slice::<serde_json::Value>(data) {
                if let Ok(json) = serde_json::to_string_pretty(&val) {
                    let truncated = if json.len() > 2000 { &json[..2000] } else { &json };
                    println!("{}", truncated);
                    return;
                }
            }
            let hex_preview: String = data.iter().take(64)
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" ");
            println!("{}{}", hex_preview, if data.len() > 64 { "..." } else { "" });
        }
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect { file } => {
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

            // Per-frame summary
            for i in 0..reader.frame_count() {
                // Read index entry hash from the reader internals isn't exposed,
                // but we can show index positions
                println!("  [{}]", i);
            }
        }
        Commands::Export { file, output, frame: frame_idx, format } => {
            if format == "html" {
                // Read the raw .ghostline file
                let raw = std::fs::read(&file)?;
                let data_b64 = base64::engine::general_purpose::STANDARD.encode(&raw);
                let filename = std::path::Path::new(&file)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();

                // Find viewer dist assets relative to executable or via env
                let viewer_dir = std::env::var("GHOSTLINE_VIEWER_DIST")
                    .unwrap_or_else(|_| {
                        // Try relative to current dir
                        "viewer/dist".to_string()
                    });
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
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet" />
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
                // JSON export
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
        Commands::Show { file, index } => {
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
        Commands::Fork { file, at, output } => {
            use ghostline_core::{GhostlineWriter, Header};
            use sha2::{Digest, Sha256};

            let mut reader = GhostlineReader::open(&file)?;
            let frame_count = reader.frame_count();

            if at >= frame_count {
                anyhow::bail!(
                    "step {} out of range — file has {} frames (0..{})",
                    at, frame_count, frame_count - 1
                );
            }

            // Compute parent_run_id: SHA-256(started_at || first_frame_hash)
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
        Commands::Replay { file, port } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(replay::run_replay_server(&file, port))?;
        }
        Commands::Proxy { port, out, target } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(proxy::run_proxy(port, out, target))?;
        }
    }

    Ok(())
}
