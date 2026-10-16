use anyhow::Result;
use clap::{Parser, Subcommand};
use script::synthetic_tournaments;
use tracing_subscriber::EnvFilter;

mod common;

#[derive(Parser)]
#[command(name = "script")]
#[command(about = "Hive development and maintenance utilities")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create or remove deterministic, organizer-owned local tournament QA data.
    SyntheticTournaments {
        #[command(subcommand)]
        command: SyntheticTournamentCommand,
    },
}

#[derive(Subcommand)]
enum SyntheticTournamentCommand {
    /// Seed the complete local tournament manual-QA fixture set.
    Seed {
        #[arg(long)]
        confirm_database_name: String,
        #[arg(long)]
        database_url: Option<String>,
        #[arg(long, default_value = "http://localhost:3000")]
        app_url: String,
        /// Reproduce a particular randomized fixture cohort.
        #[arg(long)]
        seed: Option<u64>,
    },
    /// Re-arm the undersubscribed fixture for a repeatable due-worker check.
    ArmDue {
        #[arg(long)]
        confirm_database_name: String,
        #[arg(long)]
        database_url: Option<String>,
        #[arg(long, default_value = "http://localhost:3000")]
        app_url: String,
    },
    /// Delete only tournaments and accounts owned by the synthetic marker.
    Cleanup {
        #[arg(long)]
        confirm_database_name: String,
        #[arg(long)]
        database_url: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "script=info".into()))
        .init();

    match Cli::parse().command {
        Commands::SyntheticTournaments { command } => match command {
            SyntheticTournamentCommand::Seed {
                confirm_database_name,
                database_url,
                app_url,
                seed,
            } => {
                let mut conn = common::setup_database(database_url).await?;
                synthetic_tournaments::seed(&mut conn, &confirm_database_name, &app_url, seed).await
            }
            SyntheticTournamentCommand::ArmDue {
                confirm_database_name,
                database_url,
                app_url,
            } => {
                let mut conn = common::setup_database(database_url).await?;
                synthetic_tournaments::arm_due(&mut conn, &confirm_database_name, &app_url).await
            }
            SyntheticTournamentCommand::Cleanup {
                confirm_database_name,
                database_url,
            } => {
                let mut conn = common::setup_database(database_url).await?;
                synthetic_tournaments::cleanup(&mut conn, &confirm_database_name).await
            }
        },
    }
}
