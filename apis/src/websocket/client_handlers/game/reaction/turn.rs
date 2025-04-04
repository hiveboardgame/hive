use crate::{
    common::GameActionResponse,
    providers::{games::GamesSignal, GameUpdater},
};
use hive_lib::Turn;
use leptos::prelude::*;

pub fn handle_turn(turn: Turn, gar: GameActionResponse) {
    let mut games = expect_context::<GamesSignal>();
    games.own_games_add(gar.game.clone());
    let game_updater = expect_context::<GameUpdater>();
    game_updater.response.set(Some(gar.clone()));
}
