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
    pub chat_tournament_channels: u64,
    pub chat_tournament_msgs: u64,
    pub chat_games_public_channels: u64,
    pub chat_games_public_msgs: u64,
    pub chat_games_private_channels: u64,
    pub chat_games_private_msgs: u64,
    pub chat_direct_pairs: u64,
    pub chat_direct_msgs: u64,
    pub chat_direct_lookup_users: u64,
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

pub const TELEMETRY_COLUMN_COUNT: usize = 45;
const LEGACY_TELEMETRY_COLUMN_COUNT: usize = 43;

impl TelemetryRow {
    /// Parse a single CSV row. Returns None if the row has the wrong number of
    /// fields or any field fails to parse as u64.
    pub fn from_csv_line(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() != TELEMETRY_COLUMN_COUNT && parts.len() != LEGACY_TELEMETRY_COLUMN_COUNT {
            return None;
        }
        let p = |i: usize| parts[i].trim().parse::<u64>().ok();
        let optional = |i: usize| parts.get(i).and_then(|v| v.trim().parse::<u64>().ok());
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
            chat_tournament_channels: p(26)?,
            chat_tournament_msgs: p(27)?,
            chat_games_public_channels: p(28)?,
            chat_games_public_msgs: p(29)?,
            chat_games_private_channels: p(30)?,
            chat_games_private_msgs: p(31)?,
            chat_direct_pairs: p(32)?,
            chat_direct_msgs: p(33)?,
            chat_direct_lookup_users: p(34)?,
            sessions_outer: p(35)?,
            sessions_inner_total: p(36)?,
            membership_games_sockets: p(37)?,
            membership_sockets_games: p(38)?,
            game_response_cache: p(39)?,
            last_tv_broadcast: p(40)?,
            process_vm_rss_bytes: p(41)?,
            process_vm_hwm_bytes: p(42)?,
            db_pool_max_size: optional(43).unwrap_or_default(),
            load_user_state_permit_max: optional(44).unwrap_or_default(),
        })
    }
}
