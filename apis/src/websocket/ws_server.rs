use super::{
    messages::{GameHB, MessageDestination, Ping, SocketTx},
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
    websocket::messages::{ClientActorMessage, Connect, Disconnect},
};
use actix::{
    prelude::{Actor, Context, Handler},
    AsyncContext,
    WrapFuture,
};
use codee::{binary::MsgpackSerdeCodec, Encoder};
use db_lib::{
    get_conn,
    models::{Challenge, Game, Schedule, Tournament, TournamentInvitation, User},
    DbPool,
};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection};
use hive_lib::GameStatus;
use log::{error, warn};
use rand::Rng;
use shared_types::{GameId, TimeMode};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use uuid::Uuid;

#[derive(Debug)]
pub struct WsServer {
    id: String,
    sessions: HashMap<Uuid, HashMap<Uuid, SocketTx>>, // user_id → (socket_id → SocketTx)
    games_users: HashMap<GameId, HashSet<Uuid>>,
    users_games: HashMap<Uuid, HashSet<String>>,
    data: Arc<WebsocketData>,
    pool: DbPool,
}

impl WsServer {
    pub fn new(data: Arc<WebsocketData>, pool: DbPool) -> WsServer {
        WsServer {
            id: String::from("lobby"),
            sessions: HashMap::new(),
            games_users: HashMap::new(),
            users_games: HashMap::new(),
            data,
            pool,
        }
    }
}

impl WsServer {
    fn send_message(&mut self, message: &Vec<u8>, id_to: &Uuid) {
        let Some(sockets) = self.sessions.get_mut(id_to) else {
            return;
        };
        sockets.retain(|_, socket| socket.try_send(message.clone()));
        if sockets.is_empty() {
            self.sessions.remove(id_to);
        }
    }
}

impl Actor for WsServer {
    type Context = Context<Self>;
}

impl Handler<Ping> for WsServer {
    type Result = ();

    fn handle(&mut self, _msg: Ping, _ctx: &mut Context<Self>) {
        let mut rng = rand::rng();
        let user_ids: Vec<Uuid> = self.sessions.keys().copied().collect();
        for user_id in user_ids {
            let nonce = rng.random::<u64>();
            self.data.pings.set_nonce(user_id, nonce);
            let message = ServerResult::Ok(Box::new(ServerMessage::Ping {
                nonce,
                value: self.data.pings.value(user_id),
            }));
            if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                self.send_message(&serialized, &user_id);
            };
        }
    }
}

impl Handler<GameHB> for WsServer {
    type Result = ();

    fn handle(&mut self, _msg: GameHB, ctx: &mut Context<Self>) {
        for (game_id, user_ids) in self.games_users.clone() {
            if game_id.0.as_str() == "lobby" {
                continue;
            }
            let pool = self.pool.clone();
            let sessions = self.sessions.clone();
            let future = async move {
                if let Ok(mut conn) = get_conn(&pool).await {
                    if let Ok(game) = Game::find_by_game_id(&game_id, &mut conn).await {
                        if game.game_status == GameStatus::InProgress.to_string()
                            && game.time_mode != TimeMode::Untimed.to_string()
                        {
                            if let Ok((id, white, black)) = game.get_heartbeat() {
                                let hb = HeartbeatResponse {
                                    game_id: id,
                                    white_time_left: white,
                                    black_time_left: black,
                                };
                                let message = ServerResult::Ok(Box::new(ServerMessage::Game(
                                    Box::new(GameUpdate::Heartbeat(hb)),
                                )));
                                if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                                    for user_id in user_ids {
                                        if let Some(sockets) = sessions.get(&user_id) {
                                            for socket in sockets.values() {
                                                socket.try_send(serialized.clone());
                                            }
                                        }
                                    }
                                };
                            }
                        }
                    }
                }
            };
            let actor_future = future.into_actor(self);
            ctx.wait(actor_future);
        }
    }
}

