mod chat;
mod lag_tracking;
pub use chat::Chats;
pub use lag_tracking::{Lags, Pings};
pub mod busybee;
pub mod client_handlers;

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
    mod messages;
    mod start_conn;
    mod telemetry;
    mod tournament_game_start;
    mod ws_connection;
    mod ws_hub;
    pub mod server_handlers;

    pub use start_conn::start_connection;
    pub use telemetry::{diff_and_format, read_proc_vm_bytes, DestKind, DisconnectReason, InFlightGuard, QueuedGuard, SendOutcome, TelemetrySnapshot, WsTelemetry};
    pub use tournament_game_start::TournamentGameStart;
    pub use ws_hub::WsHub;
    pub use messages::{InternalServerMessage, MessageDestination};

    use chrono::{DateTime, Utc};
    use dashmap::DashMap;
    use shared_types::GameId;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    /// Cached responses older than this are rebuilt regardless of `updated_at`
    /// to prevent serving stale user-derived fields (ratings, profile data) that
    /// can change independently of the game row.
    const GAME_RESPONSE_CACHE_TTL: Duration = Duration::from_secs(30);

    fn cache_entry_is_valid(
        cached_updated_at: DateTime<Utc>,
        cached_at: Instant,
        game_updated_at: DateTime<Utc>,
    ) -> bool {
        cached_updated_at == game_updated_at && cached_at.elapsed() < GAME_RESPONSE_CACHE_TTL
    }

    #[derive(Debug)]
    pub struct WebsocketData {
        pub chat_storage: Chats,
        pub game_start: TournamentGameStart,
        pub pings: Pings,
        pub lags: Lags,
        pub telemetry: Arc<WsTelemetry>,
        /// Per-game cache: `(response, cached_at)`. Validated by both
        /// `updated_at` equality and a TTL to catch stale user-derived fields.
        pub game_response_cache: DashMap<GameId, (Arc<crate::responses::GameResponse>, Instant)>,
    }

    impl Default for WebsocketData {
        fn default() -> Self {
            Self {
                chat_storage: Chats::default(),
                game_start: TournamentGameStart::default(),
                pings: Pings::default(),
                lags: Lags::default(),
                telemetry: Arc::new(WsTelemetry::default()),
                game_response_cache: DashMap::new(),
            }
        }
    }

    impl WebsocketData {
        /// Return a cached `GameResponse` if fresh; otherwise build, cache, and return it.
        pub async fn get_or_build_response(
            &self,
            game: &db_lib::models::Game,
            conn: &mut db_lib::DbConn<'_>,
        ) -> anyhow::Result<Arc<crate::responses::GameResponse>> {
            let game_id = GameId(game.nanoid.clone());
            if let Some(entry) = self.game_response_cache.get(&game_id) {
                let (cached, cached_at) = entry.value();
                if cache_entry_is_valid(cached.updated_at, *cached_at, game.updated_at) {
                    return Ok(cached.clone());
                }
            }
            self.telemetry.inc_from_model();
            self.telemetry.inc_state_replay();
            let resp = Arc::new(crate::responses::GameResponse::from_model(game, conn).await?);
            self.game_response_cache.insert(game_id, (resp.clone(), Instant::now()));
            Ok(resp)
        }
    }

    #[cfg(test)]
    mod cache_tests {
        use super::*;

        #[test]
        fn cache_hit_when_fresh_and_matching_updated_at() {
            let t = Utc::now();
            assert!(cache_entry_is_valid(t, Instant::now(), t));
        }

        #[test]
        fn cache_miss_when_updated_at_differs() {
            let t = Utc::now();
            let stale = t - chrono::Duration::seconds(1);
            assert!(!cache_entry_is_valid(stale, Instant::now(), t));
        }

        #[test]
        fn cache_miss_when_ttl_expired() {
            let t = Utc::now();
            let expired = Instant::now()
                .checked_sub(GAME_RESPONSE_CACHE_TTL + Duration::from_secs(1))
                .unwrap_or_else(Instant::now);
            assert!(!cache_entry_is_valid(t, expired, t));
        }
    }

}}
