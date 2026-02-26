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
    /// Export a .ghostline file to JSON
    Export {
        /// Path to the .ghostline file
        file: String,
        /// Output path (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
        /// Export only frame at this index
        #[arg(long)]
        frame: Option<usize>,
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

            // Per-frame summary
            for i in 0..reader.frame_count() {
                // Read index entry hash from the reader internals isn't exposed,
                // but we can show index positions
                println!("  [{}]", i);
            }
        }
        Commands::Export { file, output, frame: frame_idx } => {
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
