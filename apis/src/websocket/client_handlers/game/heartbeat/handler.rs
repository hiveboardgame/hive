use crate::{
    providers::{games::GamesSignal, GameUpdater},
    responses::HeartbeatResponse,
};
use leptos::{logging, prelude::*};

pub fn handle_heartbeat(hb: HeartbeatResponse) {
    let mut games_signal = expect_context::<GamesSignal>();
    let game_updater = expect_context::<GameUpdater>();
    logging::log!("Got heartbeat: {hb:?}");
    games_signal.update_heartbeat(hb.clone());
    game_updater.heartbeat.set(hb);
}
