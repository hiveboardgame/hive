use anyhow::Result;
use clap::{Parser, Subcommand};
use log::info;

mod common;
mod game_stats;
mod games_report;
mod seed;
mod users;

#[derive(Parser)]
#[command(name = "hive-scripts")]
#[command(about = "Hive database management and utility scripts")]
#[command(version = "1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Seed the database with test data for development
    Seed {
        /// Number of test users to create
        #[arg(short, long, default_value_t = 20)]
        users: usize,

        /// Number of games each user should play
        #[arg(short, long, default_value_t = 15)]
        games_per_user: usize,

        /// Clean up all test data instead of seeding
        #[arg(long)]
        cleanup: bool,

        /// Database URL (overrides DATABASE_URL env var)
        #[arg(long)]
        database_url: Option<String>,
    },

    /// List all users in the database
    ListUsers {
        /// Database URL (overrides DATABASE_URL env var)
        #[arg(long)]
        database_url: Option<String>,
    },

    /// Clean up test data from the database
    Cleanup {
        /// Database URL (overrides DATABASE_URL env var)
        #[arg(long)]
        database_url: Option<String>,
    },

    /// Generate game statistics CSV file
    GameStats {
        /// Database URL (overrides DATABASE_URL env var)
        #[arg(long)]
        database_url: Option<String>,

        /// Number of games to sample (random selection). If not specified, analyzes all games
        #[arg(short, long)]
        sample_size: Option<usize>,

        /// Exclude games from users with bot == true
        #[arg(long)]
        no_bots: bool,
    },

    /// Generate comprehensive games report CSV file
    GamesReport {
        /// Database URL (overrides DATABASE_URL env var)
        #[arg(long)]
        database_url: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    info!("Starting Hive database scripts");

    match cli.command {
        Commands::Seed {
            users,
            games_per_user,
            cleanup,
            database_url,
        } => {
            if cleanup {
                seed::cleanup_test_data(database_url).await?;
            } else {
                seed::run_seed_database(users, games_per_user, database_url).await?;
            }
        }
        Commands::ListUsers { database_url } => {
            users::list_users(database_url).await?;
        }
        Commands::Cleanup { database_url } => {
            users::cleanup_test_data(database_url).await?;
        }
        Commands::GameStats {
            database_url,
            sample_size,
            no_bots,
        } => {
            game_stats::run_game_stats(database_url, sample_size, no_bots).await?;
        }
        Commands::GamesReport { database_url } => {
            games_report::run_games_report(database_url).await?;
        }
    }

    Ok(())
}
