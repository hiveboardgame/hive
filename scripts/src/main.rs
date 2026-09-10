use anyhow::Result;
use clap::{builder::BoolishValueParser, Parser, Subcommand};
use script::{legacy_tournaments, synthetic_tournaments};
use std::path::PathBuf;
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
    /// Audit or freeze the supported tournaments from the legacy schema.
    LegacyTournaments {
        #[command(subcommand)]
        command: LegacyTournamentCommand,
    },
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

#[derive(Subcommand)]
enum LegacyTournamentCommand {
    /// Run the complete read-only legacy classification without writing a bundle.
    Audit {
        #[arg(long)]
        database_url: Option<String>,
    },
    /// Freeze a deterministic JSONL bundle after the application is stopped.
    Export {
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        confirm_database_name: String,
        #[arg(long, value_parser = BoolishValueParser::new())]
        confirm_writes_stopped: bool,
        #[arg(long)]
        database_url: Option<String>,
    },
    /// Apply a verified accepted bundle to an empty canonical tournament schema.
    Import {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        confirm_source_database_name: String,
        #[arg(long)]
        confirm_database_name: String,
        #[arg(long)]
        confirm_bundle_sha256: String,
        #[arg(long, value_parser = BoolishValueParser::new())]
        confirm_writes_stopped: bool,
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
        Commands::LegacyTournaments { command } => match command {
            LegacyTournamentCommand::Audit { database_url } => {
                let mut conn = common::setup_database(database_url).await?;
                legacy_tournaments::run_audit_command(&mut conn).await
            }
            LegacyTournamentCommand::Export {
                output,
                confirm_database_name,
                confirm_writes_stopped,
                database_url,
            } => {
                anyhow::ensure!(
                    confirm_writes_stopped,
                    "export requires --confirm-writes-stopped=true"
                );
                let mut conn = common::setup_database(database_url).await?;
                let published = legacy_tournaments::run_export_command(
                    &mut conn,
                    &output,
                    &confirm_database_name,
                )
                .await?;
                let accepted_records: u64 = published.details.record_counts.values().sum();
                let skipped_records: u64 = published.details.skipped_record_counts.values().sum();
                tracing::info!(
                    output = %published.output.display(),
                    manifest = %published.manifest.display(),
                    sha256 = %published.details.jsonl_sha256,
                    accepted_records,
                    skipped_records,
                    "published accepted legacy tournament export"
                );
                Ok(())
            }
            LegacyTournamentCommand::Import {
                input,
                confirm_source_database_name,
                confirm_database_name,
                confirm_bundle_sha256,
                confirm_writes_stopped,
                database_url,
            } => {
                anyhow::ensure!(
                    confirm_writes_stopped,
                    "import requires --confirm-writes-stopped=true"
                );
                let mut conn = common::setup_database(database_url).await?;
                let report = legacy_tournaments::import::apply(
                    &mut conn,
                    &input,
                    &confirm_source_database_name,
                    &confirm_database_name,
                    &confirm_bundle_sha256,
                )
                .await?;
                tracing::info!(
                    imported_records = report.imported_records,
                    skipped_records = report.skipped_records,
                    tournaments = report.tournaments,
                    slots = report.slots,
                    swiss_rounds = report.swiss_rounds,
                    games = report.games,
                    scheduled_slots = report.scheduled_slots,
                    schedule_offers = report.schedule_offers,
                    schedule_candidates = report.schedule_candidates,
                    final_outcomes = report.final_outcomes,
                    "applied accepted legacy tournament bundle"
                );
                Ok(())
            }
        },
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
