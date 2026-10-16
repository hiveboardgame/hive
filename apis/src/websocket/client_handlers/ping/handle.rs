use crate::providers::PingContext;
use leptos::prelude::*;

pub fn handle_ping(value: f64) {
    let mut ping = expect_context::<PingContext>();
    ping.update_ping(value);
}
