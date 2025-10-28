use rand::Rng;
use tokio::time::{interval as interval_fn, Duration};

use crate::common::ServerMessage;
use crate::websocket::new_style::server::{tasks, ServerData, TabData};

const PING_INTERVAL_MS: u64 = 1000; // consistent with previous implementation

pub async fn ping_client(tab: &TabData, server: &ServerData) {
    let mut interval = interval_fn(Duration::from_millis(PING_INTERVAL_MS));
    //firs tick is imediate
    interval.tick().await;
    loop {
        interval.tick().await;
        let nonce = rand::rng().random();
        tab.update_pings(nonce);
        let message = ServerMessage::Ping {
            nonce,
            value: tab.pings_value(),
        };
        tab.send(message, server);
    }
}
