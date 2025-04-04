use crate::{
    providers::{games::GamesSignal, timer::TimerSignal},
    responses::HeartbeatResponse,
};
use leptos::{logging, prelude::*};

pub fn handle_heartbeat(hb: HeartbeatResponse) {
    let mut games_signal = expect_context::<GamesSignal>();
    let timer = expect_context::<TimerSignal>();
    logging::log!("Got heartbeat: {hb:?}");
    games_signal.update_heartbeat(hb.clone());
    timer.update_from_hb(hb);
}
