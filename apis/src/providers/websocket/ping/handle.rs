use crate::providers::ping::PingSignal;
use chrono::{DateTime, Utc};

use leptos::*;

pub fn handle_ping(ping_sent: DateTime<Utc>) {
    let mut ping = expect_context::<PingSignal>();
    ping.update_ping(ping_sent);
}
