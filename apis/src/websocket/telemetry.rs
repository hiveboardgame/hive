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
    pub active_sockets: u64,
    pub active_users: u64,
    pub active_games: u64,
    pub lobby_subscribers: u64,
    pub max_queue_depth_seen: u64,
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
            active_sockets: load(&self.active_sockets),
            active_users: load(&self.active_users),
            active_games: load(&self.active_games),
            lobby_subscribers: load(&self.lobby_subscribers),
            max_queue_depth_seen: self.max_queue_depth_seen.swap(0, Ordering::Relaxed),
        }
    }
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
         max_queue_depth:  {}/128",
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
    )
}
