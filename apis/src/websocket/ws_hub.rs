use super::{
    messages::{MessageDestination, SocketTx},
    telemetry::{
        read_proc_vm_bytes,
        DestKind,
        InFlightGuard,
        QueuedGuard,
        SendOutcome,
        TelemetrySnapshot,
    },
    WebsocketData,
};
use crate::{
    common::{GameUpdate, ServerMessage, ServerResult, UserStatus, UserUpdate},
    notifications::{notify_game_ended, GameEndReason},
    responses::{HeartbeatResponse, UserResponse},
};
use bytes::Bytes;
use chrono::Utc;
use codee::{binary::MsgpackSerdeCodec, Encoder};
use dashmap::{DashMap, DashSet};
use db_lib::{
    get_conn,
    models::{Game, Tournament, User},
    DbConn,
    DbPool,
    DB_POOL_MAX_SIZE,
};
use hive_lib::GameStatus;
use log::error;
use rand::Rng;
use shared_types::{Conclusion, GameId, SimpleUser, TimeMode, TournamentId};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, Semaphore};
use uuid::Uuid;

pub const SOCKET_BUFFER_CAPACITY: usize = 128;
const LOBBY_GAME_ID: &str = "lobby";
/// Cap on concurrent `load_user_state` tasks. Sized so loaders cannot starve
/// the rest of the app of pool connections.
pub const LOAD_USER_STATE_CONCURRENCY: usize = DB_POOL_MAX_SIZE as usize / 2;
/// Minimum gap between consecutive `GameUpdate::Tv` broadcasts for the same
/// game. The TV view in the lobby is a UX feature, not a per-move feed.
const TV_THROTTLE: Duration = Duration::from_secs(1);
/// How long a cached tournament-membership snapshot stays fresh. Tournament
/// dispatch hits 3 DB queries per `InternalServerMessage` otherwise; busy
/// chat in a populated tournament lobby would amplify pool usage 3×. A newly
/// joined player may miss broadcasts for up to this window — acceptable
/// because chat replays on connect and tournament state is re-fetched on
/// next user action.
const TOURNAMENT_MEMBERS_TTL: Duration = Duration::from_secs(5);
/// Eviction age for `tournament_members`. An entry not refreshed for this
/// long is dropped — by definition nothing has read it within the window
/// (each read past the freshness TTL refreshes it). 60s is 12× the
/// freshness TTL, so a quiet tournament gets evicted while an active one
/// is permanently kept warm. Without this, every tournament ever
/// dispatched would persist for the lifetime of the process.
const TOURNAMENT_MEMBERS_MAX_AGE: Duration = Duration::from_secs(60);

/// Grace window before the heartbeat finalizes a Finished game. Within this
/// window the dispatcher's post-dispatch hook (HandlerOutput::finalize_games)
/// is expected to run finalization. The heartbeat covers orphan paths — a
/// handler that crashed mid-flight, an
/// admin DB tweak, etc. Measured against `game.updated_at`.
const FINISHED_GAME_GRACE: Duration = Duration::from_secs(5);

/// Maximum age for a `game_response_cache` entry before it gets swept by the
/// heartbeat. Read-time TTL handles correctness; this bound handles cleanup
/// for entries that never get re-read (e.g., abandoned games where neither
/// player returns).
const GAME_RESPONSE_CACHE_MAX_AGE: Duration = Duration::from_secs(300);

/// Per-socket minimum gap between accepted `ClientRequest::Resync` requests.
/// Browsers fire `visibilitychange` + `pageshow` in quick succession on wake;
/// without this each tab would do the full snapshot work twice. The client
/// also debounces, but a misbehaving or modified client must not be able to
/// drain the pool by spamming Resync.
const RESYNC_COOLDOWN: Duration = Duration::from_millis(500);

/// WsHub — concurrent, non-actor replacement for `WsServer`.
///
/// Shutdown: there is currently no graceful-shutdown signal; sessions are dropped
/// when the process exits. Revisit if/when we add `CancellationToken` plumbing.
pub struct WsHub {
    /// `user_id → (socket_id → Sender)`. Outer DashMap shards on user_id;
    /// inner DashMap shards on socket_id. We never hold outer write while holding
    /// any inner lock, which keeps the two-level locking deadlock-free.
    pub(in crate::websocket) sessions: DashMap<Uuid, DashMap<Uuid, mpsc::Sender<Bytes>>>,
    membership: RwLock<Membership>,
    pub(crate) data: Arc<WebsocketData>,
    pub(in crate::websocket) pool: DbPool,
    /// Bounds the number of concurrent `load_user_state` tasks so connect-burst
    /// can't blow up pool-connection usage or transient loader retention.
    loader_permits: Arc<Semaphore>,
    /// Last TV broadcast timestamp per game. Used by `should_send_tv` to
    /// coalesce per-move global fanout. Evicted on game finalization. Also
    /// doubles as the "currently on TV" set consumed by `send_lobby_snapshot`.
    pub(in crate::websocket) last_tv_broadcast: DashMap<GameId, Instant>,
    /// Cached tournament membership (players ∪ organizers). Refreshed lazily
    /// on dispatch when older than `TOURNAMENT_MEMBERS_TTL`. Bounds DB load
    /// from chatty tournament lobbies.
    tournament_members: DashMap<TournamentId, (Instant, Vec<Uuid>)>,
    /// Games whose rows were intentionally deleted by abort handlers before
    /// their final websocket fanout has completed. Heartbeat treats these like
    /// finished rows and gives the dispatcher grace to send the abort message.
    pending_deleted_games: DashMap<GameId, PendingDeletedGame>,
    /// Per-socket timestamp of the last accepted Resync. Used to enforce
    /// `RESYNC_COOLDOWN`. Entries are evicted on `on_disconnect`.
    last_resync: DashMap<Uuid, Instant>,
    /// Users whose accounts were deleted while websocket sessions were already
    /// open. Authentication is cached on each socket, so central auth checks
    /// consult this process-local set before accepting user actions.
    revoked_users: DashSet<Uuid>,
}

#[derive(Default)]
struct Membership {
    /// Sockets that receive live websocket fanout for a game surface. The
    /// lobby is represented here with `LOBBY_GAME_ID`.
    fanout: GameMembershipIndex,
    /// Sockets that keep an unfinished game eligible for timer heartbeat and
    /// finalization lifecycle work.
    heartbeat: GameMembershipIndex,
}

#[derive(Default)]
struct GameMembershipIndex {
    /// game → set of (user_id, socket_id) pairs subscribed to it.
    games_sockets: HashMap<GameId, HashSet<(Uuid, Uuid)>>,
    /// (user_id, socket_id) → set of games that socket is subscribed to.
    sockets_games: HashMap<(Uuid, Uuid), HashSet<GameId>>,
}

impl GameMembershipIndex {
    fn subscribe(&mut self, user_id: Uuid, socket_id: Uuid, game_id: &GameId) {
        self.games_sockets
            .entry(game_id.clone())
            .or_default()
            .insert((user_id, socket_id));
        self.sockets_games
            .entry((user_id, socket_id))
            .or_default()
            .insert(game_id.clone());
    }

    fn unsubscribe(&mut self, user_id: Uuid, socket_id: Uuid, game_id: &GameId) {
        let socket_pair = (user_id, socket_id);
        let prune_game = if let Some(sockets) = self.games_sockets.get_mut(game_id) {
            sockets.remove(&socket_pair);
            sockets.is_empty()
        } else {
            false
        };
        if prune_game {
            self.games_sockets.remove(game_id);
        }

        let prune_socket = if let Some(games) = self.sockets_games.get_mut(&socket_pair) {
            games.remove(game_id);
            games.is_empty()
        } else {
            false
        };
        if prune_socket {
            self.sockets_games.remove(&socket_pair);
        }
    }

    fn unsubscribe_socket(&mut self, socket_pair: (Uuid, Uuid)) {
        if let Some(games) = self.sockets_games.remove(&socket_pair) {
            for game_id in games {
                let prune_game = if let Some(sockets) = self.games_sockets.get_mut(&game_id) {
                    sockets.remove(&socket_pair);
                    sockets.is_empty()
                } else {
                    false
                };
                if prune_game {
                    self.games_sockets.remove(&game_id);
                }
            }
        }
    }

    fn evict_game(&mut self, game_id: &GameId) {
        if let Some(sockets) = self.games_sockets.remove(game_id) {
            for socket_pair in sockets {
                let prune_socket = if let Some(games) = self.sockets_games.get_mut(&socket_pair) {
                    games.remove(game_id);
                    games.is_empty()
                } else {
                    false
                };
                if prune_socket {
                    self.sockets_games.remove(&socket_pair);
                }
            }
        }
    }

    fn contains_game(&self, game_id: &GameId) -> bool {
        self.games_sockets.contains_key(game_id)
    }

    fn contains_socket(&self, user_id: Uuid, socket_id: Uuid, game_id: &GameId) -> bool {
        self.games_sockets
            .get(game_id)
            .is_some_and(|sockets| sockets.contains(&(user_id, socket_id)))
    }

