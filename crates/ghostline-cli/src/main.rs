use clap::{Parser, Subcommand};

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
            println!("Inspecting: {}", file);
            println!("(reader not yet implemented)");
        }
        Commands::Export { file, output } => {
            println!("Exporting: {} -> {:?}", file, output);
            println!("(exporter not yet implemented)");
        }
    }

    Ok(())
}
