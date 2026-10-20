mod ai;
mod bot;
mod cli;
mod config;
mod logging;
mod protocol;
mod session;
mod token;
mod turn_tracker;

use config::Config;
use session::Shared;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    logging::setup_logging()?;
    info!("Starting Hive Hydra");

    // A workspace-wide build enables both of rustls' crypto backends, and then it picks neither.
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|_| "a rustls crypto provider was already installed")?;

    let cli = cli::Cli::parse();
    let config = Config::load_from(cli.config)?;
    info!(
        max_concurrent_processes = config.max_concurrent_processes,
        bots = config.bots.len(),
        "configuration loaded"
    );

    let shared = Arc::new(Shared {
        http: token::client()?,
        websocket_url: config.websocket_url()?,
        base_url: config.base_url,
        engine_slots: Semaphore::new(config.max_concurrent_processes),
    });

    let sessions: Vec<_> = config
        .bots
        .into_iter()
        .map(|bot| tokio::spawn(session::run(bot, shared.clone())))
        .collect();
    for session in sessions {
        session.await?;
    }

    Ok(())
}
