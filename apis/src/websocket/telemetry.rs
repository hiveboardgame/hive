use std::sync::atomic::{AtomicU64, Ordering};

use super::{
    messages::MessageDestination,
    server_handlers::chat::metrics,
    ws_hub::LOAD_USER_STATE_CONCURRENCY,
};
use db_lib::DB_POOL_MAX_SIZE;

#[derive(Debug, Default)]
pub struct WsTelemetry {
    // Counters
    pub connections_total: AtomicU64,
    pub handshake_failures: AtomicU64,
    pub messages_received_total: AtomicU64,
    pub bytes_received_total: AtomicU64,
    pub bytes_sent_total: AtomicU64,
    pub dispatches_by_dest: [AtomicU64; DEST_KIND_COUNT],
    pub recipient_sends_ok: [AtomicU64; DEST_KIND_COUNT],
    pub recipient_drops_full: [AtomicU64; DEST_KIND_COUNT],
    pub recipient_drops_closed: [AtomicU64; DEST_KIND_COUNT],
    pub disconnects_by_reason: [AtomicU64; DISCONNECT_REASON_COUNT],
    // Per-game allocation/encoding counters
    pub from_model_calls_total: AtomicU64,
    pub tv_broadcasts_total: AtomicU64,
    pub games_finalized_total: AtomicU64,
    // Loader-task gauges: queued = waiting for semaphore, in_flight = running
    pub load_user_state_queued: AtomicU64,
    pub load_user_state_in_flight: AtomicU64,
    /// Drops from `load_user_state` sends (urgent, invitations, schedule,
    /// challenges, batch). These bypass `dispatch`, but are also recorded as
    /// `DestKind::User` sends so Full/Closed and queue depth remain visible.
    pub own_state_drops_total: AtomicU64,
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
    ChatSubscribers = 6,
}

pub const DEST_KIND_COUNT: usize = 7;

// Compile-time guard: bumps a const_assertion if the enum's max discriminant
// outgrows the count. Add new variants with explicit discriminants and bump
// DEST_KIND_COUNT in lockstep.
const _: () = {
    assert!((DestKind::ChatSubscribers as usize) == DEST_KIND_COUNT - 1);
};

impl From<&MessageDestination> for DestKind {
    fn from(d: &MessageDestination) -> Self {
        match d {
            MessageDestination::User(_) => DestKind::User,
            MessageDestination::Game(_) => DestKind::Game,
            MessageDestination::GameSpectators(_, _, _) => DestKind::GameSpectators,
            MessageDestination::ChatSubscribers(_) => DestKind::ChatSubscribers,
            MessageDestination::Global => DestKind::Global,
            MessageDestination::Tournament(_, _) => DestKind::Tournament,
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
}

pub const DISCONNECT_REASON_COUNT: usize = 4;

const _: () = {
    assert!((DisconnectReason::StreamErr as usize) == DISCONNECT_REASON_COUNT - 1);
};

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
    pub dispatches_by_dest: [u64; DEST_KIND_COUNT],
    pub recipient_sends_ok: [u64; DEST_KIND_COUNT],
    pub recipient_drops_full: [u64; DEST_KIND_COUNT],
    pub recipient_drops_closed: [u64; DEST_KIND_COUNT],
    pub disconnects_by_reason: [u64; DISCONNECT_REASON_COUNT],
    pub from_model_calls_total: u64,
    pub tv_broadcasts_total: u64,
    pub games_finalized_total: u64,
    pub load_user_state_queued: u64,
    pub load_user_state_in_flight: u64,
    pub load_user_state_permit_max: u64,
    pub db_pool_max_size: u64,
    pub own_state_drops_total: u64,
    pub active_sockets: u64,
    pub active_users: u64,
    pub active_games: u64,
    pub lobby_subscribers: u64,
    pub max_queue_depth_seen: u64,
    pub chat_persist_attempts_total: u64,
    pub chat_persist_successes_total: u64,
    pub chat_persist_failures_total: u64,
    pub chat_message_normalizations_total: u64,
    // Computed from external state at snapshot time.
    pub lags_trackers_len: u64,
    pub game_start_games_date_len: u64,
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

    pub fn inc_own_state_drop(&self) {
        self.own_state_drops_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Read all counters/gauges into a `TelemetrySnapshot`.
    ///
    /// # Side effect
    ///
    /// `max_queue_depth_seen` is **reset to 0** on every call (it's a
    /// max-since-last-read gauge, not a running maximum). Only one caller
    /// should poll `snapshot()` or you'll observe interval-skew: each
    /// reader will see a fraction of the actual peak. Today only
    /// `jobs::ws_telemetry` drains it; if you add a second reader (e.g. a
    /// Prometheus `/metrics` endpoint), split the destructive read into a
    /// separate method and have `snapshot()` use `load()` instead.
    pub fn snapshot(&self) -> TelemetrySnapshot {
        let load = |a: &AtomicU64| a.load(Ordering::Relaxed);
        let load_dest = |arr: &[AtomicU64; DEST_KIND_COUNT]| {
            std::array::from_fn(|i| arr[i].load(Ordering::Relaxed))
        };
        let load_disc = |arr: &[AtomicU64; DISCONNECT_REASON_COUNT]| {
            std::array::from_fn(|i| arr[i].load(Ordering::Relaxed))
        };
        let chat_metrics = metrics::snapshot();
        TelemetrySnapshot {
            connections_total: load(&self.connections_total),
            handshake_failures: load(&self.handshake_failures),
            messages_received_total: load(&self.messages_received_total),
            bytes_received_total: load(&self.bytes_received_total),
            bytes_sent_total: load(&self.bytes_sent_total),
            dispatches_by_dest: load_dest(&self.dispatches_by_dest),
            recipient_sends_ok: load_dest(&self.recipient_sends_ok),
            recipient_drops_full: load_dest(&self.recipient_drops_full),
            recipient_drops_closed: load_dest(&self.recipient_drops_closed),
            disconnects_by_reason: load_disc(&self.disconnects_by_reason),
            from_model_calls_total: load(&self.from_model_calls_total),
            tv_broadcasts_total: load(&self.tv_broadcasts_total),
            games_finalized_total: load(&self.games_finalized_total),
            load_user_state_queued: load(&self.load_user_state_queued),
            load_user_state_in_flight: load(&self.load_user_state_in_flight),
            load_user_state_permit_max: LOAD_USER_STATE_CONCURRENCY as u64,
            db_pool_max_size: DB_POOL_MAX_SIZE as u64,
            own_state_drops_total: load(&self.own_state_drops_total),
            active_sockets: load(&self.active_sockets),
            active_users: load(&self.active_users),
            active_games: load(&self.active_games),
            lobby_subscribers: load(&self.lobby_subscribers),
            max_queue_depth_seen: self.max_queue_depth_seen.swap(0, Ordering::Relaxed),
            chat_persist_attempts_total: chat_metrics.persist_attempts_total,
            chat_persist_successes_total: chat_metrics.persist_successes_total,
            chat_persist_failures_total: chat_metrics.persist_failures_total,
            chat_message_normalizations_total: chat_metrics.message_normalizations_total,
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
        telemetry
            .load_user_state_in_flight
            .fetch_add(1, Ordering::Relaxed);
        Self(telemetry)
    }
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.0
            .load_user_state_in_flight
            .fetch_sub(1, Ordering::Relaxed);
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
