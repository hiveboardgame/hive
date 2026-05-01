use std::sync::atomic::{AtomicU64, Ordering};

use super::messages::MessageDestination;

#[derive(Debug, Default)]
pub struct WsTelemetry {
    // Counters
    pub connections_total: AtomicU64,
    pub handshake_failures: AtomicU64,
    pub messages_received_total: AtomicU64,
    pub bytes_received_total: AtomicU64,
    pub bytes_sent_total: AtomicU64,
    pub dispatches_by_dest: [AtomicU64; 6],
    pub recipient_sends_ok: [AtomicU64; 6],
    pub recipient_drops_full: [AtomicU64; 6],
    pub recipient_drops_closed: [AtomicU64; 6],
    pub disconnects_by_reason: [AtomicU64; 5],
    // Per-game allocation/encoding counters
    pub from_model_calls_total: AtomicU64,
    pub state_replays_total: AtomicU64,
    pub tv_broadcasts_total: AtomicU64,
    pub games_finalized_total: AtomicU64,
    // Loader-task gauges: queued = waiting for semaphore, in_flight = running
    pub load_user_state_queued: AtomicU64,
    pub load_user_state_in_flight: AtomicU64,
    // Gauges
    pub active_sockets: AtomicU64,
    pub active_users: AtomicU64,
    pub active_games: AtomicU64,
    pub lobby_subscribers: AtomicU64,
    // Lossy max-gauge — fetch_max on send, swap(0) on snapshot read
    pub max_queue_depth_seen: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestKind {
    User = 0,
    Game = 1,
    GameSpectators = 2,
    Global = 3,
    Tournament = 4,
    Direct = 5,
}

impl From<&MessageDestination> for DestKind {
    fn from(d: &MessageDestination) -> Self {
        match d {
            MessageDestination::User(_) => DestKind::User,
            MessageDestination::Game(_) => DestKind::Game,
            MessageDestination::GameSpectators(_, _, _) => DestKind::GameSpectators,
            MessageDestination::Global => DestKind::Global,
            MessageDestination::Tournament(_) => DestKind::Tournament,
            MessageDestination::Direct(_) => DestKind::Direct,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DisconnectReason {
    Close = 0,
    Timeout = 1,
    PingFail = 2,
    StreamErr = 3,
    Continuation = 4,
}

#[derive(Debug, Clone, Copy)]
pub enum SendOutcome {
    Ok,
    Full,
    Closed,
}

#[derive(Debug, Clone, Default)]
pub struct TelemetrySnapshot {
    pub connections_total: u64,
    pub handshake_failures: u64,
    pub messages_received_total: u64,
    pub bytes_received_total: u64,
    pub bytes_sent_total: u64,
    pub dispatches_by_dest: [u64; 6],
    pub recipient_sends_ok: [u64; 6],
    pub recipient_drops_full: [u64; 6],
    pub recipient_drops_closed: [u64; 6],
    pub disconnects_by_reason: [u64; 5],
    pub from_model_calls_total: u64,
    pub state_replays_total: u64,
    pub tv_broadcasts_total: u64,
    pub games_finalized_total: u64,
    pub load_user_state_queued: u64,
    pub load_user_state_in_flight: u64,
    pub active_sockets: u64,
    pub active_users: u64,
    pub active_games: u64,
    pub lobby_subscribers: u64,
    pub max_queue_depth_seen: u64,
    // Computed from external state at snapshot time.
    pub lags_trackers_len: u64,
    pub game_start_games_date_len: u64,
    pub chat_tournament_channels: u64,
    pub chat_tournament_msgs: u64,
    pub chat_games_public_channels: u64,
    pub chat_games_public_msgs: u64,
    pub chat_games_private_channels: u64,
    pub chat_games_private_msgs: u64,
    pub chat_direct_pairs: u64,
    pub chat_direct_msgs: u64,
    pub chat_direct_lookup_users: u64,
    pub sessions_outer_len: u64,
    pub sessions_inner_total: u64,
    pub membership_games_sockets_len: u64,
    pub membership_sockets_games_len: u64,
    pub game_response_cache_len: u64,
    pub last_tv_broadcast_len: u64,
    pub process_vm_rss_bytes: u64,
    pub process_vm_hwm_bytes: u64,
}

impl WsTelemetry {
    pub fn record_connect(&self) {
        self.connections_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_handshake_fail(&self) {
        self.handshake_failures.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_disconnect(&self, reason: DisconnectReason) {
        self.disconnects_by_reason[reason as usize].fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_message_received(&self, byte_count: usize) {
        self.messages_received_total.fetch_add(1, Ordering::Relaxed);
        self.bytes_received_total
            .fetch_add(byte_count as u64, Ordering::Relaxed);
    }

    pub fn record_dispatch(&self, dest: DestKind) {
        self.dispatches_by_dest[dest as usize].fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_send(
        &self,
        dest: DestKind,
        outcome: SendOutcome,
        queue_used: usize,
        byte_count: usize,
    ) {
        self.max_queue_depth_seen
            .fetch_max(queue_used as u64, Ordering::Relaxed);
        match outcome {
            SendOutcome::Ok => {
                self.recipient_sends_ok[dest as usize].fetch_add(1, Ordering::Relaxed);
                self.bytes_sent_total
                    .fetch_add(byte_count as u64, Ordering::Relaxed);
            }
            SendOutcome::Full => {
                self.recipient_drops_full[dest as usize].fetch_add(1, Ordering::Relaxed);
            }
            SendOutcome::Closed => {
                self.recipient_drops_closed[dest as usize].fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub fn inc_active_socket(&self) {
        self.active_sockets.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec_active_socket(&self) {
        self.active_sockets.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn inc_active_user(&self) {
        self.active_users.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec_active_user(&self) {
        self.active_users.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn set_active_games(&self, n: u64) {
        self.active_games.store(n, Ordering::Relaxed);
    }

    pub fn set_lobby_subscribers(&self, n: u64) {
        self.lobby_subscribers.store(n, Ordering::Relaxed);
    }

    pub fn inc_from_model(&self) {
        self.from_model_calls_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_state_replay(&self) {
        self.state_replays_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_tv_broadcast(&self) {
        self.tv_broadcasts_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_games_finalized(&self) {
        self.games_finalized_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_load_queued(&self) {
        self.load_user_state_queued.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec_load_queued(&self) {
        self.load_user_state_queued.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> TelemetrySnapshot {
        let load = |a: &AtomicU64| a.load(Ordering::Relaxed);
        let load6 = |arr: &[AtomicU64; 6]| {
            std::array::from_fn(|i| arr[i].load(Ordering::Relaxed))
        };
        let load5 = |arr: &[AtomicU64; 5]| {
            std::array::from_fn(|i| arr[i].load(Ordering::Relaxed))
        };
        TelemetrySnapshot {
            connections_total: load(&self.connections_total),
            handshake_failures: load(&self.handshake_failures),
            messages_received_total: load(&self.messages_received_total),
            bytes_received_total: load(&self.bytes_received_total),
            bytes_sent_total: load(&self.bytes_sent_total),
            dispatches_by_dest: load6(&self.dispatches_by_dest),
            recipient_sends_ok: load6(&self.recipient_sends_ok),
            recipient_drops_full: load6(&self.recipient_drops_full),
            recipient_drops_closed: load6(&self.recipient_drops_closed),
            disconnects_by_reason: load5(&self.disconnects_by_reason),
            from_model_calls_total: load(&self.from_model_calls_total),
            state_replays_total: load(&self.state_replays_total),
            tv_broadcasts_total: load(&self.tv_broadcasts_total),
            games_finalized_total: load(&self.games_finalized_total),
            load_user_state_queued: load(&self.load_user_state_queued),
            load_user_state_in_flight: load(&self.load_user_state_in_flight),
            active_sockets: load(&self.active_sockets),
            active_users: load(&self.active_users),
            active_games: load(&self.active_games),
            lobby_subscribers: load(&self.lobby_subscribers),
            max_queue_depth_seen: self.max_queue_depth_seen.swap(0, Ordering::Relaxed),
            // Filled in by snapshot_with_state at the call site.
            ..TelemetrySnapshot::default()
        }
    }
}

/// RAII guard for `load_user_state_in_flight`. Increments on construction,
/// decrements on drop — covers all early-return paths in `load_user_state`.
pub struct InFlightGuard(std::sync::Arc<WsTelemetry>);

impl InFlightGuard {
    pub fn new(telemetry: std::sync::Arc<WsTelemetry>) -> Self {
        telemetry.load_user_state_in_flight.fetch_add(1, Ordering::Relaxed);
        Self(telemetry)
    }
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.0.load_user_state_in_flight.fetch_sub(1, Ordering::Relaxed);
    }
}

/// RAII guard for `load_user_state_queued`. Tracks tasks that have been
/// spawned but are still waiting for a semaphore permit (i.e. not yet running).
/// Drop it as soon as the permit is acquired; `InFlightGuard` takes over then.
pub struct QueuedGuard(std::sync::Arc<WsTelemetry>);

impl QueuedGuard {
    pub fn new(telemetry: std::sync::Arc<WsTelemetry>) -> Self {
        telemetry.inc_load_queued();
        Self(telemetry)
    }
}

impl Drop for QueuedGuard {
    fn drop(&mut self) {
        self.0.dec_load_queued();
    }
}

/// Parse `VmRSS` and `VmHWM` from `/proc/self/status`. Returns `(rss_bytes, hwm_bytes)`.
/// On non-Linux platforms or read failure, returns `(0, 0)`.
pub fn read_proc_vm_bytes() -> (u64, u64) {
    let Ok(contents) = std::fs::read_to_string("/proc/self/status") else {
        return (0, 0);
    };
    let mut rss_kb = 0u64;
    let mut hwm_kb = 0u64;
    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            rss_kb = parse_kb(rest);
        } else if let Some(rest) = line.strip_prefix("VmHWM:") {
            hwm_kb = parse_kb(rest);
        }
    }
    (rss_kb * 1024, hwm_kb * 1024)
}

fn parse_kb(s: &str) -> u64 {
    s.split_whitespace()
        .next()
        .and_then(|t| t.parse::<u64>().ok())
        .unwrap_or(0)
}

fn fmt_bytes(b: u64) -> String {
    if b < 1024 {
        format!("{b}B")
    } else if b < 1024 * 1024 {
        format!("{:.1}KiB", b as f64 / 1024.0)
    } else {
        format!("{:.1}MiB", b as f64 / (1024.0 * 1024.0))
    }
}

pub fn diff_and_format(
    curr: &TelemetrySnapshot,
    prev: &TelemetrySnapshot,
    interval_secs: u64,
) -> String {
    let d = |c: u64, p: u64| c.saturating_sub(p);
    let disc = &curr.disconnects_by_reason;
    let prev_disc = &prev.disconnects_by_reason;
    let disp = &curr.dispatches_by_dest;
    let prev_disp = &prev.dispatches_by_dest;
    let ok = &curr.recipient_sends_ok;
    let prev_ok = &prev.recipient_sends_ok;
    let full = &curr.recipient_drops_full;
    let prev_full = &prev.recipient_drops_full;
    let closed = &curr.recipient_drops_closed;
    let prev_closed = &prev.recipient_drops_closed;

    let disconnects_total = d(disc[0], prev_disc[0])
        + d(disc[1], prev_disc[1])
        + d(disc[2], prev_disc[2])
        + d(disc[3], prev_disc[3])
        + d(disc[4], prev_disc[4]);

    format!(
        "ws_telemetry interval={interval_secs}s\n  \
         gauges:           sockets={} users={} games={} lobby={}\n  \
         since_last:       connects={} disconnects={} (close={} timeout={} ping_fail={} stream_err={} continuation={})\n  \
         handshake_failures={}\n  \
         inbound:          msgs={} bytes={}\n  \
         outbound:         bytes={}\n  \
         dispatches:       User={} Game={} GameSpec={} Global={} Tournament={} Direct={}\n  \
         recipient_sends:  User={} Game={} GameSpec={} Global={} Tour={} Direct={}\n  \
         drops_full:       User={} Game={} GameSpec={} Global={} Tour={} Direct={}\n  \
         drops_closed:     User={} Game={} GameSpec={} Global={} Tour={} Direct={}\n  \
         max_queue_depth:  {}/128\n  \
         per_game_calls:   from_model={} state_replays={} tv_broadcasts={} finalized={}\n  \
         loader_queued:    {} loader_in_flight: {}\n  \
         lags_trackers:    {}\n  \
         game_start_dates: {}\n  \
         chat:             tour=({} ch, {} msg) gpub=({} ch, {} msg) gpriv=({} ch, {} msg) direct=({} pairs, {} msg) lookup_users={}\n  \
         sessions:         outer={} inner_total={}\n  \
         membership:       games_sockets={} sockets_games={}\n  \
         caches:           game_response={} last_tv={}\n  \
         process_vm:       rss={} hwm={}",
        curr.active_sockets,
        curr.active_users,
        curr.active_games,
        curr.lobby_subscribers,
        d(curr.connections_total, prev.connections_total),
        disconnects_total,
        d(disc[0], prev_disc[0]),
        d(disc[1], prev_disc[1]),
        d(disc[2], prev_disc[2]),
        d(disc[3], prev_disc[3]),
        d(disc[4], prev_disc[4]),
        d(curr.handshake_failures, prev.handshake_failures),
        d(curr.messages_received_total, prev.messages_received_total),
        fmt_bytes(d(curr.bytes_received_total, prev.bytes_received_total)),
        fmt_bytes(d(curr.bytes_sent_total, prev.bytes_sent_total)),
        d(disp[0], prev_disp[0]),
        d(disp[1], prev_disp[1]),
        d(disp[2], prev_disp[2]),
        d(disp[3], prev_disp[3]),
        d(disp[4], prev_disp[4]),
        d(disp[5], prev_disp[5]),
        d(ok[0], prev_ok[0]),
        d(ok[1], prev_ok[1]),
        d(ok[2], prev_ok[2]),
        d(ok[3], prev_ok[3]),
        d(ok[4], prev_ok[4]),
        d(ok[5], prev_ok[5]),
        d(full[0], prev_full[0]),
        d(full[1], prev_full[1]),
        d(full[2], prev_full[2]),
        d(full[3], prev_full[3]),
        d(full[4], prev_full[4]),
        d(full[5], prev_full[5]),
        d(closed[0], prev_closed[0]),
        d(closed[1], prev_closed[1]),
        d(closed[2], prev_closed[2]),
        d(closed[3], prev_closed[3]),
        d(closed[4], prev_closed[4]),
        d(closed[5], prev_closed[5]),
        curr.max_queue_depth_seen,
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
        fmt_bytes(curr.process_vm_rss_bytes),
        fmt_bytes(curr.process_vm_hwm_bytes),
    )
}
