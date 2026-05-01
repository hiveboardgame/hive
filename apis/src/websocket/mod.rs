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
    pub use telemetry::{diff_and_format, read_proc_vm_bytes, DestKind, DisconnectReason, InFlightGuard, SendOutcome, TelemetrySnapshot, WsTelemetry};
    pub use tournament_game_start::TournamentGameStart;
    pub use ws_hub::WsHub;
    pub use messages::{InternalServerMessage, MessageDestination};

    use dashmap::DashMap;
    use shared_types::GameId;
    use std::sync::Arc;

    #[derive(Debug)]
    pub struct WebsocketData {
        pub chat_storage: Chats,
        pub game_start: TournamentGameStart,
        pub pings: Pings,
        pub lags: Lags,
        pub telemetry: Arc<WsTelemetry>,
        /// Per-game cache of the latest `GameResponse`. Keyed on `GameId`;
        /// freshness is checked against `GameResponse::updated_at` at the
        /// call site. Invalidated by `WsHub::on_game_finished`.
        pub game_response_cache: DashMap<GameId, Arc<crate::responses::GameResponse>>,
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
        /// Return a cached `GameResponse` if one exists with the same
        /// `updated_at` as the supplied game; otherwise build a fresh one
        /// (one full state replay), insert it, and return it.
        ///
        /// Per-game alloc churn for `GameResponse::from_model` was the
        /// largest single contributor to RSS HWM in production (a 32×32
        /// `BugStack` board + history parse + legal-move map per call).
        /// Sharing one `Arc<GameResponse>` per (game, updated_at) tuple
        /// across the broadcast fanout removes the bulk of it.
        pub async fn get_or_build_response(
            &self,
            game: &db_lib::models::Game,
            conn: &mut db_lib::DbConn<'_>,
        ) -> anyhow::Result<Arc<crate::responses::GameResponse>> {
            let game_id = GameId(game.nanoid.clone());
            if let Some(cached) = self.game_response_cache.get(&game_id) {
                if cached.updated_at == game.updated_at {
                    return Ok(cached.clone());
                }
            }
            self.telemetry.inc_from_model();
            self.telemetry.inc_state_replay();
            let resp = Arc::new(crate::responses::GameResponse::from_model(game, conn).await?);
            self.game_response_cache.insert(game_id, resp.clone());
            Ok(resp)
        }
    }

}}