    fn sockets_in_game(&self, game_id: &GameId) -> Vec<(Uuid, Uuid)> {
        self.games_sockets
            .get(game_id)
            .map(|sockets| sockets.iter().copied().collect())
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy)]
struct PendingDeletedGame {
    marked_at: Instant,
    white_id: Uuid,
    black_id: Uuid,
}

/// RAII scope guard for `mark_deleted_game_pending`. Constructing the guard
/// arms the marker; dropping it clears it. Callers `disarm()` after the DB
/// `delete` commits so `finalize_game` becomes the one to clear the marker.
/// Panic safety: a panic between `mark_deleted_game_pending` and the commit
/// would otherwise leave the marker hanging until the heartbeat sweeps it
/// `GAME_RESPONSE_CACHE_MAX_AGE` later — this guard collapses that window to
/// the stack-unwind path.
pub(crate) struct PendingDeletedGuard {
    hub: Arc<WsHub>,
    game_id: GameId,
    armed: bool,
}

impl PendingDeletedGuard {
    /// Mark the guard as having been superseded by a successful commit; the
    /// pending marker stays in place until `finalize_game` clears it.
    pub(crate) fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for PendingDeletedGuard {
    fn drop(&mut self) {
        if self.armed {
            self.hub.clear_deleted_game_pending(&self.game_id);
        }
    }
}

impl WsHub {
    pub fn new(data: Arc<WebsocketData>, pool: DbPool) -> Arc<Self> {
        Arc::new(Self {
            sessions: DashMap::new(),
            membership: RwLock::new(Membership::default()),
            data,
            pool,
            loader_permits: Arc::new(Semaphore::new(LOAD_USER_STATE_CONCURRENCY)),
            last_tv_broadcast: DashMap::new(),
            tournament_members: DashMap::new(),
            pending_deleted_games: DashMap::new(),
            last_resync: DashMap::new(),
            revoked_users: DashSet::new(),
        })
    }

