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
    games_users: HashMap<GameId, HashSet<Uuid>>,
    users_games: HashMap<Uuid, HashSet<GameId>>,
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
            m.games_users
                .entry(lobby.clone())
                .or_default()
                .insert(user_id);
            m.users_games
                .entry(user_id)
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
            self.load_user_state(socket_id, user_id, username).await;
        });
    }

    /// Drops a socket. If it was the user's last, cleans membership and
    /// broadcasts an offline status to the lobby.
    pub fn on_disconnect(&self, socket_id: Uuid, user_id: Uuid, username: String) {
        // Step 1: remove the socket from the user's inner map.
        let inner_now_empty = {
            let Some(sockets) = self.sessions.get(&user_id) else {
                return;
            };
            sockets.remove(&socket_id);
            sockets.is_empty()
        };
        self.data.telemetry.dec_active_socket();

        if !inner_now_empty {
            return;
        }

        // Step 2: take the membership write lock FIRST, then atomically check-
        // and-remove the user from sessions. Holding the membership lock across
        // both the predicate-and-remove AND the membership cleanup prevents a
        // fast on_connect (which takes membership write before its sessions
        // insert) from sliding in between and ending up with the cleanup path
        // stripping the freshly-reconnected user from the lobby and broadcasting
        // a stale Offline. Lock order matches on_connect: membership → shard.
        let mut m = self.membership.write().expect("membership poisoned");
        let removed = self.sessions.remove_if(&user_id, |_, s| s.is_empty());
        if removed.is_none() {
            // User reconnected between our is_empty check and getting here, OR
            // a concurrent on_connect is queued behind our membership lock. Either
            // way, the user is or will be alive — skip cleanup.
            return;
        }
        self.data.telemetry.dec_active_user();

        // Step 3: drop user from every game's membership (still under the lock).
        if let Some(games) = m.users_games.remove(&user_id) {
            for game_id in games {
                if let Some(users) = m.games_users.get_mut(&game_id) {
                    users.remove(&user_id);
                    if users.is_empty() {
                        m.games_users.remove(&game_id);
                    }
                }
            }
        }
        drop(m); // release before refresh_membership_gauges takes a read lock

        self.refresh_membership_gauges();

        // Step 4: announce offline to the lobby — but only if the user has not
        // come back in the brief window between our drop(m) and getting here.
        // A queued on_connect would have been blocked on the membership lock;
        // when we drop, it can run to completion AND its spawned
        // load_user_state can broadcast Online before our fanout_lobby below.
        // If our Offline arrives at lobby receivers second, clients end up
        // marking a connected user as offline. Re-checking here lets the
        // reconnect's Online be the final word — at the cost of skipping the
        // intermediate Offline blip, which is the desired behaviour anyway.
        if self.sessions.contains_key(&user_id) {
            return;
        }
        let message = ServerResult::Ok(Box::new(ServerMessage::UserStatus(UserUpdate {
            status: UserStatus::Offline,
            user: None,
            username,
        })));
        if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
            self.fanout_lobby(&Bytes::from(serialized), DestKind::Global);
        }
    }

    fn refresh_membership_gauges(&self) {
        let m = self.membership.read().expect("membership poisoned");
        let lobby = Self::lobby();
        // Count game keys that aren't the lobby. Don't compute as `len() - 1`:
        // the lobby key may be absent (e.g. when no users are connected) and
        // saturating_sub would undercount real game memberships in that state.
        let active_games = m
            .games_users
            .keys()
            .filter(|gid| *gid != &lobby)
            .count() as u64;
        let lobby_count = m.games_users.get(&lobby).map_or(0, |s| s.len()) as u64;
        self.data.telemetry.set_active_games(active_games);
        self.data.telemetry.set_lobby_subscribers(lobby_count);
    }

    // ─── dispatch ─────────────────────────────────────────────────────────────

    pub async fn dispatch(
        &self,
        dest: &MessageDestination,
        bytes: Bytes,
        from: Option<Uuid>,
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
                if let Some(uid) = from {
                    self.ensure_membership(uid, &Self::lobby());
                }
                self.fanout_lobby(&bytes, DestKind::Global);
            }
            MessageDestination::Game(game_id) => {
                if let Some(uid) = from {
                    self.ensure_membership(uid, game_id);
                }
                let user_ids = self.users_in_game(game_id);
                for uid in user_ids {
                    self.send_to_user(&uid, DestKind::Game, &bytes);
                }
            }
            MessageDestination::GameSpectators(game_id, white_id, black_id) => {
                if let Some(uid) = from {
                    self.ensure_membership(uid, game_id);
                }
                let user_ids: Vec<Uuid> = {
                    let m = self.membership.read().expect("membership poisoned");
                    m.games_users
                        .get(game_id)
                        .map(|s| {
                            s.iter()
                                .filter(|u| *u != white_id && *u != black_id)
                                .copied()
                                .collect()
                        })
                        .unwrap_or_default()
                };
                for uid in user_ids {
                    self.send_to_user(&uid, DestKind::GameSpectators, &bytes);
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
        let games: Vec<(GameId, HashSet<Uuid>)> = {
            let m = self.membership.read().expect("membership poisoned");
            m.games_users
                .iter()
                .filter(|(gid, _)| gid.0.as_str() != LOBBY_GAME_ID)
                .map(|(gid, users)| (gid.clone(), users.clone()))
                .collect()
        };

        for (game_id, user_ids) in games {
            let Ok(mut conn) = get_conn(&self.pool).await else {
                continue;
            };
            let Ok(game) = Game::find_by_game_id(&game_id, &mut conn).await else {
                continue;
            };
            if game.game_status != GameStatus::InProgress.to_string()
                || game.time_mode == TimeMode::Untimed.to_string()
            {
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
            for user_id in user_ids {
                self.send_to_user(&user_id, DestKind::Game, &bytes);
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

    fn fanout_lobby(&self, bytes: &Bytes, dest: DestKind) {
        let user_ids = self.users_in_game(&Self::lobby());
        for uid in user_ids {
            self.send_to_user(&uid, dest, bytes);
        }
    }

    fn users_in_game(&self, game_id: &GameId) -> Vec<Uuid> {
        let m = self.membership.read().expect("membership poisoned");
        m.games_users
            .get(game_id)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default()
    }

    fn ensure_membership(&self, user_id: Uuid, game_id: &GameId) {
        let already_member = {
            let m = self.membership.read().expect("membership poisoned");
            m.games_users
                .get(game_id)
                .map_or(false, |s| s.contains(&user_id))
        };
        if already_member {
            return;
        }
        {
            let mut m = self.membership.write().expect("membership poisoned");
            m.games_users
                .entry(game_id.clone())
                .or_default()
                .insert(user_id);
            m.users_games
                .entry(user_id)
                .or_default()
                .insert(game_id.clone());
        }
        // First dispatch into a game adds the user to its membership; the
        // gauges (`active_games`, `lobby_subscribers`) need to reflect that.
        // Without this, `active_games` would only be refreshed on connect /
        // disconnect, leaving the metric stale during a session.
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

    // ─── user-state load (the long async block from Handler<Connect>) ─────────

    async fn load_user_state(&self, socket_id: Uuid, user_id: Uuid, username: String) {
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
                    self.send_to_user(&user_id, DestKind::User, &Bytes::from(serialized));
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
                        self.send_to_user(&user_id, DestKind::User, &Bytes::from(serialized));
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
                            self.send_to_user(&user_id, DestKind::User, &Bytes::from(serialized));
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
                            self.send_to_user(&user_id, DestKind::User, &Bytes::from(serialized));
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
                self.send_to_user(&user_id, DestKind::User, &Bytes::from(serialized));
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
                self.send_to_user(&user_id, DestKind::User, &Bytes::from(serialized));
            }
        }
    }
}