impl Handler<Disconnect> for WsServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        if let Some(user_sessions) = self.sessions.get_mut(&msg.user_id) {
            user_sessions.remove(&msg.socket_id);
            if user_sessions.is_empty() {
                self.sessions.remove(&msg.user_id);
            }
        }
        if !self.sessions.contains_key(&msg.user_id) {
            if let Some(games) = self.users_games.remove(&msg.user_id) {
                for game in games.iter() {
                    let game_id = GameId(game.to_string());
                    if let Some(game_users) = self.games_users.get_mut(&game_id) {
                        if game_users.len() > 1 {
                            game_users.remove(&msg.user_id);
                        } else {
                            self.games_users.remove(&game_id);
                        }
                    }
                }
            }
            let message = ServerResult::Ok(Box::new(ServerMessage::UserStatus(UserUpdate {
                status: UserStatus::Offline,
                user: None,
                username: msg.username,
            })));
            if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                let game_id = GameId(self.id.clone());
                if let Some(ws_server) = self.games_users.get_mut(&game_id) {
                    ws_server.remove(&msg.user_id);
                }
                let lobby_users: Vec<Uuid> = self
                    .games_users
                    .get(&game_id)
                    .map(|s| s.iter().copied().collect())
                    .unwrap_or_default();
                for user_id in lobby_users {
                    self.send_message(&serialized, &user_id);
                }
            };
        }
    }
}

impl Handler<Connect> for WsServer {
    type Result = ();

    fn handle(&mut self, msg: Connect, ctx: &mut Context<Self>) -> Self::Result {
        let user_id = msg.user_id;
        self.games_users
            .entry(GameId(msg.game_id.clone()))
            .or_default()
            .insert(msg.user_id);
        self.users_games
            .entry(msg.user_id)
            .or_default()
            .insert(msg.game_id.clone());
        self.sessions
            .entry(msg.user_id)
            .or_default()
            .insert(msg.socket.socket_id, msg.socket.clone());
        let pool = self.pool.clone();
        let address = ctx.address().clone();
        let games_users = self.games_users.clone();
        let sessions = self.sessions.clone();
        let future = async move {
            if let Ok(mut conn) = get_conn(&pool).await {
                for uuid in sessions.keys() {
                    if let Ok(user_response) = UserResponse::from_uuid(uuid, &mut conn).await {
                        let message =
                            ServerResult::Ok(Box::new(ServerMessage::UserStatus(UserUpdate {
                                status: UserStatus::Online,
                                user: Some(user_response.clone()),
                                username: user_response.username,
                            })));
                        if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                            let cam = ClientActorMessage {
                                destination: MessageDestination::User(user_id),
                                serialized,
                                from: Some(user_id),
                            };
                            address.do_send(cam);
                        };
                    }
                }

                let serialized = if let Ok(user) = User::find_by_uuid(&user_id, &mut conn).await {
                    if let Ok(user_response) = UserResponse::from_model(&user, &mut conn).await {
                        let message =
                            ServerResult::Ok(Box::new(ServerMessage::UserStatus(UserUpdate {
                                status: UserStatus::Online,
                                user: Some(user_response),
                                username: msg.username,
                            })));
                        if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                            if let Some(user_ids) = games_users.get(&GameId(msg.game_id)) {
                                for id in user_ids {
                                    if let Some(sockets) = sessions.get(id) {
                                        for socket in sockets.values() {
                                            socket.try_send(serialized.clone());
                                        }
                                    }
                                }
                            }
                        };
                    }

                    let game_ids_result = user.get_urgent_nanoids(&mut conn).await;
                    let game_ids = match game_ids_result {
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
                                    for game_id in game_ids {
                                        if let Ok(game) =
                                            GameResponse::new_from_game_id(&game_id, tc).await
                                        {
                                            if !game.finished {
                                                games.push(game)
                                            }
                                        }
                                    }
                                    Ok(games)
                                }
                                .scope_boxed()
                            })
                            .await;
                        if let Ok(games) = games {
                            let message = ServerResult::Ok(Box::new(ServerMessage::Game(
                                Box::new(GameUpdate::Urgent(games)),
                            )));
                            if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                                let cam = ClientActorMessage {
                                    destination: MessageDestination::User(user_id),
                                    serialized,
                                    from: Some(user_id),
                                };
                                address.do_send(cam);
                            };
                        }
                    }
                    if let Ok(invitations) =
                        TournamentInvitation::find_by_user(&user.id, &mut conn).await
                    {
                        for invitation in invitations {
                            if let Ok(response) =
                                TournamentResponse::from_uuid(&invitation.tournament_id, &mut conn)
                                    .await
                            {
                                let message =
                                    ServerResult::Ok(Box::new(ServerMessage::Tournament(
                                        TournamentUpdate::Invited(response.tournament_id.clone()),
                                    )));
                                if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                                    let cam = ClientActorMessage {
                                        destination: MessageDestination::User(user_id),
                                        serialized,
                                        from: Some(user_id),
                                    };
                                    address.do_send(cam);
                                };
                            }
                        }
                    }

                    if let Ok(schedules) =
                        Schedule::find_user_notifications(user.id, &mut conn).await
                    {
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
                                    let cam = ClientActorMessage {
                                        destination: MessageDestination::User(user_id),
                                        serialized,
                                        from: Some(user_id),
                                    };
                                    address.do_send(cam);
                                };
                            }
                        }
                    }

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
                    MsgpackSerdeCodec::encode(&message)
                } else {
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
                    MsgpackSerdeCodec::encode(&message)
                };
                if let Ok(serialized) = serialized {
                    let cam = ClientActorMessage {
                        destination: MessageDestination::User(user_id),
                        serialized,
                        from: Some(user_id),
                    };
                    address.do_send(cam);
                };
            }
        };
        let actor_future = future.into_actor(self);
        ctx.wait(actor_future);
    }
}

