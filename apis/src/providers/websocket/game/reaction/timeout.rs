use crate::common::server_result::GameActionResponse;
use crate::providers::{game_state::GameStateSignal, games::GamesSignal};
use leptos::*;

pub fn handle_timeout(gar: GameActionResponse) {
    let mut games = expect_context::<GamesSignal>();
    let mut game_state = expect_context::<GameStateSignal>();
    let nanoid = &gar.game.nanoid;
    games.own_games_remove(nanoid);
    games.live_games_remove(nanoid);
    if let Some(current_game_id) = game_state.signal.get_untracked().game_id {
        if current_game_id == *nanoid {
            game_state.set_game_response(gar.game.clone());
        }
    }
}
