use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod codegen;
mod schema;

#[derive(Parser)]
#[command(name = "uproto")]
#[command(about = "Protocol definition and code generation from KDL schemas")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate code from a schema file
    Gen {
        /// Path to the schema file
        schema: PathBuf,

        /// Output Rust file
        #[arg(long)]
        rust: Option<PathBuf>,

        /// Output TypeScript file
        #[arg(long)]
        ts: Option<PathBuf>,
    },

    /// Validate a schema file
    Check {
        /// Path to the schema file
        schema: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Gen { schema, rust, ts } => {
            let schema = schema::Schema::load(&schema)?;

            if let Some(rust_path) = rust {
                let code = codegen::rust::generate(&schema);
                std::fs::write(&rust_path, code)?;
                println!("Generated: {}", rust_path.display());
            }

            if let Some(ts_path) = ts {
                let code = codegen::typescript::generate(&schema);
                std::fs::write(&ts_path, code)?;
                println!("Generated: {}", ts_path.display());
            }
        }
        Commands::Check { schema } => {
            let _schema = schema::Schema::load(&schema)?;
            println!("Schema is valid");
        }
    }

    Ok(())
}
