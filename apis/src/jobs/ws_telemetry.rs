use crate::websocket::{diff_and_format, WebsocketData};
use actix_web::web::Data;
use std::{
    fs::OpenOptions,
    io::Write,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const CSV_HEADER: &str = "timestamp,interval_secs,\
    max_queue_depth,active_sockets,active_users,active_games,\
    drops_full_user,drops_full_game,drops_full_gamespec,drops_full_global,drops_full_tour,drops_full_direct,\
    drops_closed_user,drops_closed_game,drops_closed_gamespec,drops_closed_global,drops_closed_tour,drops_closed_direct";

pub fn run(data: Data<WebsocketData>, interval_secs: u64) {
    let metrics_path = std::env::var("WS_METRICS_LOG_FILE").ok();

    if let Some(ref path) = metrics_path {
        // Write header only when starting a new file.
        let is_new = std::fs::metadata(path).map(|m| m.len() == 0).unwrap_or(true);
        if is_new {
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(f, "{CSV_HEADER}");
            }
        }
    }

    actix_rt::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        let mut prev = data.telemetry.snapshot();
        loop {
            interval.tick().await;
            let curr = data.telemetry.snapshot();
            log::warn!("{}", diff_and_format(&curr, &prev, interval_secs));

            if let Some(ref path) = metrics_path {
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let d = |c: u64, p: u64| c.saturating_sub(p);
                let f = &curr.recipient_drops_full;
                let fp = &prev.recipient_drops_full;
                let c = &curr.recipient_drops_closed;
                let cp = &prev.recipient_drops_closed;
                if let Ok(mut file) = OpenOptions::new().append(true).open(path) {
                    let _ = writeln!(
                        file,
                        "{ts},{interval_secs},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                        curr.max_queue_depth_seen, // already a since-last-snapshot value (swap(0))
                        curr.active_sockets,
                        curr.active_users,
                        curr.active_games,
                        d(f[0], fp[0]), d(f[1], fp[1]), d(f[2], fp[2]),
                        d(f[3], fp[3]), d(f[4], fp[4]), d(f[5], fp[5]),
                        d(c[0], cp[0]), d(c[1], cp[1]), d(c[2], cp[2]),
                        d(c[3], cp[3]), d(c[4], cp[4]), d(c[5], cp[5]),
                    );
                }
            }

            prev = curr;
        }
    });
}
