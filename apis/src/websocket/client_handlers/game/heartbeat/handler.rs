use crate::{
    providers::{games::GamesSignal, timer::TimerSignal},
    responses::HeartbeatResponse,
};
use leptos::prelude::*;

pub fn handle_heartbeat(hb: HeartbeatResponse) {
    let mut games_signal = expect_context::<GamesSignal>();
    games_signal.update_heartbeat(hb.clone());
    let timer = expect_context::<TimerSignal>();
    timer.update_from_hb(hb);
}
