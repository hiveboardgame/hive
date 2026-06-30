use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TelemetryRange {
    LastHour,
    Last24h,
    Last7d,
    All,
}

impl TelemetryRange {
    pub fn cutoff_secs(&self, now_secs: u64) -> u64 {
        match self {
            TelemetryRange::LastHour => now_secs.saturating_sub(60 * 60),
            TelemetryRange::Last24h => now_secs.saturating_sub(24 * 60 * 60),
            TelemetryRange::Last7d => now_secs.saturating_sub(7 * 24 * 60 * 60),
            TelemetryRange::All => 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelemetryRow {
    pub timestamp: u64,
    pub interval_secs: u64,
    pub max_queue_depth: u64,
    pub active_sockets: u64,
    pub active_users: u64,
    pub active_games: u64,
    pub drops_full_user: u64,
    pub drops_full_game: u64,
    pub drops_full_gamespec: u64,
    pub drops_full_global: u64,
    pub drops_full_tour: u64,
    pub drops_full_direct: u64,
    pub drops_closed_user: u64,
    pub drops_closed_game: u64,
    pub drops_closed_gamespec: u64,
    pub drops_closed_global: u64,
    pub drops_closed_tour: u64,
    pub drops_closed_direct: u64,
    pub from_model_calls: u64,
    pub tv_broadcasts: u64,
    pub games_finalized: u64,
    pub load_user_state_queued: u64,
    pub load_user_state_in_flight: u64,
    pub own_state_drops: u64,
    pub lags_trackers: u64,
    pub game_start_games_date: u64,
    pub chat_persist_attempts: u64,
    pub chat_persist_successes: u64,
    pub chat_persist_failures: u64,
    pub chat_message_normalizations: u64,
    pub sessions_outer: u64,
    pub sessions_inner_total: u64,
    pub membership_games_sockets: u64,
    pub membership_sockets_games: u64,
    pub game_response_cache: u64,
    pub last_tv_broadcast: u64,
    pub process_vm_rss_bytes: u64,
    pub process_vm_hwm_bytes: u64,
    pub db_pool_max_size: u64,
    pub load_user_state_permit_max: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PushMetrics {
    pub received: u64,
    pub dropped_queue_full: u64,
    pub suppressed_prefs: u64,
    pub prefs_db_error: u64,
    pub ack_eligible: u64,
    pub ack_suppressed: u64,
    pub ack_fired: u64,
    pub test_pushes: u64,
    pub no_device: u64,
    pub device_db_error: u64,
    pub delivered: u64,
    pub retryable: u64,
    pub token_dead: u64,
    pub failed: u64,
    pub retry_delivered: u64,
    pub retry_gave_up: u64,
}

pub const TELEMETRY_COLUMN_COUNT: usize = 40;

impl TelemetryRow {
    /// Parse a single CSV row. Returns None if the row has the wrong number of
    /// fields or any field fails to parse as u64.
    pub fn from_csv_line(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() != TELEMETRY_COLUMN_COUNT {
            return None;
        }
        let p = |i: usize| parts[i].trim().parse::<u64>().ok();
        Some(Self {
            timestamp: p(0)?,
            interval_secs: p(1)?,
            max_queue_depth: p(2)?,
            active_sockets: p(3)?,
            active_users: p(4)?,
            active_games: p(5)?,
            drops_full_user: p(6)?,
            drops_full_game: p(7)?,
            drops_full_gamespec: p(8)?,
            drops_full_global: p(9)?,
            drops_full_tour: p(10)?,
            drops_full_direct: p(11)?,
            drops_closed_user: p(12)?,
            drops_closed_game: p(13)?,
            drops_closed_gamespec: p(14)?,
            drops_closed_global: p(15)?,
            drops_closed_tour: p(16)?,
            drops_closed_direct: p(17)?,
            from_model_calls: p(18)?,
            tv_broadcasts: p(19)?,
            games_finalized: p(20)?,
            load_user_state_queued: p(21)?,
            load_user_state_in_flight: p(22)?,
            own_state_drops: p(23)?,
            lags_trackers: p(24)?,
            game_start_games_date: p(25)?,
            chat_persist_attempts: p(26)?,
            chat_persist_successes: p(27)?,
            chat_persist_failures: p(28)?,
            chat_message_normalizations: p(29)?,
            sessions_outer: p(30)?,
            sessions_inner_total: p(31)?,
            membership_games_sockets: p(32)?,
            membership_sockets_games: p(33)?,
            game_response_cache: p(34)?,
            last_tv_broadcast: p(35)?,
            process_vm_rss_bytes: p(36)?,
            process_vm_hwm_bytes: p(37)?,
            db_pool_max_size: p(38)?,
            load_user_state_permit_max: p(39)?,
        })
    }
}
