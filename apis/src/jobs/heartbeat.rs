use crate::websocket::WsHub;
use actix_web::web::Data;
use std::{sync::Arc, time::Duration};
use tokio::time::MissedTickBehavior;

pub fn run(hub: Data<Arc<WsHub>>) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(3));
        // If a tick takes longer than 3s (busy DB, many active games), don't
        // immediately fire the next one back-to-back — that would let two
        // game_heartbeat passes overlap and contend for the same locks.
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            hub.game_heartbeat().await;
        }
    });
}