impl Handler<ClientActorMessage> for WsServer {
    type Result = ();

    fn handle(&mut self, cam: ClientActorMessage, ctx: &mut Context<Self>) -> Self::Result {
        match cam.destination {
            MessageDestination::Direct(socket) => {
                socket.try_send(cam.serialized);
            }
            MessageDestination::Global => {
                if let Some(from) = cam.from {
                    self.games_users
                        .entry(GameId(self.id.clone()))
                        .or_default()
                        .insert(from);
                }
                if let Some(users) = self.games_users.get(&GameId(self.id.clone())) {
                    let user_ids: Vec<Uuid> = users.iter().copied().collect();
                    for client in user_ids {
                        self.send_message(&cam.serialized, &client);
                    }
                } else {
                    warn!(
                        "Game '{}' not found in games_users when sending global message",
                        self.id
                    );
                }
            }
            MessageDestination::Game(ref game_id) => {
                if let Some(from) = cam.from {
                    self.games_users
                        .entry(game_id.clone())
                        .or_default()
                        .insert(from);
                }
                if let Some(users) = self.games_users.get(game_id) {
                    let user_ids: Vec<Uuid> = users.iter().copied().collect();
                    for client in user_ids {
                        self.send_message(&cam.serialized, &client);
                    }
                } else {
                    warn!("Game '{game_id}' not found in games_users when sending game message");
                }
            }
            MessageDestination::GameSpectators(game_id, white_id, black_id) => {
                if let Some(from) = cam.from {
                    self.games_users
                        .entry(game_id.clone())
                        .or_default()
                        .insert(from);
                }
                if let Some(users) = self.games_users.get(&game_id) {
                    let user_ids: Vec<Uuid> = users
                        .iter()
                        .filter(|&&u| u != white_id && u != black_id)
                        .copied()
                        .collect();
                    for user in user_ids {
                        self.send_message(&cam.serialized, &user);
                    }
                } else {
                    warn!("Game '{game_id}' not found in games_users when sending game spectators message");
                }
            }
            MessageDestination::User(user_id) => {
                self.send_message(&cam.serialized, &user_id);
            }
            MessageDestination::Tournament(tournament) => {
                let pool = self.pool.clone();
                let sessions = self.sessions.clone();
                let future = async move {
                    if let Ok(mut conn) = get_conn(&pool).await {
                        if let Ok(tournament) =
                            Tournament::from_nanoid(&tournament.to_string(), &mut conn).await
                        {
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
                            for user_id in user_ids {
                                if let Some(sockets) = sessions.get(&user_id) {
                                    for socket in sockets.values() {
                                        socket.try_send(cam.serialized.clone());
                                    }
                                } else {
                                    println!("Couldn't find socket for {user_id}");
                                }
                            }
                        };
                    }
                };
                let actor_future = future.into_actor(self);
                ctx.wait(actor_future);
            }
        }
    }
}
