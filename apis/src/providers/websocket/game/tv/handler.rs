use crate::{providers::games::GamesSignal, responses::GameResponse};

use leptos::*;

pub fn handle_tv(game: GameResponse) {
    let mut games = expect_context::<GamesSignal>();
    games.live_games_add(game);
}