    /// Atomically check the resync cooldown and stamp `now` on success. Returns
    /// true if the caller should proceed with a snapshot, false if the socket
    /// is within `RESYNC_COOLDOWN` of its last accepted resync. Used by
    /// `ResyncHandler`.
    pub(in crate::websocket) fn allow_resync(&self, socket_id: Uuid) -> bool {
        use dashmap::mapref::entry::Entry;
        let now = Instant::now();
        match self.last_resync.entry(socket_id) {
            Entry::Vacant(e) => {
                e.insert(now);
                true
            }
            Entry::Occupied(mut e) => {
                if now.duration_since(*e.get()) >= RESYNC_COOLDOWN {
                    *e.get_mut() = now;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn lobby() -> GameId {
        GameId(LOBBY_GAME_ID.to_string())
    }

    pub fn revoke_user(&self, user_id: Uuid) {
        self.revoked_users.insert(user_id);
    }

    pub fn is_user_revoked(&self, user_id: Uuid) -> bool {
        self.revoked_users.contains(&user_id)
    }

    // ─── connect / disconnect ─────────────────────────────────────────────────

    /// Synchronously register a new socket and trigger the async user-state load.
    /// Consumes a clone of the Arc so the spawned load task can keep `self` alive.
    pub fn on_connect(self: Arc<Self>, socket_id: Uuid, tx: mpsc::Sender<Bytes>, user: SimpleUser) {
        let user_id = user.user_id;
        let lobby = Self::lobby();
        // Lock order: membership write → outer DashMap shard. on_disconnect uses
        // the same order so the two operations can't deadlock and a fast
        // reconnect can't race a tail-end disconnect into a stale Offline.
        let is_first_socket = {
            let mut m = self.membership.write().unwrap_or_else(|p| p.into_inner());
            let was_empty = {
                let user_entry = self.sessions.entry(user_id).or_default();
                let empty = user_entry.is_empty();
                user_entry.insert(socket_id, tx);
                empty
            };
            m.fanout
                .games_sockets
                .entry(lobby.clone())
                .or_default()
                .insert((user_id, socket_id));
            m.fanout
                .sockets_games
                .entry((user_id, socket_id))
                .or_default()
                .insert(lobby.clone());
            was_empty
        };

        self.data.telemetry.inc_active_socket();
        if is_first_socket {
            self.data.telemetry.inc_active_user();
        }
        let broadcast_online = user.authed && is_first_socket;
        self.refresh_membership_gauges();

        // Async state load spawned independently so readers and dispatches
        // proceed immediately while state loads in the background.
        //
        // `loader_permits` caps concurrency. Tasks that can't get a permit
        // immediately are counted in `load_user_state_queued` until they do.
        // The sender is looked up from `sessions` after the permit is acquired
        // so disconnected sockets don't hold a channel open while tasks wait.
        let permits = self.loader_permits.clone();
        actix_rt::spawn(async move {
            let _queued = QueuedGuard::new(self.data.telemetry.clone());
            let permit = match permits.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => match permits.acquire_owned().await {
                    Ok(p) => p,
                    Err(_) => return,
                },
            };
            drop(_queued);
            if !self.is_socket_connected(user_id, socket_id) {
                return;
            }
            let Some(tx) = self
                .sessions
                .get(&user_id)
                .and_then(|socks| socks.get(&socket_id).map(|t| t.clone()))
            else {
                return;
            };
            let _guard = InFlightGuard::new(self.data.telemetry.clone());
            self.load_user_state(socket_id, user, tx, broadcast_online)
                .await;
            drop(permit);
        });
    }

    /// Drops a socket, cleaning up its per-socket game subscriptions immediately.
    /// If it was the user's last socket, also cleans up user-level state and
    /// broadcasts Offline to the lobby.
    pub fn on_disconnect(&self, socket_id: Uuid, user: SimpleUser) {
        let user_id = user.user_id;
        // Lock order matches on_connect (membership → sessions): a racing
        // on_connect observing was_empty=true while we're partway through
        // would otherwise leave the active_users gauge overcounted.
        let removed_user = {
            let mut m = self.membership.write().unwrap_or_else(|p| p.into_inner());

            // If the session entry is already gone (out-of-order cleanup),
            // still scrub membership — but skip dec_active_socket since
            // AtomicU64::fetch_sub(1) on 0 wraps to u64::MAX.
            let (inner_now_empty, socket_was_present) = match self.sessions.get(&user_id) {
                Some(sockets) => {
                    let removed = sockets.remove(&socket_id).is_some();
                    (sockets.is_empty(), removed)
                }
                None => (true, false),
            };

            if socket_was_present {
                self.data.telemetry.dec_active_socket();
            }

            self.last_resync.remove(&socket_id);

            let socket_pair = (user_id, socket_id);
            m.fanout.unsubscribe_socket(socket_pair);
            m.heartbeat.unsubscribe_socket(socket_pair);

            if inner_now_empty {
                let removed = self.sessions.remove_if(&user_id, |_, s| s.is_empty());
                if removed.is_some() {
                    self.data.telemetry.dec_active_user();
                    self.data.pings.remove(user_id);
                    true
                } else {
                    // A concurrent on_connect inserted a new socket between our
                    // sockets.remove and here — the inner map is non-empty again.
                    // User is alive; leave everything as-is.
                    false
                }
            } else {
                false
            }
        };

        self.refresh_membership_gauges();

        // Step 3: if this was the last socket and the user hasn't reconnected,
        // broadcast Offline. Re-check sessions after dropping the lock for the
        // same fast-reconnect reason: on_connect was queued behind our lock,
        // may now have run and broadcast Online — we suppress our Offline so the
        // reconnect's Online is the final word.
        if user.authed && removed_user && !self.sessions.contains_key(&user_id) {
            let message = ServerResult::Ok(Box::new(ServerMessage::UserStatus(UserUpdate {
                status: UserStatus::Offline,
                user: None,
                username: user.username,
            })));
            if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                self.fanout_lobby(&Bytes::from(serialized), DestKind::Global);
            }
        }
    }

    /// Build a `TelemetrySnapshot` enriched with sizes computed from
    /// `Membership`, `WebsocketData`, and `/proc/self/status`. Called once
    /// per `ws_telemetry` interval — locks are read-only and the iterations
    /// are O(N) over already-bounded structures.
    pub fn snapshot_with_state(&self) -> TelemetrySnapshot {
        let mut snap = self.data.telemetry.snapshot();

        // sessions
        let sessions_outer_len = self.sessions.len() as u64;
        let mut sessions_inner_total: u64 = 0;
        for entry in self.sessions.iter() {
            sessions_inner_total = sessions_inner_total.saturating_add(entry.value().len() as u64);
        }
        snap.sessions_outer_len = sessions_outer_len;
        snap.sessions_inner_total = sessions_inner_total;

        // membership
        {
            let m = self.membership.read().unwrap_or_else(|p| p.into_inner());
            snap.membership_games_sockets_len = m.fanout.games_sockets.len() as u64;
            snap.membership_sockets_games_len = m.fanout.sockets_games.len() as u64;
        }

        // lags
        if let Some(trackers) = self.data.lags.snapshot_len() {
            snap.lags_trackers_len = trackers as u64;
        }

        // tournament_game_start
        if let Ok(games_date) = self.data.game_start.games_date.read() {
            snap.game_start_games_date_len = games_date.len() as u64;
        }

        // caches
        snap.game_response_cache_len = self.data.game_response_cache.len() as u64;
        snap.last_tv_broadcast_len = self.last_tv_broadcast.len() as u64;

        // process VM
        let (rss, hwm) = read_proc_vm_bytes();
        snap.process_vm_rss_bytes = rss;
        snap.process_vm_hwm_bytes = hwm;

        snap
    }

    fn refresh_membership_gauges(&self) {
        let m = self.membership.read().unwrap_or_else(|p| p.into_inner());
        let lobby = Self::lobby();
        // Count game keys that aren't the lobby. Don't compute as `len() - 1`:
        // the lobby key may be absent (e.g. when no users are connected) and
        // saturating_sub would undercount real game memberships in that state.
        let active_games = m
            .fanout
            .games_sockets
            .keys()
            .filter(|gid| *gid != &lobby)
            .count() as u64;
        let lobby_count = m.fanout.games_sockets.get(&lobby).map_or(0, |s| s.len()) as u64;
        self.data.telemetry.set_active_games(active_games);
        self.data.telemetry.set_lobby_subscribers(lobby_count);
    }

    // ─── dispatch ─────────────────────────────────────────────────────────────

    pub async fn dispatch(
        &self,
        dest: &MessageDestination,
        bytes: Bytes,
        from: Option<(Uuid, Uuid)>, // (user_id, socket_id) of the sender
    ) {
        let dest_kind = DestKind::from(dest);
        self.data.telemetry.record_dispatch(dest_kind);

        match dest {
            MessageDestination::Direct(socket) => {
                self.send_via_tx(&socket.tx, DestKind::Direct, &bytes);
            }
            MessageDestination::User(user_id) => {
                self.send_to_user(user_id, DestKind::User, &bytes);
            }
            MessageDestination::Global => {
                if let Some((uid, sid)) = from {
                    self.subscribe_game_fanout(uid, sid, &Self::lobby());
                }
                self.fanout_lobby(&bytes, DestKind::Global);
            }
            MessageDestination::Game(game_id) => {
                if let Some((uid, sid)) = from {
                    self.subscribe_game_fanout(uid, sid, game_id);
                }
                let socket_pairs = self.sockets_in_game(game_id);
                for (uid, sid) in socket_pairs {
                    self.send_to_socket(&uid, &sid, DestKind::Game, &bytes);
                }
            }
            MessageDestination::GameSpectators(game_id, white_id, black_id) => {
                if let Some((uid, sid)) = from {
                    self.subscribe_game_fanout(uid, sid, game_id);
                }
                let socket_pairs: Vec<(Uuid, Uuid)> = {
                    let m = self.membership.read().unwrap_or_else(|p| p.into_inner());
                    m.fanout
                        .games_sockets
                        .get(game_id)
                        .map(|s| {
                            s.iter()
                                .filter(|(uid, _)| uid != white_id && uid != black_id)
                                .copied()
                                .collect()
                        })
                        .unwrap_or_default()
                };
                for (uid, sid) in socket_pairs {
                    self.send_to_socket(&uid, &sid, DestKind::GameSpectators, &bytes);
                }
            }
            MessageDestination::Tournament(tournament_id, echo_user_id) => {
                let mut user_ids = match self.tournament_members_cached(tournament_id).await {
                    Some(ids) => ids,
                    None if echo_user_id.is_some() => {
                        // Member-list rebuild failed (DB/connection error). The
                        // message is already persisted, so members will see it on
                        // their next history fetch — but live fanout is lost. Log
                        // it so this isn't a silent delivery hole.
                        log::warn!(
                            "tournament chat live fanout skipped for {tournament_id:?}: \
                             member list unavailable; delivering to sender only"
                        );
                        Vec::new()
                    }
                    None => return,
                };
                if let Some(echo_user_id) = echo_user_id {
                    if !user_ids.contains(echo_user_id) {
                        user_ids.push(*echo_user_id);
                    }
                }
                for uid in user_ids {
                    self.send_to_user(&uid, DestKind::Tournament, &bytes);
                }
            }
        }
    }

    /// Dispatch a `Reaction` to both players plus all spectators with a
    /// single msgpack serialization. `Bytes::clone` on the three fanouts is
    /// a refcount bump, so the wire payload is allocated exactly once per
    /// reaction — vs. three full clones + three serializations under the
    /// old `reaction_messages` path.
    pub async fn dispatch_reaction(
        &self,
        reaction: super::messages::Reaction,
        from: Option<(Uuid, Uuid)>,
    ) {
        let super::messages::Reaction {
            game_id,
            white_id,
            black_id,
            gar,
        } = reaction;
        let payload = ServerMessage::Game(Box::new(GameUpdate::Reaction(gar)));
        let result = ServerResult::Ok(Box::new(payload));
        let Ok(serialized) = MsgpackSerdeCodec::encode(&result) else {
            return;
        };
        let bytes = Bytes::from(serialized);
        // Send to both players (every tab gets the update) and to anyone
        // spectating who isn't a player. Bytes::clone is O(1).
        self.dispatch(&MessageDestination::User(white_id), bytes.clone(), from)
            .await;
        self.dispatch(&MessageDestination::User(black_id), bytes.clone(), from)
            .await;
        self.dispatch(
            &MessageDestination::GameSpectators(game_id, white_id, black_id),
            bytes,
            from,
        )
        .await;
    }

    /// Broadcasts TimedOut and finalizes a timed-out game. Shared by the sweeper
    /// (no-viewer) and heartbeat (active-viewer) paths so neither duplicates it.
    /// Loser is `current_player_id` — the side whose clock ran out.
    pub async fn broadcast_timeout_finalize(
        &self,
        conn: &mut DbConn<'_>,
        finalized: &Game,
    ) -> anyhow::Result<()> {
        let game_id = GameId(finalized.nanoid.clone());
        let game_response = self.data.get_or_build_response(finalized, conn).await?;
        let loser = User::find_by_uuid(&finalized.current_player_id, conn).await?;
        let reaction = super::messages::Reaction {
            game_id: game_id.clone(),
            white_id: finalized.white_id,
            black_id: finalized.black_id,
            gar: crate::common::GameActionResponse {
                game_action: crate::common::GameReaction::TimedOut,
                game: (*game_response).clone(),
                game_id: game_id.clone(),
                user_id: finalized.current_player_id,
                username: loser.username,
            },
        };
        self.dispatch_reaction(reaction, None).await;
        if game_response.time_mode == TimeMode::RealTime && self.should_send_tv(&game_id, true) {
            self.data.telemetry.inc_tv_broadcast();
            let payload = ServerMessage::Game(Box::new(GameUpdate::Tv((*game_response).clone())));
            let result = ServerResult::Ok(Box::new(payload));
            if let Ok(serialized) = MsgpackSerdeCodec::encode(&result) {
                self.dispatch(&MessageDestination::Global, Bytes::from(serialized), None)
                    .await;
            }
        }
        if let Err(e) = notify_game_ended(finalized, GameEndReason::Timeout, conn).await {
            error!("notify game ended (timeout) {}: {e}", finalized.nanoid);
        }
        self.finalize_game(&game_id, finalized.white_id, finalized.black_id);
        Ok(())
    }

    /// Drop a cached tournament-members entry. Call from join/leave/start/
    /// finish handlers when membership has changed — otherwise stale entries
    /// can serve up to `TOURNAMENT_MEMBERS_TTL` of incorrect fanout (e.g., a
    /// freshly-joined user missing tournament chat).
    pub fn invalidate_tournament_members(&self, tournament_id: &TournamentId) {
        self.tournament_members.remove(tournament_id);
    }

    /// Resolve a tournament's recipient set (players ∪ organizers), serving
    /// from cache when fresh. On a miss or stale entry, performs the 3 DB
    /// queries once and caches the result for `TOURNAMENT_MEMBERS_TTL`.
    /// Returns `None` only when the DB lookup fails entirely.
    async fn tournament_members_cached(&self, tournament_id: &TournamentId) -> Option<Vec<Uuid>> {
        // Refresh `cached_at` on hit so a busy tournament's entry serves
        // until invalidate_tournament_members runs. Without this, every
        // TOURNAMENT_MEMBERS_TTL window forces a fresh 3-query rebuild even
        // though membership is invalidated explicitly on join/leave/start/etc.
        if let Some(mut entry) = self.tournament_members.get_mut(tournament_id) {
            let (cached_at, ids) = entry.value_mut();
            if cached_at.elapsed() < TOURNAMENT_MEMBERS_TTL {
                let ids = ids.clone();
                *cached_at = Instant::now();
                return Some(ids);
            }
        }

        let mut conn = get_conn(&self.pool).await.ok()?;
        let tournament = Tournament::from_nanoid(&tournament_id.to_string(), &mut conn)
            .await
            .ok()?;
        let mut user_ids = HashSet::new();
        if let Ok(players) = tournament.players(&mut conn).await {
            for player in players {
                user_ids.insert(player.id);
            }
        }
        if let Ok(organizers) = tournament.organizers(&mut conn).await {
            for org in organizers {
                user_ids.insert(org.id);
            }
        }
        let ids: Vec<Uuid> = user_ids.into_iter().collect();
        self.tournament_members
            .insert(tournament_id.clone(), (Instant::now(), ids.clone()));
        Some(ids)
    }

    // ─── periodic jobs ────────────────────────────────────────────────────────

    pub fn ping_all(&self) {
        let mut rng = rand::rng();
        let user_ids: Vec<Uuid> = self.sessions.iter().map(|e| *e.key()).collect();
        for user_id in user_ids {
            let nonce = rng.random::<u64>();
            self.data.pings.set_nonce(user_id, nonce);
            let message = ServerResult::Ok(Box::new(ServerMessage::Ping {
                nonce,
                value: self.data.pings.value(user_id),
            }));
            if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                self.send_to_user(&user_id, DestKind::User, &Bytes::from(serialized));
            }
        }
    }

    pub async fn game_heartbeat(&self) {
        // Piggyback on the unconditional 3s heartbeat to sweep stale
        // tournament-member cache entries. Nothing else evicts them.
        // The retain is O(N) over already-bounded cached tournaments.
        self.tournament_members
            .retain(|_, (cached_at, _)| cached_at.elapsed() < TOURNAMENT_MEMBERS_MAX_AGE);

        // Sweep idle game-response cache entries. The read path validates
        // freshness (updated_at + TTL); this bound handles entries that
        // never get re-read (e.g., abandoned games where the player ghosts
        // before the row finalizes).
        self.data
            .game_response_cache
            .retain(|_, (_, cached_at)| cached_at.elapsed() < GAME_RESPONSE_CACHE_MAX_AGE);
        self.pending_deleted_games
            .retain(|_, pending| pending.marked_at.elapsed() < GAME_RESPONSE_CACHE_MAX_AGE);

        let games: Vec<(GameId, HashSet<(Uuid, Uuid)>)> = {
            let m = self.membership.read().unwrap_or_else(|p| p.into_inner());
            m.heartbeat
                .games_sockets
                .iter()
                .filter(|(gid, _)| gid.0.as_str() != LOBBY_GAME_ID)
                .map(|(gid, sockets)| (gid.clone(), sockets.clone()))
                .collect()
        };

        if games.is_empty() {
            return;
        }

        let Ok(mut conn) = get_conn(&self.pool).await else {
            return;
        };
        let game_ids: Vec<GameId> = games.iter().map(|(gid, _)| gid.clone()).collect();
        let fetched = match Game::find_by_nanoids(&game_ids, &mut conn).await {
            Ok(v) => v,
            Err(_) => return,
        };
        let mut by_nanoid: HashMap<GameId, Game> = fetched
            .into_iter()
            .map(|g| (GameId(g.nanoid.clone()), g))
            .collect();

        for (game_id, socket_pairs) in games {
            let Some(game) = by_nanoid.remove(&game_id) else {
                // Abort deletes the row before the final websocket message is
                // dispatched. If the abort handler marked this game as pending,
                // give the dispatcher the same grace normal finished rows get
                // before evicting membership.
                if let Some(pending) = self.pending_deleted_game(&game_id) {
                    if pending.marked_at.elapsed() < FINISHED_GAME_GRACE {
                        continue;
                    }
                    // Re-check membership: dispatcher's post-dispatch hook may
                    // have already finalized between our snapshot and now.
                    // Without this guard, finalize_game double-bumps
                    // games_finalized_total.
                    if self.is_game_in_heartbeat(&game_id) {
                        self.finalize_game(&game_id, pending.white_id, pending.black_id);
                    }
                } else {
                    // Legacy leaked membership or externally deleted row: no
                    // player ids remain, so only membership can be scrubbed here.
                    if self.is_game_in_heartbeat(&game_id) {
                        self.evict_game_heartbeat(&game_id);
                        self.evict_game_fanout(&game_id);
                        self.data.telemetry.inc_games_finalized();
                    }
                }
                continue;
            };
            if matches!(
                GameStatus::from_str(&game.game_status),
                Ok(GameStatus::Finished(_)) | Ok(GameStatus::Adjudicated)
            ) {
                // Safety-net path: normal finalization runs in the
                // dispatcher's post-message hook. The grace window prevents
                // a heartbeat tick from finalizing between the
                // game.finished=true commit and the dispatcher's hook,
                // which would drop the opponent from the final fanout.
                let age = (Utc::now() - game.updated_at)
                    .to_std()
                    .unwrap_or(Duration::ZERO);
                if age >= FINISHED_GAME_GRACE && self.is_game_in_heartbeat(&game_id) {
                    // Re-checking membership prevents the double-count race
                    // where the dispatcher's hook ran between our snapshot
                    // and this iteration's `finalize_game` call.
                    //
                    // If the heartbeat itself finalized this timeout, no
                    // dispatcher hook will broadcast it — players and spectators
                    // would see a frozen clock. Broadcast it here instead.
                    if game.conclusion == Conclusion::Timeout.to_string() {
                        if let Err(e) = self.broadcast_timeout_finalize(&mut conn, &game).await {
                            error!("game_heartbeat timeout broadcast {}: {e}", game.nanoid);
                        }
                        // Already finalized above; skip the finalize_game below.
                        continue;
                    }
                    self.finalize_game(&game_id, game.white_id, game.black_id);
                }
                continue;
            }
            if game.time_mode == TimeMode::Untimed.to_string() {
                // No timer to report, but keep heartbeat membership so the
                // finished-row safety net can still finalize orphan paths.
                continue;
            }
            if game.game_status == GameStatus::NotStarted.to_string() {
                // Timer hasn't started yet (real-time: waiting for black's first
                // move). Keep heartbeat membership so this game is picked up
                // once it starts or if it is finalized by another path.
                continue;
            }
            let Ok((id, white, black)) = game.get_heartbeat() else {
                continue;
            };
            let hb = HeartbeatResponse {
                game_id: id,
                white_time_left: white,
                black_time_left: black,
            };
            let message = ServerResult::Ok(Box::new(ServerMessage::Game(Box::new(
                GameUpdate::Heartbeat(hb),
            ))));
            let Ok(serialized) = MsgpackSerdeCodec::encode(&message) else {
                continue;
            };
            let bytes = Bytes::from(serialized);
            for (user_id, socket_id) in socket_pairs {
                self.send_to_socket(&user_id, &socket_id, DestKind::Game, &bytes);
            }
        }
    }

    // ─── private helpers ──────────────────────────────────────────────────────

    /// Try-send to one mpsc::Sender and record the outcome. Does NOT remove
    /// the socket on Full or Closed — the reader task is the single source
    /// of cleanup, and reaping here would orphan a still-live socket whose
    /// queue is just temporarily full. The bounded queue prevents OOM;
    /// the message itself is dropped on Full/Closed.
    fn send_via_tx(&self, tx: &mpsc::Sender<Bytes>, dest: DestKind, bytes: &Bytes) -> SendOutcome {
        let outcome = match tx.try_send(bytes.clone()) {
            Ok(_) => SendOutcome::Ok,
            Err(mpsc::error::TrySendError::Full(_)) => SendOutcome::Full,
            Err(mpsc::error::TrySendError::Closed(_)) => SendOutcome::Closed,
        };
        // Sample after send to capture post-enqueue depth.
        let used = SOCKET_BUFFER_CAPACITY.saturating_sub(tx.capacity());
        let charged = if matches!(outcome, SendOutcome::Ok) {
            bytes.len()
        } else {
            0
        };
        self.data
            .telemetry
            .record_send(dest, outcome, used, charged);
        outcome
    }

    pub(in crate::websocket) fn send_own_state_via_tx(
        &self,
        tx: &mpsc::Sender<Bytes>,
        bytes: &Bytes,
    ) {
        if !matches!(self.send_via_tx(tx, DestKind::User, bytes), SendOutcome::Ok) {
            self.data.telemetry.inc_own_state_drop();
        }
    }

    fn send_to_user(&self, user_id: &Uuid, dest: DestKind, bytes: &Bytes) {
        let Some(sockets) = self.sessions.get(user_id) else {
            return;
        };
        for entry in sockets.iter() {
            self.send_via_tx(entry.value(), dest, bytes);
        }
    }

    /// Send to one specific socket. Used for game-scoped dispatch where only the
    /// subscribed socket (not all of the user's tabs) should receive the message.
    fn send_to_socket(&self, user_id: &Uuid, socket_id: &Uuid, dest: DestKind, bytes: &Bytes) {
        let Some(sockets) = self.sessions.get(user_id) else {
            return;
        };
        let Some(tx) = sockets.get(socket_id) else {
            return;
        };
        self.send_via_tx(&tx, dest, bytes);
    }

    fn fanout_lobby(&self, bytes: &Bytes, dest: DestKind) {
        let socket_pairs = self.sockets_in_game(&Self::lobby());
        for (uid, sid) in socket_pairs {
            self.send_to_socket(&uid, &sid, dest, bytes);
        }
    }

    fn sockets_in_game(&self, game_id: &GameId) -> Vec<(Uuid, Uuid)> {
        let m = self.membership.read().unwrap_or_else(|p| p.into_inner());
        m.fanout.sockets_in_game(game_id)
    }

    pub fn subscribe_game_fanout(&self, user_id: Uuid, socket_id: Uuid, game_id: &GameId) {
        let already_member = {
            let m = self.membership.read().unwrap_or_else(|p| p.into_inner());
            m.fanout.contains_socket(user_id, socket_id, game_id)
        };
        if already_member {
            return;
        }
        {
            let mut m = self.membership.write().unwrap_or_else(|p| p.into_inner());
            m.fanout.subscribe(user_id, socket_id, game_id);
        }
        self.refresh_membership_gauges();
    }

    pub fn subscribe_game_heartbeat(&self, user_id: Uuid, socket_id: Uuid, game_id: &GameId) {
        if game_id.0.as_str() == LOBBY_GAME_ID {
            return;
        }
        let already_member = {
            let m = self.membership.read().unwrap_or_else(|p| p.into_inner());
            m.heartbeat.contains_socket(user_id, socket_id, game_id)
        };
        if already_member {
            return;
        }
        {
            let mut m = self.membership.write().unwrap_or_else(|p| p.into_inner());
            m.heartbeat.subscribe(user_id, socket_id, game_id);
        }
        self.refresh_membership_gauges();
    }

    /// Remove the specific socket from both fanout and heartbeat membership.
    pub fn unsubscribe_game(&self, user_id: Uuid, socket_id: Uuid, game_id: &GameId) {
        if game_id.0.as_str() == LOBBY_GAME_ID {
            return;
        }
        {
            let mut m = self.membership.write().unwrap_or_else(|p| p.into_inner());
            m.fanout.unsubscribe(user_id, socket_id, game_id);
            m.heartbeat.unsubscribe(user_id, socket_id, game_id);
        }
        self.refresh_membership_gauges();
    }

    /// True iff any socket currently keeps `game_id` eligible for heartbeat
    /// lifecycle work. Used to avoid double-counting finalization if the
    /// dispatcher ran between the heartbeat's snapshot and this loop iteration.
    fn is_game_in_heartbeat(&self, game_id: &GameId) -> bool {
        let m = self.membership.read().unwrap_or_else(|p| p.into_inner());
        m.heartbeat.contains_game(game_id)
    }

    /// Remove a game from heartbeat membership. Fanout membership remains so
    /// already-open finished-game tabs keep receiving post-game messages.
    pub fn evict_game_heartbeat(&self, game_id: &GameId) {
        {
            let mut m = self.membership.write().unwrap_or_else(|p| p.into_inner());
            m.heartbeat.evict_game(game_id);
        }
        self.refresh_membership_gauges();
    }

    /// Remove a game from fanout membership. This is reserved for deleted game
    /// surfaces where there is no durable post-game page to keep subscribed.
    pub fn evict_game_fanout(&self, game_id: &GameId) {
        {
            let mut m = self.membership.write().unwrap_or_else(|p| p.into_inner());
            m.fanout.evict_game(game_id);
        }
        self.refresh_membership_gauges();
    }

    pub(crate) fn mark_deleted_game_pending(
        &self,
        game_id: GameId,
        white_id: Uuid,
        black_id: Uuid,
    ) {
        self.pending_deleted_games.insert(
            game_id,
            PendingDeletedGame {
                marked_at: Instant::now(),
                white_id,
                black_id,
            },
        );
    }

    pub(crate) fn clear_deleted_game_pending(&self, game_id: &GameId) {
        self.pending_deleted_games.remove(game_id);
    }

    /// Build a panic-safe scope guard around `mark_deleted_game_pending`.
    /// Drop clears the marker by default; call `disarm()` after the delete
    /// commits so `finalize_game` is the one to clear it.
    pub(crate) fn arm_pending_delete(
        self: &Arc<Self>,
        game_id: GameId,
        white_id: Uuid,
        black_id: Uuid,
    ) -> PendingDeletedGuard {
        self.mark_deleted_game_pending(game_id.clone(), white_id, black_id);
        PendingDeletedGuard {
            hub: Arc::clone(self),
            game_id,
            armed: true,
        }
    }

    fn pending_deleted_game(&self, game_id: &GameId) -> Option<PendingDeletedGame> {
        self.pending_deleted_games
            .get(game_id)
            .map(|pending| *pending.value())
    }

    /// Clean up all per-game state keyed on `GameId` except membership.
    ///
    /// Handlers return `GameFinalize` values alongside their message list;
    /// `ws_connection` dispatches first, then calls `finalize_game`.
    /// Membership eviction must NOT happen here because dispatch(Game) reads
    /// fanout membership to find subscribers. `finalize_game` is the wrapper
    /// that runs this + heartbeat eviction + counter.
    pub fn on_game_finished(&self, game_id: &GameId, white_id: Uuid, black_id: Uuid) {
        self.data
            .lags
            .remove_pair(white_id, black_id, game_id.clone());

        // Game chat is preserved intentionally: post-game chat history must
        // remain accessible after finalization (e.g. post-mortem discussion).
        // Durable history is in Postgres; the recent-message cache is bounded
        // in the chat storage layer.

        if let Ok(mut games_date) = self.data.game_start.games_date.write() {
            games_date.remove(game_id);
        }

        self.data.game_response_cache.remove(game_id);
        self.last_tv_broadcast.remove(game_id);

        // games_finalized_total is bumped by `finalize_game`, not here, so
        // tests that exercise on_game_finished standalone don't double-count.
    }

    /// Full game-finalization ritual for durable finished games: per-game state
    /// cleanup, heartbeat eviction, and the finalized-game counter. Fanout
    /// membership stays in place so already-open finished-game tabs keep
    /// receiving spectator chat and other game-surface messages.
    pub fn finalize_game(&self, game_id: &GameId, white_id: Uuid, black_id: Uuid) {
        if self.pending_deleted_game(game_id).is_some() {
            self.finalize_deleted_game(game_id, white_id, black_id);
            return;
        }
        self.clear_deleted_game_pending(game_id);
        self.on_game_finished(game_id, white_id, black_id);
        self.evict_game_heartbeat(game_id);
        self.data.telemetry.inc_games_finalized();
    }

    /// Finalization for games whose DB row has been deleted, such as aborts.
    /// There is no durable game surface to keep alive, so both membership
    /// indexes are evicted after the final websocket fanout has been sent.
    fn finalize_deleted_game(&self, game_id: &GameId, white_id: Uuid, black_id: Uuid) {
        self.clear_deleted_game_pending(game_id);
        self.on_game_finished(game_id, white_id, black_id);
        self.evict_game_heartbeat(game_id);
        self.evict_game_fanout(game_id);
        self.data.telemetry.inc_games_finalized();
    }

    /// Returns true and stamps the game iff a TV broadcast should go out now.
    /// When `is_final` is true (game just finished), always returns true and
    /// clears the throttle entry — the final lobby update must never be dropped.
    /// Otherwise coalesces to at most once per `TV_THROTTLE`.
    pub fn should_send_tv(&self, game_id: &GameId, is_final: bool) -> bool {
        if is_final {
            self.last_tv_broadcast.remove(game_id);
            return true;
        }
        let now = Instant::now();
        match self.last_tv_broadcast.entry(game_id.clone()) {
            dashmap::mapref::entry::Entry::Occupied(mut e) => {
                if now.duration_since(*e.get()) >= TV_THROTTLE {
                    *e.get_mut() = now;
                    true
                } else {
                    false
                }
            }
            dashmap::mapref::entry::Entry::Vacant(e) => {
                e.insert(now);
                true
            }
        }
    }

    /// Returns true iff the specific socket we were spawned for is still in
    /// `sessions`. Used by `load_user_state` to bail when a fast disconnect
    /// (or disconnect+reconnect with a different socket_id) raced our DB load.
    pub(in crate::websocket) fn is_socket_connected(&self, user_id: Uuid, socket_id: Uuid) -> bool {
        self.sessions
            .get(&user_id)
            .is_some_and(|sockets| sockets.contains_key(&socket_id))
    }

    // ─── test helpers ─────────────────────────────────────────────────────────

    /// Register a socket directly without triggering the async state load.
    /// Returns the receiver for the socket's outbound channel.
    #[cfg(test)]
    fn register_socket(&self, user_id: Uuid, socket_id: Uuid) -> mpsc::Receiver<Bytes> {
        let (tx, rx) = mpsc::channel(SOCKET_BUFFER_CAPACITY);
        let lobby = Self::lobby();
        let mut m = self.membership.write().unwrap_or_else(|p| p.into_inner());
        self.sessions
            .entry(user_id)
            .or_default()
            .insert(socket_id, tx);
        m.fanout
            .games_sockets
            .entry(lobby.clone())
            .or_default()
            .insert((user_id, socket_id));
        m.fanout
            .sockets_games
            .entry((user_id, socket_id))
            .or_default()
            .insert(lobby);
        rx
    }

    /// Subscribe an already-registered socket to game fanout.
    #[cfg(test)]
    fn join_game_fanout(&self, user_id: Uuid, socket_id: Uuid, game_id: &GameId) {
        self.subscribe_game_fanout(user_id, socket_id, game_id);
    }

    #[cfg(test)]
    fn join_game_heartbeat(&self, user_id: Uuid, socket_id: Uuid, game_id: &GameId) {
        self.subscribe_game_heartbeat(user_id, socket_id, game_id);
    }

    #[cfg(test)]
    fn has_game_fanout(&self, user_id: Uuid, socket_id: Uuid, game_id: &GameId) -> bool {
        let m = self.membership.read().unwrap_or_else(|p| p.into_inner());
        m.fanout.contains_socket(user_id, socket_id, game_id)
    }

    #[cfg(test)]
    fn has_game_heartbeat(&self, user_id: Uuid, socket_id: Uuid, game_id: &GameId) -> bool {
        let m = self.membership.read().unwrap_or_else(|p| p.into_inner());
        m.heartbeat.contains_socket(user_id, socket_id, game_id)
    }

    #[cfg(test)]
    fn heartbeat_game_ids(&self) -> Vec<GameId> {
        let m = self.membership.read().unwrap_or_else(|p| p.into_inner());
        m.heartbeat.games_sockets.keys().cloned().collect()
    }

    // ─── user-state load (the long async block from Handler<Connect>) ──────────

    async fn load_user_state(
        &self,
        socket_id: Uuid,
        user: SimpleUser,
        tx: mpsc::Sender<Bytes>,
        broadcast_online: bool,
    ) {
        let user_id = user.user_id;
        let Ok(mut conn) = get_conn(&self.pool).await else {
            return;
        };

        // Bail if the connection already went away — no point loading state
        // for a socket that's gone, and the lobby Online broadcast below would
        // otherwise advertise a user that on_disconnect already announced
        // Offline. Re-checked before each user-visible broadcast since each
        // await is a chance for the disconnect to race in.
        if !self.is_socket_connected(user_id, socket_id) {
            return;
        }

        // The only connect-time side effect that doesn't belong in the resync
        // snapshot is the Online broadcast — the snapshot owns everything
        // else (invitations, schedules, urgent games, challenges, TV, roster).
        let user_model = if user.authed {
            match User::find_active_by_uuid(&user_id, &mut conn).await {
                Ok(user_model) => {
                    if broadcast_online {
                        // Re-check before the Online broadcast: the slow DB lookup gives
                        // plenty of time for a disconnect to race in. If it has, our Online
                        // would override the Offline that on_disconnect already sent and
                        // leave the lobby ghosting a user that's actually gone.
                        if !self.is_socket_connected(user_id, socket_id) {
                            return;
                        }
                        if let Ok(user_response) =
                            UserResponse::from_model(&user_model, &mut conn).await
                        {
                            // Re-check after the await: from_model is a DB round-trip during
                            // which on_disconnect may have run and broadcast Offline. Without
                            // this guard, fanout_lobby would overwrite that Offline with a
                            // stale Online.
                            if !self.is_socket_connected(user_id, socket_id) {
                                return;
                            }
                            let message =
                                ServerResult::Ok(Box::new(ServerMessage::UserStatus(UserUpdate {
                                    status: UserStatus::Online,
                                    user: Some(user_response),
                                    username: user.username.clone(),
                                })));
                            if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                                self.fanout_lobby(&Bytes::from(serialized), DestKind::Global);
                            }
                        }
                    }
                    Some(user_model)
                }
                Err(e) => {
                    error!("Failed to load authenticated websocket user {user_id}: {e}");
                    return;
                }
            }
        } else {
            None
        };

        let socket = SocketTx { socket_id, tx };
        self.send_lobby_snapshot(&mut conn, user_id, &socket, user_model.as_ref())
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::messages::{GameFinalize, SocketTx},
        *,
    };
    use crate::websocket::WsTelemetry;

