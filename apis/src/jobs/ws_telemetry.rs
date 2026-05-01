use crate::websocket::{diff_and_format, WebsocketData, WsHub};
use actix_web::web::Data;
use std::sync::Arc;
use std::{
    fs::OpenOptions,
    io::Write,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const CSV_HEADER: &str = "timestamp,interval_secs,\
    max_queue_depth,active_sockets,active_users,active_games,\
    drops_full_user,drops_full_game,drops_full_gamespec,drops_full_global,drops_full_tour,drops_full_direct,\
    drops_closed_user,drops_closed_game,drops_closed_gamespec,drops_closed_global,drops_closed_tour,drops_closed_direct,\
    from_model_calls,state_replays,tv_broadcasts,games_finalized,load_user_state_queued,load_user_state_in_flight,\
    lags_trackers,game_start_games_date,\
    chat_tournament_channels,chat_tournament_msgs,\
    chat_games_public_channels,chat_games_public_msgs,\
    chat_games_private_channels,chat_games_private_msgs,\
    chat_direct_pairs,chat_direct_msgs,chat_direct_lookup_users,\
    sessions_outer,sessions_inner_total,membership_games_sockets,membership_sockets_games,\
    game_response_cache,last_tv_broadcast,process_vm_rss_bytes,process_vm_hwm_bytes";

pub fn run(data: Data<WebsocketData>, hub: Data<Arc<WsHub>>, interval_secs: u64) {
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

    let _ = data; // retained for backward compatibility with the call site

    actix_rt::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        let mut prev = hub.snapshot_with_state();
        loop {
            interval.tick().await;
            let curr = hub.snapshot_with_state();
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
                        "{ts},{interval_secs},\
                         {},{},{},{},\
                         {},{},{},{},{},{},\
                         {},{},{},{},{},{},\
                         {},{},{},{},{},{},\
                         {},{},\
                         {},{},\
                         {},{},\
                         {},{},\
                         {},{},{},\
                         {},{},{},{},\
                         {},{},{},{}",
                        curr.max_queue_depth_seen,
                        curr.active_sockets,
                        curr.active_users,
                        curr.active_games,
                        d(f[0], fp[0]), d(f[1], fp[1]), d(f[2], fp[2]),
                        d(f[3], fp[3]), d(f[4], fp[4]), d(f[5], fp[5]),
                        d(c[0], cp[0]), d(c[1], cp[1]), d(c[2], cp[2]),
                        d(c[3], cp[3]), d(c[4], cp[4]), d(c[5], cp[5]),
                        d(curr.from_model_calls_total, prev.from_model_calls_total),
                        d(curr.state_replays_total, prev.state_replays_total),
                        d(curr.tv_broadcasts_total, prev.tv_broadcasts_total),
                        d(curr.games_finalized_total, prev.games_finalized_total),
                        curr.load_user_state_queued,
                        curr.load_user_state_in_flight,
                        curr.lags_trackers_len,
                        curr.game_start_games_date_len,
                        curr.chat_tournament_channels,
                        curr.chat_tournament_msgs,
                        curr.chat_games_public_channels,
                        curr.chat_games_public_msgs,
                        curr.chat_games_private_channels,
                        curr.chat_games_private_msgs,
                        curr.chat_direct_pairs,
                        curr.chat_direct_msgs,
                        curr.chat_direct_lookup_users,
                        curr.sessions_outer_len,
                        curr.sessions_inner_total,
                        curr.membership_games_sockets_len,
                        curr.membership_sockets_games_len,
                        curr.game_response_cache_len,
                        curr.last_tv_broadcast_len,
                        curr.process_vm_rss_bytes,
                        curr.process_vm_hwm_bytes,
                    );
                }
            }

            prev = curr;
        }
    });
}
