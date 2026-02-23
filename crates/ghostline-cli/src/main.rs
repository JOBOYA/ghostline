use base64::Engine;
use clap::{Parser, Subcommand};
use ghostline_core::{GhostlineReader, MAGIC};

#[derive(Parser)]
#[command(name = "ghostline")]
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
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect { file } => {
            let reader = GhostlineReader::open(&file)?;
            let magic = std::str::from_utf8(MAGIC).unwrap_or("?");
            let ts = chrono::DateTime::from_timestamp_millis(reader.started_at as i64)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| format!("{}", reader.started_at));

            println!("Magic:       {}", magic);
            println!("Version:     {}", reader.version);
            println!("Frames:      {}", reader.frame_count());
            println!("Started at:  {}", ts);
        }
        Commands::Export { file, output } => {
            let mut reader = GhostlineReader::open(&file)?;
            let b64 = base64::engine::general_purpose::STANDARD;
            let mut frames = Vec::new();

            for i in 0..reader.frame_count() {
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

    Ok(())
}
