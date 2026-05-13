use crate::websocket::WsHub;
use actix_web::web::Data;
use std::{sync::Arc, time::Duration};
use tokio::time::MissedTickBehavior;

pub fn run(hub: Data<Arc<WsHub>>) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(1));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            hub.ping_all();
        }
    });
}
