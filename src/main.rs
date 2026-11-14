mod schema;
mod setup;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "fiveorm", version, about = "FiveORM: database toolkit for FiveM")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create schema.yml, choosing a FiveM framework interactively.
    Init {
        #[arg(long, value_enum)]
        framework: Option<schema::Framework>,
        #[arg(long, default_value = "schema.yml")]
        schema: PathBuf,
    },
    /// Install the FiveORM FiveM resource and configure a resource manifest.
    Setup {
        /// Path to the consuming resource (defaults to the current directory).
        #[arg(long)]
        resource: Option<PathBuf>,
        /// FiveM resources/ directory (auto-discovered when possible).
        #[arg(long)]
        resources_dir: Option<PathBuf>,
        /// server.cfg path (auto-discovered when possible).
        #[arg(long)]
        server_cfg: Option<PathBuf>,
    },
    /// Validate schema.yml without modifying the database.
    Validate {
        #[arg(long, default_value = "schema.yml")]
        schema: PathBuf,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("FiveORM error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Commands::Init { framework, schema } => schema::init(&schema, framework),
        Commands::Setup {
            resource,
            resources_dir,
            server_cfg,
        } => setup::run(resource, resources_dir, server_cfg),
        Commands::Validate { schema } => schema::validate(&schema),
    }
}
