use anyhow::Result;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use script::synthetic_tournaments;
use std::env;
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
        /// Base URL for printed links; defaults to the direct instance address, then APP_ORIGIN,
        /// then http://localhost:3000.
        #[arg(long)]
        app_url: Option<String>,
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
        /// Base URL for printed links; defaults to the direct instance address, then APP_ORIGIN,
        /// then http://localhost:3000.
        #[arg(long)]
        app_url: Option<String>,
    },
    /// Delete only tournaments and accounts owned by the synthetic marker.
    Cleanup {
        #[arg(long)]
        confirm_database_name: String,
        #[arg(long)]
        database_url: Option<String>,
    },
}

fn local_app_url() -> String {
    let nonempty_env = |name| env::var(name).ok().filter(|value| !value.trim().is_empty());
    nonempty_env("LEPTOS_SITE_ADDR")
        .map(|address| direct_app_url(&address))
        .or_else(|| nonempty_env("APP_ORIGIN"))
        .unwrap_or_else(|| String::from("http://localhost:3000"))
}

fn direct_app_url(address: &str) -> String {
    if let Some((host, port)) = address.rsplit_once(':') {
        if matches!(host, "0.0.0.0" | "::" | "[::]") {
            return format!("http://localhost:{port}");
        }
    }
    format!("http://{address}")
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
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
                let app_url = app_url.unwrap_or_else(local_app_url);
                synthetic_tournaments::seed(&mut conn, &confirm_database_name, &app_url, seed).await
            }
            SyntheticTournamentCommand::ArmDue {
                confirm_database_name,
                database_url,
                app_url,
            } => {
                let mut conn = common::setup_database(database_url).await?;
                let app_url = app_url.unwrap_or_else(local_app_url);
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

#[cfg(test)]
mod tests {
    use super::direct_app_url;

    #[test]
    fn wildcard_listen_addresses_become_connectable_links() {
        for address in ["0.0.0.0:3110", "[::]:3110", ":::3110"] {
            assert_eq!(direct_app_url(address), "http://localhost:3110");
        }
        for address in ["127.0.0.1:3110", "localhost:3110", "[::1]:3110"] {
            assert_eq!(direct_app_url(address), format!("http://{address}"));
        }
    }
}
