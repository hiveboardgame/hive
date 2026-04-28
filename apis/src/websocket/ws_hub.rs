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
        let is_first_socket = {
            let user_entry = self.sessions.entry(user_id).or_insert_with(DashMap::new);
            let was_empty = user_entry.is_empty();
            user_entry.insert(socket_id, tx);
            was_empty
        };

        {
            let mut m = self.membership.write().expect("membership poisoned");
            m.games_users.entry(lobby.clone()).or_default().insert(user_id);
            m.users_games.entry(user_id).or_default().insert(lobby.clone());
        }

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
            self.load_user_state(user_id, username).await;
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

        // Step 2: atomically remove the user iff still empty.
        // (Closes the race with a concurrent on_connect for the same user.)
        let removed = self.sessions.remove_if(&user_id, |_, s| s.is_empty());
        if removed.is_none() {
            return;
        }
        self.data.telemetry.dec_active_user();

        // Step 3: drop user from every game's membership.
        {
            let mut m = self.membership.write().expect("membership poisoned");
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
        }
        self.refresh_membership_gauges();

        // Step 4: announce offline to the lobby.
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
        let total_games = m.games_users.len() as u64;
        let lobby_count = m
            .games_users
            .get(&Self::lobby())
            .map_or(0, |s| s.len()) as u64;
        self.data.telemetry.set_active_games(total_games.saturating_sub(1));
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
        let dead: Vec<Uuid> = {
            let Some(sockets) = self.sessions.get(user_id) else {
                return;
            };
            let mut dead = Vec::new();
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
                        dead.push(*entry.key());
                        self.data.telemetry.record_send(
                            dest,
                            SendOutcome::Full,
                            used,
                            bytes.len(),
                        );
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        dead.push(*entry.key());
                        self.data
                            .telemetry
                            .record_send(dest, SendOutcome::Closed, used, 0);
                    }
                }
            }
            dead
        };

        if !dead.is_empty() {
            if let Some(sockets) = self.sessions.get(user_id) {
                for id in &dead {
                    sockets.remove(id);
                }
            }
        }
        // Atomic: removes the outer entry only if still empty under shard write lock.
        // Closes the race with a concurrent `on_connect` inserting a new socket.
        self.sessions.remove_if(user_id, |_, s| s.is_empty());
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

    // ─── user-state load (the long async block from Handler<Connect>) ─────────

    async fn load_user_state(&self, user_id: Uuid, username: String) {
        let Ok(mut conn) = get_conn(&self.pool).await else {
            return;
        };

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