    async fn make_hub() -> Arc<WsHub> {
        // bb8 builds the pool struct without making DB connections (min_idle = 0 by default).
        // Non-Tournament dispatch arms never call get_conn, so the unreachable host is fine.
        let pool = db_lib::get_pool("postgresql://test:test@127.0.0.1:9/test")
            .await
            .expect("bb8 pool builds without connecting");
        WsHub::new(Arc::new(WebsocketData::default()), pool)
    }

    #[tokio::test]
    async fn dispatch_user_routes_to_all_sockets() {
        let hub = make_hub().await;
        let uid = Uuid::new_v4();
        let sid_a = Uuid::new_v4();
        let sid_b = Uuid::new_v4();
        let mut rx_a = hub.register_socket(uid, sid_a);
        let mut rx_b = hub.register_socket(uid, sid_b);

        hub.dispatch(
            &MessageDestination::User(uid),
            Bytes::from_static(b"hi"),
            None,
        )
        .await;

        assert_eq!(rx_a.recv().await.unwrap(), Bytes::from_static(b"hi"));
        assert_eq!(rx_b.recv().await.unwrap(), Bytes::from_static(b"hi"));
    }

    #[tokio::test]
    async fn dispatch_user_unknown_is_noop() {
        let hub = make_hub().await;
        hub.dispatch(
            &MessageDestination::User(Uuid::new_v4()),
            Bytes::from_static(b"x"),
            None,
        )
        .await;
    }

