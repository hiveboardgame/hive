use crate::providers::{ApiRequests, PingContext};
use leptos::*;

pub fn handle_ping(nonce: u64, value: f64) {
    let mut ping = expect_context::<PingContext>();
    ping.update_ping(value);
    ApiRequests::new().pong(nonce);
}
