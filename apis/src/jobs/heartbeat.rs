use crate::websocket::WsHub;
use actix_web::web::Data;
use std::sync::Arc;
use std::time::Duration;

pub fn run(hub: Data<Arc<WsHub>>) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(3));
        loop {
            interval.tick().await;
            hub.game_heartbeat().await;
        }
    });
}