    #[tokio::test]
    async fn revoke_user_marks_user_revoked() {
        let hub = make_hub().await;
        let uid = Uuid::new_v4();
        let other = Uuid::new_v4();

        assert!(!hub.is_user_revoked(uid));
        hub.revoke_user(uid);

        assert!(hub.is_user_revoked(uid));
        assert!(!hub.is_user_revoked(other));
    }

    #[tokio::test]
    async fn dispatch_direct_routes_only_to_target_socket() {
        let hub = make_hub().await;
        let uid = Uuid::new_v4();
        let sid = Uuid::new_v4();
        let mut bystander_rx = hub.register_socket(uid, sid);

        let (target_tx, mut target_rx) = mpsc::channel(8);
        let socket_tx = SocketTx {
            socket_id: Uuid::new_v4(),
            tx: target_tx,
        };

        hub.dispatch(
            &MessageDestination::Direct(socket_tx),
            Bytes::from_static(b"dm"),
            None,
        )
        .await;

        assert_eq!(target_rx.try_recv().unwrap(), Bytes::from_static(b"dm"));
        assert!(bystander_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn dispatch_game_routes_to_subscribed_sockets_only() {
        let hub = make_hub().await;
        let uid_a = Uuid::new_v4();
        let sid_a = Uuid::new_v4();
        let uid_b = Uuid::new_v4();
        let sid_b = Uuid::new_v4();
        let uid_c = Uuid::new_v4();
        let sid_c = Uuid::new_v4();

        let game_id = GameId("test-game".to_string());

        let mut rx_a = hub.register_socket(uid_a, sid_a);
        let mut rx_b = hub.register_socket(uid_b, sid_b);
        let mut rx_c = hub.register_socket(uid_c, sid_c); // lobby only, not in game

        hub.join_game_fanout(uid_a, sid_a, &game_id);
        hub.join_game_fanout(uid_b, sid_b, &game_id);

        hub.dispatch(
            &MessageDestination::Game(game_id),
            Bytes::from_static(b"move"),
            None,
        )
        .await;

        assert_eq!(rx_a.try_recv().unwrap(), Bytes::from_static(b"move"));
        assert_eq!(rx_b.try_recv().unwrap(), Bytes::from_static(b"move"));
        assert!(rx_c.try_recv().is_err());
    }

    #[tokio::test]
    async fn dispatch_game_from_implicitly_subscribes_sender() {
        let hub = make_hub().await;
        let uid = Uuid::new_v4();
        let sid = Uuid::new_v4();
        let game_id = GameId("implicit-game".to_string());

        let mut rx = hub.register_socket(uid, sid);

        // First dispatch with from= subscribes (uid, sid) to the game.
        hub.dispatch(
            &MessageDestination::Game(game_id.clone()),
            Bytes::from_static(b"a"),
            Some((uid, sid)),
        )
        .await;
        // Sender is now a member, so it receives subsequent dispatches too.
        hub.dispatch(
            &MessageDestination::Game(game_id),
            Bytes::from_static(b"b"),
            None,
        )
        .await;

        assert_eq!(rx.try_recv().unwrap(), Bytes::from_static(b"a"));
        assert_eq!(rx.try_recv().unwrap(), Bytes::from_static(b"b"));
        assert!(!hub.has_game_heartbeat(uid, sid, &GameId("implicit-game".to_string())));
    }

    #[tokio::test]
    async fn dispatch_global_reaches_all_lobby_sockets() {
        let hub = make_hub().await;
        let uid_a = Uuid::new_v4();
        let sid_a = Uuid::new_v4();
        let uid_b = Uuid::new_v4();
        let sid_b = Uuid::new_v4();

        let mut rx_a = hub.register_socket(uid_a, sid_a);
        let mut rx_b = hub.register_socket(uid_b, sid_b);

        hub.dispatch(
            &MessageDestination::Global,
            Bytes::from_static(b"broadcast"),
            None,
        )
        .await;

        assert_eq!(rx_a.try_recv().unwrap(), Bytes::from_static(b"broadcast"));
        assert_eq!(rx_b.try_recv().unwrap(), Bytes::from_static(b"broadcast"));
    }

    #[tokio::test]
    async fn dispatch_tournament_echo_reaches_non_member_sender() {
        let hub = make_hub().await;
        let sender_id = Uuid::new_v4();
        let sender_sid = Uuid::new_v4();
        let tournament_id = TournamentId("admin-echo".to_string());
        let mut sender_rx = hub.register_socket(sender_id, sender_sid);

        hub.tournament_members
            .insert(tournament_id.clone(), (Instant::now(), Vec::new()));

        hub.dispatch(
            &MessageDestination::Tournament(tournament_id, Some(sender_id)),
            Bytes::from_static(b"chat"),
            None,
        )
        .await;

        assert_eq!(sender_rx.try_recv().unwrap(), Bytes::from_static(b"chat"));
    }

    #[tokio::test]
    async fn dispatch_tournament_echo_dedupes_member_sender() {
        let hub = make_hub().await;
        let sender_id = Uuid::new_v4();
        let sender_sid = Uuid::new_v4();
        let tournament_id = TournamentId("member-echo".to_string());
        let mut sender_rx = hub.register_socket(sender_id, sender_sid);

        hub.tournament_members
            .insert(tournament_id.clone(), (Instant::now(), vec![sender_id]));

        hub.dispatch(
            &MessageDestination::Tournament(tournament_id, Some(sender_id)),
            Bytes::from_static(b"chat"),
            None,
        )
        .await;

        assert_eq!(sender_rx.try_recv().unwrap(), Bytes::from_static(b"chat"));
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn dispatch_game_spectators_excludes_players() {
        let hub = make_hub().await;
        let white_id = Uuid::new_v4();
        let white_sid = Uuid::new_v4();
        let black_id = Uuid::new_v4();
        let black_sid = Uuid::new_v4();
        let spec_id = Uuid::new_v4();
        let spec_sid = Uuid::new_v4();

        let game_id = GameId("spectator-game".to_string());

        let mut rx_white = hub.register_socket(white_id, white_sid);
        let mut rx_black = hub.register_socket(black_id, black_sid);
        let mut rx_spec = hub.register_socket(spec_id, spec_sid);

        hub.join_game_fanout(white_id, white_sid, &game_id);
        hub.join_game_fanout(black_id, black_sid, &game_id);
        hub.join_game_fanout(spec_id, spec_sid, &game_id);

        hub.dispatch(
            &MessageDestination::GameSpectators(game_id, white_id, black_id),
            Bytes::from_static(b"spec-msg"),
            None,
        )
        .await;

        assert!(rx_white.try_recv().is_err());
        assert!(rx_black.try_recv().is_err());
        assert_eq!(rx_spec.try_recv().unwrap(), Bytes::from_static(b"spec-msg"));
    }

    #[tokio::test]
    async fn unsubscribe_game_removes_socket_from_fanout_and_heartbeat() {
        let hub = make_hub().await;
        let uid = Uuid::new_v4();
        let sid = Uuid::new_v4();
        let game_id = GameId("leave-game".to_string());

        let mut rx = hub.register_socket(uid, sid);
        hub.join_game_fanout(uid, sid, &game_id);
        hub.join_game_heartbeat(uid, sid, &game_id);
        hub.unsubscribe_game(uid, sid, &game_id);

        assert!(!hub.has_game_fanout(uid, sid, &game_id));
        assert!(!hub.has_game_heartbeat(uid, sid, &game_id));

        hub.dispatch(
            &MessageDestination::Game(game_id),
            Bytes::from_static(b"after-leave"),
            None,
        )
        .await;

        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn finalize_game_evicts_heartbeat_but_preserves_fanout() {
        let hub = make_hub().await;
        let uid = Uuid::new_v4();
        let sid = Uuid::new_v4();
        let white_id = Uuid::new_v4();
        let black_id = Uuid::new_v4();
        let game_id = GameId("finish-keeps-fanout".to_string());

        let mut rx = hub.register_socket(uid, sid);
        hub.join_game_fanout(uid, sid, &game_id);
        hub.join_game_heartbeat(uid, sid, &game_id);

        hub.finalize_game(&game_id, white_id, black_id);

        assert!(hub.has_game_fanout(uid, sid, &game_id));
        assert!(!hub.has_game_heartbeat(uid, sid, &game_id));

        hub.dispatch(
            &MessageDestination::Game(game_id),
            Bytes::from_static(b"post-game-update"),
            None,
        )
        .await;

        assert_eq!(
            rx.try_recv().unwrap(),
            Bytes::from_static(b"post-game-update"),
            "finished-game fanout must survive finalization",
        );
    }

    #[tokio::test]
    async fn spectator_chat_after_finalization_reaches_existing_spectators() {
        let hub = make_hub().await;
        let white_id = Uuid::new_v4();
        let black_id = Uuid::new_v4();
        let spec_a = Uuid::new_v4();
        let spec_a_sid = Uuid::new_v4();
        let spec_b = Uuid::new_v4();
        let spec_b_sid = Uuid::new_v4();
        let spec_c = Uuid::new_v4();
        let spec_c_sid = Uuid::new_v4();
        let game_id = GameId("post-game-chat".to_string());

        let mut rx_a = hub.register_socket(spec_a, spec_a_sid);
        let mut rx_b = hub.register_socket(spec_b, spec_b_sid);
        let mut rx_c = hub.register_socket(spec_c, spec_c_sid);
        for (uid, sid) in [
            (spec_a, spec_a_sid),
            (spec_b, spec_b_sid),
            (spec_c, spec_c_sid),
        ] {
            hub.join_game_fanout(uid, sid, &game_id);
            hub.join_game_heartbeat(uid, sid, &game_id);
        }

        hub.finalize_game(&game_id, white_id, black_id);

        hub.dispatch(
            &MessageDestination::GameSpectators(game_id, white_id, black_id),
            Bytes::from_static(b"post-game-chat"),
            Some((spec_a, spec_a_sid)),
        )
        .await;

        assert_eq!(
            rx_a.try_recv().unwrap(),
            Bytes::from_static(b"post-game-chat")
        );
        assert_eq!(
            rx_b.try_recv().unwrap(),
            Bytes::from_static(b"post-game-chat")
        );
        assert_eq!(
            rx_c.try_recv().unwrap(),
            Bytes::from_static(b"post-game-chat")
        );
    }

    #[tokio::test]
    async fn heartbeat_membership_ignores_fanout_only_subscriptions() {
        let hub = make_hub().await;
        let uid = Uuid::new_v4();
        let sid = Uuid::new_v4();
        let game_id = GameId("fanout-only".to_string());

        hub.register_socket(uid, sid);
        hub.join_game_fanout(uid, sid, &game_id);

        assert!(!hub.heartbeat_game_ids().contains(&game_id));

        hub.join_game_heartbeat(uid, sid, &game_id);

        assert!(hub.heartbeat_game_ids().contains(&game_id));
    }

    #[tokio::test]
    async fn pending_deleted_game_finalization_removes_fanout_and_heartbeat() {
        let hub = make_hub().await;
        let uid = Uuid::new_v4();
        let sid = Uuid::new_v4();
        let white_id = Uuid::new_v4();
        let black_id = Uuid::new_v4();
        let game_id = GameId("deleted-cleanup".to_string());

        let mut rx = hub.register_socket(uid, sid);
        hub.join_game_fanout(uid, sid, &game_id);
        hub.join_game_heartbeat(uid, sid, &game_id);
        hub.mark_deleted_game_pending(game_id.clone(), white_id, black_id);

        hub.finalize_game(&game_id, white_id, black_id);

        assert!(hub.pending_deleted_game(&game_id).is_none());
        assert!(!hub.has_game_fanout(uid, sid, &game_id));
        assert!(!hub.has_game_heartbeat(uid, sid, &game_id));

        hub.dispatch(
            &MessageDestination::Game(game_id),
            Bytes::from_static(b"deleted"),
            None,
        )
        .await;

        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn finalize_game_clears_pending_deleted_game_marker() {
        let hub = make_hub().await;
        let game_id = GameId("deleted-game".to_string());
        let white_id = Uuid::new_v4();
        let black_id = Uuid::new_v4();

        hub.mark_deleted_game_pending(game_id.clone(), white_id, black_id);
        assert!(hub.pending_deleted_game(&game_id).is_some());

        hub.finalize_game(&game_id, white_id, black_id);

        assert!(hub.pending_deleted_game(&game_id).is_none());
    }

    #[test]
    fn game_finalize_builds_user_scoped_removal_messages() {
        let game_id = GameId("removed-game".to_string());
        let white_id = Uuid::new_v4();
        let black_id = Uuid::new_v4();
        let finalize = GameFinalize {
            game_id: game_id.clone(),
            white_id,
            black_id,
        };

        let messages = finalize.own_game_removed_messages();

        assert_eq!(messages.len(), 2);
        assert!(messages.iter().any(|message| {
            matches!(&message.destination, MessageDestination::User(id) if *id == white_id)
                && matches!(
                    &message.message,
                    ServerMessage::Game(update)
                        if matches!(update.as_ref(), GameUpdate::OwnGameRemoved(id) if id == &game_id)
                )
        }));
        assert!(messages.iter().any(|message| {
            matches!(&message.destination, MessageDestination::User(id) if *id == black_id)
                && matches!(
                    &message.message,
                    ServerMessage::Game(update)
                        if matches!(update.as_ref(), GameUpdate::OwnGameRemoved(id) if id == &game_id)
                )
        }));
    }

    // Regression: on_game_finished must not evict membership. Handler
    // finalizers run after dispatch; if this helper evicted membership itself,
    // any caller that used it before final-message dispatch would drop the
    // opponent from the final move/control fanout.
    #[tokio::test]
    async fn game_update_reaches_opponent_after_on_game_finished() {
        let hub = make_hub().await;
        let white_id = Uuid::new_v4();
        let white_sid = Uuid::new_v4();
        let black_id = Uuid::new_v4();
        let black_sid = Uuid::new_v4();
        let game_id = GameId("finish-game".to_string());

        let mut rx_white = hub.register_socket(white_id, white_sid);
        let mut rx_black = hub.register_socket(black_id, black_sid);
        hub.join_game_fanout(white_id, white_sid, &game_id);
        hub.join_game_fanout(black_id, black_sid, &game_id);

        // Simulate the cleanup half independently: on_game_finished can run
        // before dispatch only if it leaves membership intact.
        hub.on_game_finished(&game_id, white_id, black_id);
        hub.dispatch(
            &MessageDestination::Game(game_id),
            Bytes::from_static(b"final-move"),
            Some((white_id, white_sid)),
        )
        .await;

        assert_eq!(
            rx_white.try_recv().unwrap(),
            Bytes::from_static(b"final-move")
        );
        assert_eq!(
            rx_black.try_recv().unwrap(),
            Bytes::from_static(b"final-move"),
            "opponent must receive the final game update",
        );
    }

    // Regression: the final TV update for a finished realtime game must not be
    // suppressed by the throttle. Clients use it to remove the game from the
    // live-games lobby list; dropping it leaves stale entries in the UI.
    #[tokio::test]
    async fn final_tv_update_bypasses_throttle() {
        let hub = make_hub().await;
        let game_id = GameId("tv-final-game".to_string());

        // Prime the throttle.
        assert!(hub.should_send_tv(&game_id, false));
        // Within the throttle window a non-final update is suppressed.
        assert!(!hub.should_send_tv(&game_id, false));
        // The final update must always go through.
        assert!(
            hub.should_send_tv(&game_id, true),
            "final TV update must not be suppressed by the throttle",
        );
    }

    #[tokio::test]
    async fn non_final_tv_updates_are_throttled() {
        let hub = make_hub().await;
        let game_id = GameId("tv-throttle-game".to_string());

        assert!(hub.should_send_tv(&game_id, false));
        assert!(!hub.should_send_tv(&game_id, false));
        assert!(!hub.should_send_tv(&game_id, false));
    }

    /// `visibilitychange` + `pageshow` fire in close succession on wake; the
    /// server-side cooldown stops a second snapshot from running for the same
    /// socket. Per-socket so multi-tab users still get fresh data for the tab
    /// that actually woke.
    #[tokio::test]
    async fn allow_resync_rate_limits_per_socket() {
        let hub = make_hub().await;
        let sid_a = Uuid::new_v4();
        let sid_b = Uuid::new_v4();

        assert!(hub.allow_resync(sid_a), "first resync for socket A allowed");
        assert!(
            !hub.allow_resync(sid_a),
            "second resync within cooldown blocked"
        );
        assert!(
            hub.allow_resync(sid_b),
            "different socket not rate-limited by socket A"
        );
    }

    /// Disconnect must evict the socket's resync stamp so a fast disconnect
    /// then reconnect with the same socket_id (extremely unlikely but legal)
    /// doesn't carry over a stale cooldown.
    #[tokio::test]
    async fn allow_resync_evicts_on_disconnect() {
        let hub = make_hub().await;
        let uid = Uuid::new_v4();
        let sid = Uuid::new_v4();
        let _rx = hub.register_socket(uid, sid);

        assert!(hub.allow_resync(sid));
        assert!(!hub.allow_resync(sid));

        hub.on_disconnect(
            sid,
            SimpleUser {
                user_id: uid,
                username: "anon".to_string(),
                authed: false,
                admin: false,
            },
        );

        assert!(
            hub.allow_resync(sid),
            "disconnect must clear the per-socket cooldown stamp"
        );
    }

    // Regression (#1): cache keyed only on updated_at would serve stale
    // user-derived fields (ratings, profile) after a player completes another
    // game without the current game row changing. TTL bounds that window.
    // Tests live in websocket::cache_tests (pure-logic, no DB needed).

    // Regression (#3): QueuedGuard tracks loader tasks waiting for a semaphore
    // permit, giving telemetry visibility into the queued-vs-active split.
    #[test]
    fn queued_guard_increments_and_decrements() {
        use std::sync::atomic::Ordering;
        let telemetry = Arc::new(WsTelemetry::default());
        assert_eq!(telemetry.load_user_state_queued.load(Ordering::Relaxed), 0);
        let guard = QueuedGuard::new(telemetry.clone());
        assert_eq!(telemetry.load_user_state_queued.load(Ordering::Relaxed), 1);
        drop(guard);
        assert_eq!(telemetry.load_user_state_queued.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn own_state_send_records_queue_depth_and_drop_reason() {
        use std::sync::atomic::Ordering;

        let hub = make_hub().await;
        let (tx, rx) = mpsc::channel(SOCKET_BUFFER_CAPACITY);
        let user_dest = DestKind::User as usize;

        for _ in 0..SOCKET_BUFFER_CAPACITY {
            hub.send_own_state_via_tx(&tx, &Bytes::from_static(b"a"));
        }
        hub.send_own_state_via_tx(&tx, &Bytes::from_static(b"b"));
        drop(rx);
        hub.send_own_state_via_tx(&tx, &Bytes::from_static(b"c"));

        assert_eq!(
            hub.data.telemetry.recipient_sends_ok[user_dest].load(Ordering::Relaxed),
            SOCKET_BUFFER_CAPACITY as u64
        );
        assert_eq!(
            hub.data.telemetry.recipient_drops_full[user_dest].load(Ordering::Relaxed),
            1
        );
        assert_eq!(
            hub.data.telemetry.recipient_drops_closed[user_dest].load(Ordering::Relaxed),
            1
        );
        assert_eq!(
            hub.data
                .telemetry
                .own_state_drops_total
                .load(Ordering::Relaxed),
            2
        );
        assert_eq!(
            hub.data
                .telemetry
                .max_queue_depth_seen
                .load(Ordering::Relaxed),
            SOCKET_BUFFER_CAPACITY as u64
        );
    }

    // Regression (#4): on_game_finished must NOT increment games_finalized_total.
    // `finalize_game` is the single authoritative path that bumps the counter,
    // so handlers/tests calling on_game_finished alone don't double-count.
    #[tokio::test]
    async fn games_finalized_not_incremented_by_on_game_finished() {
        use std::sync::atomic::Ordering;
        let hub = make_hub().await;
        let white_id = Uuid::new_v4();
        let black_id = Uuid::new_v4();
        let game_id = GameId("finalize-count".to_string());

        hub.on_game_finished(&game_id, white_id, black_id);
        // Calling twice to confirm idempotence on the counter too.
        hub.on_game_finished(&game_id, white_id, black_id);

        assert_eq!(
            hub.data
                .telemetry
                .games_finalized_total
                .load(Ordering::Relaxed),
            0,
            "on_game_finished must not increment games_finalized_total",
        );
    }
}
