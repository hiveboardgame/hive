use crate::providers::games::GamesSignal;

use leptos::*;

pub fn handle_timeout(nanoid: String) {
    let mut games = expect_context::<GamesSignal>();
    games.own_games_remove(&nanoid);
    games.live_games_remove(&nanoid);
}
