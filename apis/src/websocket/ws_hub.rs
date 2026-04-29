use super::{
    messages::MessageDestination,
    telemetry::{DestKind, SendOutcome},
    WebsocketData,
};
use crate::{
    common::{
        ChallengeUpdate,
        GameUpdate,
        ScheduleUpdate,
        ServerMessage,
        ServerResult,
        TournamentUpdate,
        UserStatus,
        UserUpdate,
    },
    responses::{
        ChallengeResponse,
        GameResponse,
        HeartbeatResponse,
        ScheduleResponse,
        TournamentResponse,
        UserResponse,
    },
};
use bytes::Bytes;
use codee::{binary::MsgpackSerdeCodec, Encoder};
use dashmap::DashMap;
use db_lib::{
    get_conn,
    models::{Challenge, Game, Schedule, Tournament, TournamentInvitation, User},
    DbPool,
};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection};
use hive_lib::GameStatus;
use log::error;
use rand::Rng;
use shared_types::{GameId, TimeMode};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::{Arc, RwLock},
};
use tokio::sync::mpsc;
use uuid::Uuid;

const SOCKET_BUFFER_CAPACITY: usize = 128;
const LOBBY_GAME_ID: &str = "lobby";

/// WsHub — concurrent, non-actor replacement for `WsServer`.
///
/// Shutdown: there is currently no graceful-shutdown signal; sessions are dropped
/// when the process exits. Revisit if/when we add `CancellationToken` plumbing.
pub struct WsHub {
    /// `user_id → (socket_id → Sender)`. Outer DashMap shards on user_id;
    /// inner DashMap shards on socket_id. We never hold outer write while holding
    /// any inner lock, which keeps the two-level locking deadlock-free.
    sessions: DashMap<Uuid, DashMap<Uuid, mpsc::Sender<Bytes>>>,
    membership: RwLock<Membership>,
    pub data: Arc<WebsocketData>,
    pool: DbPool,
}

#[derive(Default)]
struct Membership {
    /// game → set of (user_id, socket_id) pairs subscribed to it.
    games_sockets: HashMap<GameId, HashSet<(Uuid, Uuid)>>,
    /// (user_id, socket_id) → set of games that socket is subscribed to.
    sockets_games: HashMap<(Uuid, Uuid), HashSet<GameId>>,
}

impl WsHub {
    pub fn new(data: Arc<WebsocketData>, pool: DbPool) -> Arc<Self> {
        Arc::new(Self {
            sessions: DashMap::new(),
            membership: RwLock::new(Membership::default()),
            data,
            pool,
        })
    }

    fn lobby() -> GameId {
        GameId(LOBBY_GAME_ID.to_string())
    }

    // ─── connect / disconnect ─────────────────────────────────────────────────

    /// Synchronously register a new socket and trigger the async user-state load.
    /// Consumes a clone of the Arc so the spawned load task can keep `self` alive.
    pub fn on_connect(
        self: Arc<Self>,
        socket_id: Uuid,
        user_id: Uuid,
        username: String,
        tx: mpsc::Sender<Bytes>,
    ) {
        let lobby = Self::lobby();
        let tx_for_load = tx.clone();
        // Lock order: membership write → outer DashMap shard. on_disconnect uses
        // the same order so the two operations can't deadlock and a fast
        // reconnect can't race a tail-end disconnect into a stale Offline.
        let is_first_socket = {
            let mut m = self.membership.write().expect("membership poisoned");
            let was_empty = {
                let user_entry =
                    self.sessions.entry(user_id).or_insert_with(DashMap::new);
                let empty = user_entry.is_empty();
                user_entry.insert(socket_id, tx);
                empty
            };
            m.games_sockets
                .entry(lobby.clone())
                .or_default()
                .insert((user_id, socket_id));
            m.sockets_games
                .entry((user_id, socket_id))
                .or_default()
                .insert(lobby.clone());
            was_empty
        };

        self.data.telemetry.inc_active_socket();
        if is_first_socket {
            self.data.telemetry.inc_active_user();
        }
        self.refresh_membership_gauges();

        // Async state load (urgent games, invitations, schedules, challenges, etc.)
        // matches the previous `Handler<Connect>` async block, which used
        // `ctx.wait` (blocked the actor but not the reader). Here we spawn
        // independently — readers and other dispatches proceed immediately.
        actix_rt::spawn(async move {
            self.load_user_state(socket_id, user_id, username, tx_for_load).await;
        });
    }

