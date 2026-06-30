mod lag_tracking;
pub use lag_tracking::{Lags, Pings};
pub mod busybee;
pub mod client_handlers;

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
    mod lobby_snapshot;
    mod messages;
    mod start_conn;
    mod telemetry;
    mod tournament_game_start;
    mod ws_connection;
    mod ws_hub;
    pub mod server_handlers;

    pub use start_conn::start_connection;
    pub use telemetry::{diff_and_format, TelemetrySnapshot, WsTelemetry};
    pub(crate) use tournament_game_start::TournamentGameStart;
    pub use ws_hub::{WsHub, SOCKET_BUFFER_CAPACITY};
    pub use messages::{reaction_messages, GameFinalize, GameSubscription, InternalServerMessage, MessageDestination, Reaction};

    use crate::notifications::PendingNotifications;
    use chrono::{DateTime, Utc};
    use dashmap::DashMap;
    use shared_types::GameId;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::Notify;

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
        pub game_start: TournamentGameStart,
        pub pings: Pings,
        pub lags: Lags,
        pub telemetry: Arc<WsTelemetry>,
        /// Per-game cache: `(response, cached_at)`. Validated by both
        /// `updated_at` equality and a TTL to catch stale user-derived fields.
        pub game_response_cache: DashMap<GameId, (Arc<crate::responses::GameResponse>, Instant)>,
        /// Singleflight: while one task is rebuilding a `GameResponse` for
        /// `game_id`, concurrent misses park on this `Notify` instead of all
        /// dog-piling the DB. The builder removes its own entry and calls
        /// `notify_waiters` so parked tasks re-read the cache.
        game_response_inflight: DashMap<GameId, Arc<Notify>>,
        pub pending_notifications: Arc<PendingNotifications>,
    }

    impl Default for WebsocketData {
        fn default() -> Self {
            Self {
                game_start: TournamentGameStart::default(),
                pings: Pings::default(),
                lags: Lags::default(),
                telemetry: Arc::new(WsTelemetry::default()),
                game_response_cache: DashMap::new(),
                game_response_inflight: DashMap::new(),
                pending_notifications: Arc::new(PendingNotifications::default()),
            }
        }
    }

    impl WebsocketData {
        /// Return a cached `GameResponse` if fresh; otherwise build, cache, and return it.
        ///
        /// Singleflight semantics: concurrent callers on the same `GameId` share
        /// a single DB build. The first caller claims the slot via the
        /// `game_response_inflight` map; subsequent callers `notified().await`
        /// on the same `Notify` and re-read the cache when woken.
        pub async fn get_or_build_response(
            &self,
            game: &db_lib::models::Game,
            conn: &mut db_lib::DbConn<'_>,
        ) -> anyhow::Result<Arc<crate::responses::GameResponse>> {
            let game_id = GameId(game.nanoid.clone());

            loop {
                if let Some(entry) = self.game_response_cache.get(&game_id) {
                    let (cached, cached_at) = entry.value();
                    if cache_entry_is_valid(cached.updated_at, *cached_at, game.updated_at) {
                        return Ok(cached.clone());
                    }
                }

                // Try to claim the build slot atomically. If somebody else is
                // already building, hold the `Notify` and park; otherwise
                // insert our own `Notify`, fall through, and become the
                // builder.
                let waiter = {
                    let entry = self.game_response_inflight.entry(game_id.clone());
                    match entry {
                        dashmap::mapref::entry::Entry::Occupied(o) => {
                            // Create the wait future while the inflight entry
                            // is still observed. `notify_waiters()` only wakes
                            // futures created before the call, so returning
                            // the `Arc<Notify>` and calling `notified()` later
                            // can lose the builder's wakeup.
                            Some(o.get().clone().notified_owned())
                        }
                        dashmap::mapref::entry::Entry::Vacant(v) => {
                            v.insert(Arc::new(Notify::new()));
                            None
                        }
                    }
                };

                if let Some(waiter) = waiter {
                    // Someone else is rebuilding. Wait, then retry the cache
                    // read — if the builder succeeded the next pass hits the
                    // cache; if it failed we'll take the build slot ourselves.
                    waiter.await;
                    continue;
                }

                // We're the builder. Make sure the inflight slot is cleared
                // and waiters are released no matter how we exit this scope
                // (Ok, Err, or panic via unwind).
                let _guard = InflightGuard {
                    map: &self.game_response_inflight,
                    game_id: &game_id,
                };
                self.telemetry.inc_from_model();
                let resp = Arc::new(
                    crate::responses::GameResponse::from_model(game, conn).await?,
                );
                self.game_response_cache
                    .insert(game_id.clone(), (resp.clone(), Instant::now()));
                return Ok(resp);
            }
        }
    }

    /// RAII guard for the singleflight slot. Drop removes our entry from
    /// `game_response_inflight` and wakes every parked waiter so they retry
    /// the cache read. Panic-safe by virtue of running on the unwind path.
    struct InflightGuard<'a> {
        map: &'a DashMap<GameId, Arc<Notify>>,
        game_id: &'a GameId,
    }

    impl Drop for InflightGuard<'_> {
        fn drop(&mut self) {
            if let Some((_, notify)) = self.map.remove(self.game_id) {
                notify.notify_waiters();
            }
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

        #[tokio::test]
        async fn inflight_waiter_created_before_builder_notify_is_woken() {
            let map = DashMap::<GameId, Arc<Notify>>::new();
            let game_id = GameId("notify-race".to_string());
            map.insert(game_id.clone(), Arc::new(Notify::new()));

            let waiter = {
                match map.entry(game_id.clone()) {
                    dashmap::mapref::entry::Entry::Occupied(o) => {
                        o.get().clone().notified_owned()
                    }
                    dashmap::mapref::entry::Entry::Vacant(_) => {
                        panic!("test setup should have inserted inflight entry")
                    }
                }
            };

            let (_, notify) = map.remove(&game_id).expect("inflight entry exists");
            notify.notify_waiters();

            tokio::time::timeout(Duration::from_millis(50), waiter)
                .await
                .expect("waiter created before notify_waiters must be woken");
        }
    }

}}
