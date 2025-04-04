use crate::{
    providers::games::GamesSignal,
    responses::HeartbeatResponse,
};
use leptos::{logging, prelude::*};

pub fn handle_heartbeat(hb: HeartbeatResponse) {
    let mut games_signal = expect_context::<GamesSignal>();
    games_signal.update_heartbeat(hb.clone());
    logging::log!("Got heartbeat: {hb:?}");
}
