use std::sync::Arc;

use rand::Rng;
use tokio::time::{interval as interval_fn, Duration};

use crate::common::ServerMessage;
use crate::websocket::new_style::server::{ServerData, TabData};

const PING_INTERVAL_MS: u64 = 1000; // consistent with previous implementation

pub async fn ping_client(client: TabData, server_data: Arc<ServerData>) {
    let mut interval = interval_fn(Duration::from_millis(PING_INTERVAL_MS));
    loop {
        interval.tick().await;
        let nonce = rand::rng().random();
        client.update_pings(nonce);
        let message = ServerMessage::Ping {
            nonce,
            value: client.pings_value(),
        };
        client.send(message, &server_data);
    }
}
