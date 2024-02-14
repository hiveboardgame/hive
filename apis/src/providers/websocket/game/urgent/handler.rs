use crate::providers::auth_context::AuthContext;
use crate::{providers::games::GamesSignal, responses::game::GameResponse};
use leptos::*;
use leptos::logging::log;

pub fn handle_urgent(games: Vec<GameResponse>) {
    let mut games_signal = expect_context::<GamesSignal>();
    let auth_context = expect_context::<AuthContext>();
    log!("Got {:?} urgent games user is: {:?}", games.iter().map(|g|g.nanoid.clone()).collect::<Vec<String>>(), untrack(auth_context.user));
    games_signal.own_games_set(games);
}
