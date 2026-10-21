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
    pub chat_persist_failures: u64,
    pub chat_rate_limit_rejections: u64,
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
    pub lag_moves: u64,
    pub protocol_lag_p50_ms: u64,
    pub protocol_lag_p90_ms: u64,
    pub protocol_lag_missing: u64,
    pub lag_cap_hits: u64,
    pub tcp_rtt_connects: u64,
    pub tcp_rtt_p50_ms: u64,
    pub tcp_rtt_p90_ms: u64,
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

pub const TELEMETRY_COLUMN_COUNT: usize = 46;
// Rows written before the lag columns existed; their lag fields read as zero.
const PRE_LAG_COLUMN_COUNT: usize = 38;

impl TelemetryRow {
    /// Parse a single CSV row. Returns None if the row has the wrong number of
    /// fields or any field fails to parse as u64.
    pub fn from_csv_line(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() != TELEMETRY_COLUMN_COUNT && parts.len() != PRE_LAG_COLUMN_COUNT {
            return None;
        }
        let p = |i: usize| parts[i].trim().parse::<u64>().ok();
        let lag = |i: usize| match parts.get(i) {
            Some(_) => p(i),
            None => Some(0),
        };
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
            chat_persist_failures: p(26)?,
            chat_rate_limit_rejections: p(27)?,
            sessions_outer: p(28)?,
            sessions_inner_total: p(29)?,
            membership_games_sockets: p(30)?,
            membership_sockets_games: p(31)?,
            game_response_cache: p(32)?,
            last_tv_broadcast: p(33)?,
            process_vm_rss_bytes: p(34)?,
            process_vm_hwm_bytes: p(35)?,
            db_pool_max_size: p(36)?,
            load_user_state_permit_max: p(37)?,
            lag_moves: lag(38)?,
            protocol_lag_p50_ms: lag(39)?,
            protocol_lag_p90_ms: lag(40)?,
            protocol_lag_missing: lag(41)?,
            lag_cap_hits: lag(42)?,
            tcp_rtt_connects: lag(43)?,
            tcp_rtt_p50_ms: lag(44)?,
            tcp_rtt_p90_ms: lag(45)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(columns: usize) -> String {
        (0..columns)
            .map(|column| column.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    fn lag_columns(row: &TelemetryRow) -> [u64; TELEMETRY_COLUMN_COUNT - PRE_LAG_COLUMN_COUNT] {
        [
            row.lag_moves,
            row.protocol_lag_p50_ms,
            row.protocol_lag_p90_ms,
            row.protocol_lag_missing,
            row.lag_cap_hits,
            row.tcp_rtt_connects,
            row.tcp_rtt_p50_ms,
            row.tcp_rtt_p90_ms,
        ]
    }

    #[test]
    fn a_row_with_lag_columns_parses_each_in_header_order() {
        let parsed = TelemetryRow::from_csv_line(&row(TELEMETRY_COLUMN_COUNT)).expect("parses");

        assert_eq!(parsed.load_user_state_permit_max, 37);
        assert_eq!(
            lag_columns(&parsed),
            std::array::from_fn(|i| (PRE_LAG_COLUMN_COUNT + i) as u64)
        );
    }

    #[test]
    fn a_row_from_before_the_lag_columns_still_parses() {
        let parsed = TelemetryRow::from_csv_line(&row(PRE_LAG_COLUMN_COUNT)).expect("parses");

        assert_eq!(parsed.load_user_state_permit_max, 37);
        assert_eq!(
            lag_columns(&parsed),
            [0; TELEMETRY_COLUMN_COUNT - PRE_LAG_COLUMN_COUNT]
        );
    }

    #[test]
    fn rows_of_any_other_width_are_rejected() {
        for columns in [PRE_LAG_COLUMN_COUNT - 1, 40, TELEMETRY_COLUMN_COUNT + 1] {
            assert!(
                TelemetryRow::from_csv_line(&row(columns)).is_none(),
                "{columns}"
            );
        }
    }
}
