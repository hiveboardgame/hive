use crate::{
    ai,
    config::BotConfig,
    protocol::{ClientRequest, Game, GameAction},
    turn_tracker::{TurnTracker, TurnTracking},
};
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use tracing::{debug, error, info};

pub struct Turn {
    pub game: Game,
    pub position_key: u64,
    pub bot: Arc<BotConfig>,
    pub outbox: mpsc::UnboundedSender<ClientRequest>,
}

pub async fn play_turn(turn: Turn, engine_slots: Arc<Semaphore>, turn_tracker: TurnTracker) {
    if let Some(bestmove) = think(&turn, &engine_slots).await {
        info!(bot = %turn.bot.name, game_id = %turn.game.game_id, bestmove = %bestmove, "playing move");
        let play = ClientRequest::Game {
            game_id: turn.game.game_id.clone(),
            action: GameAction::Play(bestmove),
        };
        if turn.outbox.send(play).is_err() {
            error!(bot = %turn.bot.name, game_id = %turn.game.game_id, "session is gone, move dropped");
        }
    }
    turn_tracker.processed(turn.position_key).await;
}

async fn think(turn: &Turn, engine_slots: &Semaphore) -> Option<String> {
    let bot = &turn.bot.name;
    let game_id = &turn.game.game_id;
    let _permit = match engine_slots.acquire().await {
        Ok(permit) => permit,
        Err(e) => {
            error!(bot = %bot, game_id = %game_id, error = %e, "engine slots closed");
            return None;
        }
    };
    let game_string = turn.game.uhp_game_string();
    debug!(bot = %bot, game_id = %game_id, game_string = %game_string, "thinking");
    let child = match ai::spawn_process(&turn.bot.ai_command, bot) {
        Ok(child) => child,
        Err(e) => {
            error!(bot = %bot, game_id = %game_id, error = %e, "failed to spawn AI process");
            return None;
        }
    };
    match ai::run_commands(child, &game_string, &turn.bot.bestmove_command_args).await {
        Ok(bestmove) => Some(bestmove),
        Err(e) => {
            error!(bot = %bot, game_id = %game_id, error = %e, "AI failed to produce a move");
            None
        }
    }
}