    /// Drops a socket, cleaning up its per-socket game subscriptions immediately.
    /// If it was the user's last socket, also cleans up user-level state and
    /// broadcasts Offline to the lobby.
    pub fn on_disconnect(&self, socket_id: Uuid, user_id: Uuid, username: String) {
        // Step 1: remove this socket from the user's inner map.
        let inner_now_empty = {
            let Some(sockets) = self.sessions.get(&user_id) else {
                return;
            };
            sockets.remove(&socket_id);
            sockets.is_empty()
        };
        self.data.telemetry.dec_active_socket();

        // Step 2: take the membership write lock and clean up this socket's
        // subscriptions. We do this for every disconnect — not just the last
        // socket — so closing tab A removes its game subscriptions immediately
        // even while tab B is still connected.
        //
        // If this is the last socket, we also atomically remove the user from
        // sessions (lock order: membership → outer DashMap shard, same as
        // on_connect, preventing the fast-reconnect race).
        let removed_user = {
            let mut m = self.membership.write().expect("membership poisoned");

            // Remove this socket from every game it subscribed to.
            if let Some(games) = m.sockets_games.remove(&(user_id, socket_id)) {
                for game_id in games {
                    if let Some(sockets) = m.games_sockets.get_mut(&game_id) {
                        sockets.remove(&(user_id, socket_id));
                        if sockets.is_empty() {
                            m.games_sockets.remove(&game_id);
                        }
                    }
                }
            }

            if inner_now_empty {
                let removed = self.sessions.remove_if(&user_id, |_, s| s.is_empty());
                if removed.is_some() {
                    self.data.telemetry.dec_active_user();
                    self.data.pings.remove(user_id);
                    true
                } else {
                    // A concurrent on_connect landed between our is_empty check
                    // and acquiring the membership lock — user is alive again.
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
        if removed_user && !self.sessions.contains_key(&user_id) {
            let message = ServerResult::Ok(Box::new(ServerMessage::UserStatus(UserUpdate {
                status: UserStatus::Offline,
                user: None,
                username,
            })));
            if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                self.fanout_lobby(&Bytes::from(serialized), DestKind::Global);
            }
        }
    }

    fn refresh_membership_gauges(&self) {
        let m = self.membership.read().expect("membership poisoned");
        let lobby = Self::lobby();
        // Count game keys that aren't the lobby. Don't compute as `len() - 1`:
        // the lobby key may be absent (e.g. when no users are connected) and
        // saturating_sub would undercount real game memberships in that state.
        let active_games = m
            .games_sockets
            .keys()
            .filter(|gid| *gid != &lobby)
            .count() as u64;
        let lobby_count = m.games_sockets.get(&lobby).map_or(0, |s| s.len()) as u64;
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
                let bytes_len = bytes.len();
                let used = socket.capacity_used();
                let outcome = socket.try_send_classified(bytes);
                let charged = match outcome {
                    SendOutcome::Closed => 0,
                    _ => bytes_len,
                };
                self.data
                    .telemetry
                    .record_send(DestKind::Direct, outcome, used, charged);
            }
            MessageDestination::User(user_id) => {
                self.send_to_user(user_id, DestKind::User, &bytes);
            }
            MessageDestination::Global => {
                if let Some((uid, sid)) = from {
                    self.ensure_membership(uid, sid, &Self::lobby());
                }
                self.fanout_lobby(&bytes, DestKind::Global);
            }
            MessageDestination::Game(game_id) => {
                if let Some((uid, sid)) = from {
                    self.ensure_membership(uid, sid, game_id);
                }
                let socket_pairs = self.sockets_in_game(game_id);
                for (uid, sid) in socket_pairs {
                    self.send_to_socket(&uid, &sid, DestKind::Game, &bytes);
                }
            }
            MessageDestination::GameSpectators(game_id, white_id, black_id) => {
                if let Some((uid, sid)) = from {
                    self.ensure_membership(uid, sid, game_id);
                }
                let socket_pairs: Vec<(Uuid, Uuid)> = {
                    let m = self.membership.read().expect("membership poisoned");
                    m.games_sockets
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
            MessageDestination::Tournament(tournament_id) => {
                let Ok(mut conn) = get_conn(&self.pool).await else {
                    return;
                };
                let Ok(tournament) =
                    Tournament::from_nanoid(&tournament_id.to_string(), &mut conn).await
                else {
                    return;
                };
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
                for uid in user_ids {
                    self.send_to_user(&uid, DestKind::Tournament, &bytes);
                }
            }
        }
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
        let games: Vec<(GameId, HashSet<(Uuid, Uuid)>)> = {
            let m = self.membership.read().expect("membership poisoned");
            m.games_sockets
                .iter()
                .filter(|(gid, _)| gid.0.as_str() != LOBBY_GAME_ID)
                .map(|(gid, sockets)| (gid.clone(), sockets.clone()))
                .collect()
        };

        for (game_id, socket_pairs) in games {
            let Ok(mut conn) = get_conn(&self.pool).await else {
                continue;
            };
            let Ok(game) = Game::find_by_game_id(&game_id, &mut conn).await else {
                continue;
            };
            if matches!(
                GameStatus::from_str(&game.game_status),
                Ok(GameStatus::Finished(_)) | Ok(GameStatus::Adjudicated)
            ) {
                // Game is over — evict from membership so the heartbeat
                // stops querying the DB for it on every tick.
                self.evict_game_from_membership(&game_id);
                continue;
            }
            if game.time_mode == TimeMode::Untimed.to_string() {
                // No timer to report — skip heartbeat but keep membership so
                // players still receive real-time move notifications.
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

    fn send_to_user(&self, user_id: &Uuid, dest: DestKind, bytes: &Bytes) {
        let Some(sockets) = self.sessions.get(user_id) else {
            return;
        };
        // Don't reap on Full or Closed.
        //
        // - Full means the socket's outbound queue is at capacity. The reader
        //   is still alive and heartbeating (heartbeat ping bypasses the mpsc),
        //   so removing the socket from sessions would orphan it: it would keep
        //   running, receive no application messages, and on_disconnect would
        //   later short-circuit because sessions[user_id] was already gone —
        //   leaking gauge state, membership, and the offline broadcast.
        // - Closed means the receiver was dropped (writer task exited because
        //   session.binary errored). The reader will detect the broken transport
        //   on its next ping/poll and call on_disconnect, which is the single
        //   source of cleanup.
        //
        // The bounded queue still prevents OOM (the message itself is dropped).
        // TODO: if we want a real "force-close slow client" mechanism, plumb a
        // Session handle (or a kill-channel) into the hub so we can actually
        // close the WebSocket on Full and let the reader clean up normally.
        for entry in sockets.iter() {
            let used = SOCKET_BUFFER_CAPACITY.saturating_sub(entry.value().capacity());
            match entry.value().try_send(bytes.clone()) {
                Ok(_) => self.data.telemetry.record_send(
                    dest,
                    SendOutcome::Ok,
                    used,
                    bytes.len(),
                ),
                Err(mpsc::error::TrySendError::Full(_)) => {
                    self.data.telemetry.record_send(
                        dest,
                        SendOutcome::Full,
                        used,
                        bytes.len(),
                    );
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    self.data
                        .telemetry
                        .record_send(dest, SendOutcome::Closed, used, 0);
                }
            }
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
        let used = SOCKET_BUFFER_CAPACITY.saturating_sub(tx.capacity());
        match tx.try_send(bytes.clone()) {
            Ok(_) => self.data.telemetry.record_send(dest, SendOutcome::Ok, used, bytes.len()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.data.telemetry.record_send(dest, SendOutcome::Full, used, bytes.len());
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.data.telemetry.record_send(dest, SendOutcome::Closed, used, 0);
            }
        }
    }

    fn fanout_lobby(&self, bytes: &Bytes, dest: DestKind) {
        let socket_pairs = self.sockets_in_game(&Self::lobby());
        for (uid, sid) in socket_pairs {
            self.send_to_socket(&uid, &sid, dest, bytes);
        }
    }

    fn sockets_in_game(&self, game_id: &GameId) -> Vec<(Uuid, Uuid)> {
        let m = self.membership.read().expect("membership poisoned");
        m.games_sockets
            .get(game_id)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default()
    }

    fn ensure_membership(&self, user_id: Uuid, socket_id: Uuid, game_id: &GameId) {
        let already_member = {
            let m = self.membership.read().expect("membership poisoned");
            m.games_sockets
                .get(game_id)
                .map_or(false, |s| s.contains(&(user_id, socket_id)))
        };
        if already_member {
            return;
        }
        {
            let mut m = self.membership.write().expect("membership poisoned");
            m.games_sockets
                .entry(game_id.clone())
                .or_default()
                .insert((user_id, socket_id));
            m.sockets_games
                .entry((user_id, socket_id))
                .or_default()
                .insert(game_id.clone());
        }
        self.refresh_membership_gauges();
    }

    /// Symmetric counterpart to `ensure_membership`. Removes the specific socket
    /// from `game_id`'s subscriber set and prunes both maps if they become empty.
    pub fn leave_membership(&self, user_id: Uuid, socket_id: Uuid, game_id: &GameId) {
        if game_id.0.as_str() == LOBBY_GAME_ID {
            return;
        }
        {
            let mut m = self.membership.write().expect("membership poisoned");
            if let Some(sockets) = m.games_sockets.get_mut(game_id) {
                sockets.remove(&(user_id, socket_id));
                if sockets.is_empty() {
                    m.games_sockets.remove(game_id);
                }
            }
            if let Some(games) = m.sockets_games.get_mut(&(user_id, socket_id)) {
                games.remove(game_id);
                if games.is_empty() {
                    m.sockets_games.remove(&(user_id, socket_id));
                }
            }
        }
        self.refresh_membership_gauges();
    }

    /// Remove a game from membership entirely — all sockets that were subscribed
    /// to it are unsubscribed in one write. Used by `game_heartbeat` to evict
    /// finished or untimed games so they stop generating DB queries each tick.
    fn evict_game_from_membership(&self, game_id: &GameId) {
        {
            let mut m = self.membership.write().expect("membership poisoned");
            if let Some(sockets) = m.games_sockets.remove(game_id) {
                for socket_pair in sockets {
                    if let Some(games) = m.sockets_games.get_mut(&socket_pair) {
                        games.remove(game_id);
                        if games.is_empty() {
                            m.sockets_games.remove(&socket_pair);
                        }
                    }
                }
            }
        }
        self.refresh_membership_gauges();
    }

    /// Returns true iff the specific socket we were spawned for is still in
    /// `sessions`. Used by `load_user_state` to bail when a fast disconnect
    /// (or disconnect+reconnect with a different socket_id) raced our DB load.
    fn is_socket_connected(&self, user_id: Uuid, socket_id: Uuid) -> bool {
        self.sessions
            .get(&user_id)
            .map_or(false, |sockets| sockets.contains_key(&socket_id))
    }

    // ─── test helpers ─────────────────────────────────────────────────────────

    /// Register a socket directly without triggering the async state load.
    /// Returns the receiver for the socket's outbound channel.
    #[cfg(test)]
    fn register_socket(&self, user_id: Uuid, socket_id: Uuid) -> mpsc::Receiver<Bytes> {
        let (tx, rx) = mpsc::channel(SOCKET_BUFFER_CAPACITY);
        let lobby = Self::lobby();
        let mut m = self.membership.write().expect("membership poisoned");
        self.sessions
            .entry(user_id)
            .or_insert_with(DashMap::new)
            .insert(socket_id, tx);
        m.games_sockets
            .entry(lobby.clone())
            .or_default()
            .insert((user_id, socket_id));
        m.sockets_games
            .entry((user_id, socket_id))
            .or_default()
            .insert(lobby);
        rx
    }

    /// Subscribe an already-registered socket to a game channel.
    #[cfg(test)]
    fn join_game(&self, user_id: Uuid, socket_id: Uuid, game_id: &GameId) {
        let mut m = self.membership.write().expect("membership poisoned");
        m.games_sockets
            .entry(game_id.clone())
            .or_default()
            .insert((user_id, socket_id));
        m.sockets_games
            .entry((user_id, socket_id))
            .or_default()
            .insert(game_id.clone());
    }

    // ─── user-state load (the long async block from Handler<Connect>) ──────────

    async fn load_user_state(
        &self,
        socket_id: Uuid,
        user_id: Uuid,
        username: String,
        tx: mpsc::Sender<Bytes>,
    ) {
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

        // Send Online status of every currently-connected user to the new connector.
        let existing_user_ids: Vec<Uuid> = self.sessions.iter().map(|e| *e.key()).collect();
        for uid in existing_user_ids {
            if let Ok(user_response) = UserResponse::from_uuid(&uid, &mut conn).await {
                let message = ServerResult::Ok(Box::new(ServerMessage::UserStatus(UserUpdate {
                    status: UserStatus::Online,
                    user: Some(user_response.clone()),
                    username: user_response.username,
                })));
                if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                    let _ = tx.try_send(Bytes::from(serialized));
                }
            }
        }

        // Branch: authed user with a DB row, vs anonymous.
        if let Ok(user) = User::find_by_uuid(&user_id, &mut conn).await {
            // Re-check before the Online broadcast: the slow DB lookup gives
            // plenty of time for a disconnect to race in. If it has, our Online
            // would override the Offline that on_disconnect already sent and
            // leave the lobby ghosting a user that's actually gone.
            if !self.is_socket_connected(user_id, socket_id) {
                return;
            }
            // Announce the new user's Online status to lobby.
            if let Ok(user_response) = UserResponse::from_model(&user, &mut conn).await {
                let message = ServerResult::Ok(Box::new(ServerMessage::UserStatus(UserUpdate {
                    status: UserStatus::Online,
                    user: Some(user_response),
                    username: username.clone(),
                })));
                if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                    self.fanout_lobby(&Bytes::from(serialized), DestKind::Global);
                }
            }

            // Urgent games.
            let game_ids = match user.get_urgent_nanoids(&mut conn).await {
                Ok(ids) => ids,
                Err(e) => {
                    error!("Failed to get urgent game_ids for user {user_id}: {e}");
                    Vec::new()
                }
            };
            if !game_ids.is_empty() {
                let games = conn
                    .transaction::<_, anyhow::Error, _>(move |tc| {
                        let mut games = Vec::new();
                        async move {
                            for gid in game_ids {
                                if let Ok(game) =
                                    GameResponse::new_from_game_id(&gid, tc).await
                                {
                                    if !game.finished {
                                        games.push(game);
                                    }
                                }
                            }
                            Ok(games)
                        }
                        .scope_boxed()
                    })
                    .await;
                if let Ok(games) = games {
                    let message = ServerResult::Ok(Box::new(ServerMessage::Game(Box::new(
                        GameUpdate::Urgent(games),
                    ))));
                    if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                        let _ = tx.try_send(Bytes::from(serialized));
                    }
                }
            }

            // Tournament invitations.
            if let Ok(invitations) =
                TournamentInvitation::find_by_user(&user.id, &mut conn).await
            {
                for invitation in invitations {
                    if let Ok(response) =
                        TournamentResponse::from_uuid(&invitation.tournament_id, &mut conn).await
                    {
                        let message = ServerResult::Ok(Box::new(ServerMessage::Tournament(
                            TournamentUpdate::Invited(response.tournament_id.clone()),
                        )));
                        if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                            let _ = tx.try_send(Bytes::from(serialized));
                        }
                    }
                }
            }

            // Schedule notifications.
            if let Ok(schedules) = Schedule::find_user_notifications(user.id, &mut conn).await {
                for schedule in schedules {
                    let is_opponent = schedule.opponent_id == user.id;
                    if let Ok(response) =
                        ScheduleResponse::from_model(schedule, &mut conn).await
                    {
                        let schedule_update = if is_opponent {
                            ScheduleUpdate::Proposed(response)
                        } else {
                            ScheduleUpdate::Accepted(response)
                        };
                        let message = ServerResult::Ok(Box::new(ServerMessage::Schedule(
                            schedule_update,
                        )));
                        if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                            let _ = tx.try_send(Bytes::from(serialized));
                        }
                    }
                }
            }

            // Challenges (public excluding self + own + direct).
            let mut responses = Vec::new();
            if let Ok(challenges) =
                Challenge::get_public_exclude_user(user.id, &mut conn).await
            {
                for challenge in challenges {
                    if let Ok(response) =
                        ChallengeResponse::from_model(&challenge, &mut conn).await
                    {
                        responses.push(response);
                    }
                }
            }
            if let Ok(challenges) = Challenge::get_own(user.id, &mut conn).await {
                for challenge in challenges {
                    if let Ok(response) =
                        ChallengeResponse::from_model(&challenge, &mut conn).await
                    {
                        responses.push(response);
                    }
                }
            }
            if let Ok(challenges) = Challenge::direct_challenges(user.id, &mut conn).await {
                for challenge in challenges {
                    if let Ok(response) =
                        ChallengeResponse::from_model(&challenge, &mut conn).await
                    {
                        responses.push(response);
                    }
                }
            }
            let message = ServerResult::Ok(Box::new(ServerMessage::Challenge(
                ChallengeUpdate::Challenges(responses),
            )));
            if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                let _ = tx.try_send(Bytes::from(serialized));
            }
        } else {
            // Anonymous: only public challenges.
            let mut responses = Vec::new();
            if let Ok(challenges) = Challenge::get_public(&mut conn).await {
                for challenge in challenges {
                    if let Ok(response) =
                        ChallengeResponse::from_model(&challenge, &mut conn).await
                    {
                        responses.push(response);
                    }
                }
            }
            let message = ServerResult::Ok(Box::new(ServerMessage::Challenge(
                ChallengeUpdate::Challenges(responses),
            )));
            if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                let _ = tx.try_send(Bytes::from(serialized));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::messages::SocketTx;

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

        hub.dispatch(&MessageDestination::User(uid), Bytes::from_static(b"hi"), None)
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

        hub.join_game(uid_a, sid_a, &game_id);
        hub.join_game(uid_b, sid_b, &game_id);

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

        hub.join_game(white_id, white_sid, &game_id);
        hub.join_game(black_id, black_sid, &game_id);
        hub.join_game(spec_id, spec_sid, &game_id);

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
    async fn leave_membership_unsubscribes_socket_from_game() {
        let hub = make_hub().await;
        let uid = Uuid::new_v4();
        let sid = Uuid::new_v4();
        let game_id = GameId("leave-game".to_string());

        let mut rx = hub.register_socket(uid, sid);
        hub.join_game(uid, sid, &game_id);
        hub.leave_membership(uid, sid, &game_id);

        hub.dispatch(
            &MessageDestination::Game(game_id),
            Bytes::from_static(b"after-leave"),
            None,
        )
        .await;

        assert!(rx.try_recv().is_err());
    }
}
