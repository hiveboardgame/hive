use super::messages::GameHB;
use super::WebsocketData;
use super::{messages::MessageDestination, messages::Ping};
use crate::{
    common::{
        ChallengeUpdate, GameUpdate, ScheduleUpdate, ServerMessage, ServerResult, TournamentUpdate,
        UserStatus, UserUpdate,
    },
    responses::{
        ChallengeResponse, GameResponse, HeartbeatResponse, ScheduleResponse, TournamentResponse,
        UserResponse,
    },
    websocket::messages::{ClientActorMessage, Connect, Disconnect, WsMessage},
};
use actix::{
    prelude::{Actor, Context, Handler, Recipient},
    AsyncContext, WrapFuture,
};
use codee::binary::MsgpackSerdeCodec;
use codee::Encoder;
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
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug)]
pub struct WsServer {
    id: String,
    sessions: HashMap<Uuid, Vec<Recipient<WsMessage>>>, // user_id to (socket_)id
    games_users: HashMap<GameId, HashSet<Uuid>>,        // game_id to set of users
    users_games: HashMap<Uuid, HashSet<String>>,        // user_id to set of games
    data: Arc<WebsocketData>,
    pool: DbPool,
}

impl WsServer {
    pub fn new(data: Arc<WebsocketData>, pool: DbPool) -> WsServer {
        WsServer {
            // TODO: work on replacing the id here...
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
    fn send_message(&self, message: &Vec<u8>, id_to: &Uuid) {
        if let Some(sockets) = self.sessions.get(id_to) {
            for socket in sockets {
                socket.do_send(WsMessage(message.to_owned()));
            }
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
        for user_id in self.sessions.keys() {
            let nonce = rng.random::<u64>();
            self.data.pings.set_nonce(*user_id, nonce);
            let message = ServerResult::Ok(Box::new(ServerMessage::Ping {
                nonce,
                value: self.data.pings.value(*user_id),
            }));
            if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                self.send_message(&serialized, user_id);
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
                                            for socket in sockets {
                                                socket.do_send(WsMessage(serialized.clone()));
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
        // Remove the WS connection from the user sessions
        if let Some(user_sessions) = self.sessions.get_mut(&msg.user_id) {
            user_sessions.retain(|session| *session != msg.addr);
            if user_sessions.is_empty() {
                self.sessions.remove(&msg.user_id);
            }
        }
        // If that was the last WS connection for that user
        if !self.sessions.contains_key(&msg.user_id) {
            if let Some(games) = self.users_games.remove(&msg.user_id) {
                for game in games.iter() {
                    let game_id = GameId(game.to_string());
                    if let Some(game_users) = self.games_users.get_mut(&game_id) {
                        if game_users.len() > 1 {
                            game_users.remove(&msg.user_id);
                        } else {
                            //only one in the game, remove it entirely
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
                if let Some(ws_server) = self.games_users.get(&game_id) {
                    ws_server
                        .iter()
                        .for_each(|user_id| self.send_message(&serialized, user_id));
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
            .push(msg.addr.clone());
        let pool = self.pool.clone();
        let address = ctx.address().clone();
        let games_users = self.games_users.clone();
        let sessions = self.sessions.clone();
        let future = async move {
            if let Ok(mut conn) = get_conn(&pool).await {
                //Get currently online users
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
                            // TODO: one needs to be a game::join to everyone in the game, the other one just to the
                            // ws_server that the user came online
                            if let Some(user_ids) = games_users.get(&GameId(msg.game_id)) {
                                for id in user_ids {
                                    if let Some(sockets) = sessions.get(id) {
                                        for socket in sockets {
                                            socket.do_send(WsMessage(serialized.clone()));
                                        }
                                    }
                                }
                            }
                        };
                    }

                    // Send games which require input from the user
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
                    // send tournament invitations
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

                    // send schedule notifications
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

                    // Send challenges on join
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
                socket.do_send(WsMessage(cam.serialized));
            }
            MessageDestination::Global => {
                // Make sure the user is in the game:
                if let Some(from) = cam.from {
                    self.games_users
                        .entry(GameId(self.id.clone()))
                        .or_default()
                        .insert(from);
                }
                // Send the message to everyone
                if let Some(users) = self.games_users.get(&GameId(self.id.clone())) {
                    users
                        .iter()
                        .for_each(|client| self.send_message(&cam.serialized, client));
                } else {
                    warn!(
                        "Game '{}' not found in games_users when sending global message",
                        self.id
                    );
                }
            }
            MessageDestination::Game(ref game_id) => {
                // Make sure the user is in the game:
                if let Some(from) = cam.from {
                    self.games_users
                        .entry(game_id.clone())
                        .or_default()
                        .insert(from);
                }
                // Send the message to everyone
                if let Some(users) = self.games_users.get(game_id) {
                    users
                        .iter()
                        .for_each(|client| self.send_message(&cam.serialized, client));
                } else {
                    warn!("Game '{game_id}' not found in games_users when sending game message");
                }
            }
            MessageDestination::GameSpectators(game_id, white_id, black_id) => {
                // Make sure the user is in the game:
                if let Some(from) = cam.from {
                    self.games_users
                        .entry(game_id.clone())
                        .or_default()
                        .insert(from);
                }
                // Send the message to everyone except white_id and black_id
                if let Some(users) = self.games_users.get(&game_id) {
                    users.iter().for_each(|user| {
                        if *user != white_id && *user != black_id {
                            self.send_message(&cam.serialized, user);
                        }
                    });
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
                            for user_id in user_ids.clone() {
                                if let Some(sockets) = sessions.get(&user_id) {
                                    for socket in sockets {
                                        socket.do_send(WsMessage(cam.serialized.clone()));
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
