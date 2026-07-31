use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod cleanup;
mod common;
mod moves;
mod tournaments;

#[derive(Parser)]
#[command(name = "script")]
#[command(about = "Seeds and plays tournaments for frontend testing")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create the seed users and a tournament in every state, per mode.
    Tournaments {
        /// Play a few real moves into each game before deciding it, so the
        /// boards are worth opening. Slower.
        #[arg(long)]
        play_moves: bool,

        /// Where to write the fixture manifest the end-to-end tests read.
        #[arg(long, default_value = "apis/end2end/seeded.json")]
        manifest: PathBuf,

        /// Skip writing the manifest.
        #[arg(long)]
        no_manifest: bool,

        #[arg(long)]
        database_url: Option<String>,
    },

    /// Remove the seeded users, their games, and their tournaments.
    Cleanup {
        #[arg(long)]
        database_url: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "script=info".into()),
        )
        .init();

    match Cli::parse().command {
        Commands::Tournaments {
            play_moves,
            manifest,
            no_manifest,
            database_url,
        } => {
            let manifest = (!no_manifest).then_some(manifest);
            tournaments::run(database_url, play_moves, manifest).await
        }
        Commands::Cleanup { database_url } => cleanup::run(database_url).await,
    }
}
