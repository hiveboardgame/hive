use crate::websocket::{diff_and_format, WebsocketData};
use actix_web::web::Data;
use std::time::Duration;

pub fn run(data: Data<WebsocketData>, interval_secs: u64) {
    actix_rt::spawn(async move {
        let mut interval =
            tokio::time::interval(Duration::from_secs(interval_secs));
        let mut prev = data.telemetry.snapshot();
        loop {
            interval.tick().await;
            let curr = data.telemetry.snapshot();
            log::warn!("{}", diff_and_format(&curr, &prev, interval_secs));
            prev = curr;
        }
    });
}
