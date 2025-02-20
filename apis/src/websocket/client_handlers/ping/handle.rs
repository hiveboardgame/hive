use crate::providers::{ApiRequestsProvider, PingContext};
use leptos::prelude::*;

pub fn handle_ping(nonce: u64, value: f64) {
    let mut ping = expect_context::<PingContext>();
    let api = expect_context::<ApiRequestsProvider>().0.get_value();
    ping.update_ping(value);
    api.pong(nonce);
}
