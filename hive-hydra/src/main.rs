use hivegame_bot_api::HiveGame;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, Semaphore};
use tracing::{debug, error, info};

mod turn_tracker;
use turn_tracker::{TurnTracker, TurnTracking};
mod ai;
mod bot;
mod hivegame_bot_api;
use hivegame_bot_api::HiveGameApi;
mod config;
use config::{BotConfig, Config};
mod cli;
mod logging;

struct BotGameTurn {
    game: HiveGame,
    hash: u64,
    bot: BotConfig,
    token: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging first
    logging::setup_logging()?;
    info!("Starting Hive Hydra");

    // Parse command line arguments
    let cli = cli::Cli::parse();
    debug!("CLI arguments parsed");

    // Load configuration from specified file
    let config = Config::load_from(cli.config)?;
    info!(
        "Configuration loaded, max concurrent processes: {}",
        config.max_concurrent_processes
    );

    // Create a shared HiveGameApi instance
    let api = Arc::new(HiveGameApi::new(config.base_url.clone()));

    let (sender, receiver) = mpsc::channel(config.queue_capacity);
    let receiver = Arc::new(Mutex::new(receiver));
    let semaphore = Arc::new(Semaphore::new(config.max_concurrent_processes));
    let active_processes = Arc::new(Mutex::new(Vec::new()));
    let turn_tracker = TurnTracker::new();

    info!(
        "Initialized channel with capacity: {}",
        config.queue_capacity
    );

    let cleanup_tracker = turn_tracker.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            cleanup_tracker.cleanup().await;
            debug!("Cleanup cycle completed");
        }
    });

    // Spawn a producer task for each bot
    let mut producer_handles = Vec::new();
    info!("Starting producer tasks for {} bots", config.bots.len());

    for bot in config.bots {
        info!("Spawning producer task for bot: {}", bot.name);
        let api_clone = api.clone();
        let producer_handle = tokio::spawn(bot::producer_task(
            sender.clone(),
            turn_tracker.clone(),
            api_clone,
            bot,
        ));
        producer_handles.push(producer_handle);
    }

    let consumer_handle = tokio::spawn(bot::consumer_task(
        receiver,
        semaphore,
        active_processes,
        turn_tracker.clone(),
        api.clone(),
    ));
    info!("Consumer task started");

    // Wait for all producers and the consumer
    for handle in producer_handles {
        if let Err(e) = handle.await? {
            error!("Producer task error: {}", e);
        }
    }
    if let Err(e) = consumer_handle.await? {
        error!("Consumer task error: {}", e);
    }

    Ok(())
}
