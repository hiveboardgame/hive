use hivegame_bot_api::HiveGame;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, Semaphore};
use tracing::{debug, error, info};

mod turn_tracker;
use turn_tracker::{TurnTracker, TurnTracking};
mod ai;
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
        let producer_handle = tokio::spawn(producer_task(
            sender.clone(),
            turn_tracker.clone(),
            api_clone,
            bot,
        ));
        producer_handles.push(producer_handle);
    }

    let consumer_handle = tokio::spawn(consumer_task(
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

async fn producer_task(
    sender: mpsc::Sender<BotGameTurn>,
    turn_tracker: TurnTracker,
    api: Arc<HiveGameApi>,
    bot: BotConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Producer task started for bot: {}", bot.name);

    // Authenticate to get token
    let token = match api.auth(&bot.email, &bot.password).await {
        Ok(token) => {
            info!("Authentication successful for bot: {}", bot.name);
            token
        }
        Err(e) => {
            error!("Authentication failed for bot {}: {}", bot.name, e);
            return Err(Box::new(e));
        }
    };

    loop {
        // Get challenges for this bot
        match api.challenges(&token).await {
            Ok(challenge_ids) => {
                if !challenge_ids.is_empty() {
                    info!(
                        "Bot {} has {} pending challenges: {:?}",
                        bot.name,
                        challenge_ids.len(),
                        challenge_ids
                    );

                    // Accept each challenge
                    for challenge_id in challenge_ids {
                        match api.accept_challenge(&challenge_id, &token).await {
                            Ok(_) => {
                                info!(
                                    "Bot {} successfully accepted challenge {}",
                                    bot.name, challenge_id
                                );
                            }
                            Err(e) => {
                                error!(
                                    "Bot {} failed to accept challenge {}: {}",
                                    bot.name, challenge_id, e
                                );
                            }
                        }
                    }
                } else {
                    debug!("No challenges found for bot {}", bot.name);
                }
            }
            Err(e) => error!("Failed to fetch challenges for bot {}: {}", bot.name, e),
        }

        match api.get_games(&token).await {
            Ok(game_strings) => {
                debug!(
                    "Retrieved {} games for bot {}",
                    game_strings.len(),
                    bot.name
                );
                for game in game_strings {
                    let hash = game.hash();

                    if turn_tracker.tracked(hash).await {
                        debug!("Game {} already tracked for bot {}", hash, bot.name);
                        continue;
                    }

                    let turn = BotGameTurn {
                        game,
                        hash,
                        bot: bot.clone(),
                        token: token.clone(),
                    };

                    turn_tracker.processing(hash).await;
                    debug!("Processing game {} for bot {}", hash, bot.name);

                    if sender.send(turn).await.is_err() {
                        error!("Failed to send turn to queue for bot {}", bot.name);
                        continue;
                    }
                }
            }
            Err(e) => error!("Failed to fetch games for bot {}: {}", bot.name, e),
        }

        debug!("Starting new cycle for bot {}", bot.name);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn consumer_task(
    receiver: Arc<Mutex<mpsc::Receiver<BotGameTurn>>>,
    semaphore: Arc<Semaphore>,
    active_processes: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
    turn_tracker: TurnTracker,
    api: Arc<HiveGameApi>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Consumer task started");

    loop {
        let mut rx = receiver.lock().await;
        if let Some(turn) = rx.recv().await {
            drop(rx);
            debug!("Received turn for bot {}", turn.bot.name);

            let api_clone = api.clone();
            let handle = tokio::spawn(process_turn(
                turn,
                semaphore.clone(),
                turn_tracker.clone(),
                api_clone,
            ));

            active_processes.lock().await.push(handle);
            cleanup_processes(active_processes.clone()).await;
        }
    }
}

async fn process_turn(
    turn: BotGameTurn,
    semaphore: Arc<Semaphore>,
    turn_tracker: TurnTracker,
    api: Arc<HiveGameApi>,
) {
    let _permit = match semaphore.acquire().await {
        Ok(permit) => permit,
        Err(e) => {
            error!(
                "Failed to acquire semaphore for bot {}: {}",
                turn.bot.name, e
            );
            turn_tracker.processed(turn.hash).await;
            return;
        }
    };

    debug!(
        "Processing turn for bot {} with hash {}",
        turn.bot.name, turn.hash
    );

    let child = match ai::spawn_process(&turn.bot.ai_command, &turn.bot.name) {
        Ok(child) => child,
        Err(e) => {
            error!(
                "Failed to spawn AI process for bot {}: {}",
                turn.bot.name, e
            );
            turn_tracker.processed(turn.hash).await;
            return;
        }
    };

    // Convert game to string using the HiveGame method
    let game_string = turn.game.game_string();

    match ai::run_commands(child, &game_string, &turn.bot.bestmove_command_args).await {
        Ok(bestmove) => {
            info!("Bot '{}' bestmove: '{}'", turn.bot.name, bestmove);

            // Determine the game identifier to use (prefer nanoid, fall back to game_id)
            let game_identifier = match &turn.game.nanoid {
                Some(id) => id.clone(),
                None => turn.game.game_id.clone(),
            };

            // Send the move to the server using the token
            match api
                .play_move(&game_identifier, &bestmove, &turn.token)
                .await
            {
                Ok(_) => {
                    info!(
                        "Move '{}' sent successfully for game {}",
                        bestmove, game_identifier
                    );
                }
                Err(e) => {
                    error!("Failed to send move for bot {}: {}", turn.bot.name, e);
                }
            }
        }
        Err(e) => {
            error!(
                "Error running AI commands for bot '{}': '{}'",
                turn.bot.name, e
            );
        }
    }

    turn_tracker.processed(turn.hash).await;
    debug!(
        "Turn processed for bot {} with hash {}",
        turn.bot.name, turn.hash
    );
}

async fn cleanup_processes(active_processes: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>) {
    let mut processes = active_processes.lock().await;
    let initial_count = processes.len();
    processes.retain(|handle| !handle.is_finished());
    let removed = initial_count - processes.len();
    if removed > 0 {
        debug!("Cleaned up {} finished processes", removed);
    }
}
