use crate::websocket::WsHub;
use actix_web::web::Data;
use std::{
    fs::OpenOptions,
    io::Write,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const CSV_HEADER: &str = "timestamp,interval_secs,\
    max_queue_depth,active_sockets,active_users,active_games,\
    drops_full_user,drops_full_game,drops_full_gamespec,drops_full_global,drops_full_tour,drops_full_direct,drops_full_chat_subscribers,\
    drops_closed_user,drops_closed_game,drops_closed_gamespec,drops_closed_global,drops_closed_tour,drops_closed_direct,drops_closed_chat_subscribers,\
    from_model_calls,tv_broadcasts,games_finalized,load_user_state_queued,load_user_state_in_flight,own_state_drops,\
    lags_trackers,game_start_games_date,\
    chat_persist_attempts,chat_persist_successes,chat_persist_failures,chat_message_normalizations,\
    sessions_outer,sessions_inner_total,membership_games_sockets,membership_sockets_games,\
    game_response_cache,last_tv_broadcast,process_vm_rss_bytes,process_vm_hwm_bytes,\
    db_pool_max_size,load_user_state_permit_max";

/// Spawn the periodic WS telemetry snapshot task.
///
/// Each tick takes a read lock on `lags`, `game_start.games_date`, and the
/// four chat maps, then sums message counts across every channel. Cost is
/// O(total chat channels) per tick, plus 5 short read-lock acquisitions.
/// Recommended minimum interval: **30 s**. Below ~10 s the snapshot starts
/// to compete with real WS traffic for the chat locks; do not run sub-second.
///
/// `csv_path` controls CSV writing: `Some(path)` writes one row per tick,
/// `None` skips CSV and only emits the periodic log line.
pub fn run(hub: Data<Arc<WsHub>>, interval_secs: u64, csv_path: Option<String>) {
    let metrics_path = csv_path;

    if let Some(ref path) = metrics_path {
        // Write header only when starting a new file.
        let is_new = std::fs::metadata(path)
            .map(|m| m.len() == 0)
            .unwrap_or(true);
        if is_new {
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(f, "{CSV_HEADER}");
            }
        }
    }

    actix_rt::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        let mut prev = hub.snapshot_with_state();
        loop {
            interval.tick().await;
            let curr = hub.snapshot_with_state();

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
                    let fields = [
                        curr.max_queue_depth_seen,
                        curr.active_sockets,
                        curr.active_users,
                        curr.active_games,
                        d(f[0], fp[0]),
                        d(f[1], fp[1]),
                        d(f[2], fp[2]),
                        d(f[3], fp[3]),
                        d(f[4], fp[4]),
                        d(f[5], fp[5]),
                        d(f[6], fp[6]),
                        d(c[0], cp[0]),
                        d(c[1], cp[1]),
                        d(c[2], cp[2]),
                        d(c[3], cp[3]),
                        d(c[4], cp[4]),
                        d(c[5], cp[5]),
                        d(c[6], cp[6]),
                        d(curr.from_model_calls_total, prev.from_model_calls_total),
                        d(curr.tv_broadcasts_total, prev.tv_broadcasts_total),
                        d(curr.games_finalized_total, prev.games_finalized_total),
                        curr.load_user_state_queued,
                        curr.load_user_state_in_flight,
                        d(curr.own_state_drops_total, prev.own_state_drops_total),
                        curr.lags_trackers_len,
                        curr.game_start_games_date_len,
                        d(
                            curr.chat_persist_attempts_total,
                            prev.chat_persist_attempts_total,
                        ),
                        d(
                            curr.chat_persist_successes_total,
                            prev.chat_persist_successes_total,
                        ),
                        d(
                            curr.chat_persist_failures_total,
                            prev.chat_persist_failures_total,
                        ),
                        d(
                            curr.chat_message_normalizations_total,
                            prev.chat_message_normalizations_total,
                        ),
                        curr.sessions_outer_len,
                        curr.sessions_inner_total,
                        curr.membership_games_sockets_len,
                        curr.membership_sockets_games_len,
                        curr.game_response_cache_len,
                        curr.last_tv_broadcast_len,
                        curr.process_vm_rss_bytes,
                        curr.process_vm_hwm_bytes,
                        curr.db_pool_max_size,
                        curr.load_user_state_permit_max,
                    ]
                    .map(|value| value.to_string());
                    let _ = writeln!(file, "{ts},{interval_secs},{}", fields.join(","));
                }
            }

            prev = curr;
        }
    });
}
