use super::messages::GameHB;
use super::messages::MessageDestination;
use crate::{
    common::{GameUpdate, ServerMessage, ServerResult},
    responses::HeartbeatResponse,
    websocket::messages::{ClientActorMessage, WsMessage},
};
use actix::{
    prelude::{Actor, Context, Handler, Recipient},
    AsyncContext, WrapFuture,
};
use codee::binary::MsgpackSerdeCodec;
use codee::Encoder;
use db_lib::{
    get_conn,
    models::{Game, Tournament},
    DbPool,
};
use hive_lib::GameStatus;
use log::warn;
use shared_types::{GameId, TimeMode};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug)]
pub struct WsServer {
    id: String,
    sessions: HashMap<Uuid, Vec<Recipient<WsMessage>>>, // user_id to (socket_)id
    games_users: HashMap<GameId, HashSet<Uuid>>,        // game_id to set of users
    pool: DbPool,
}

impl WsServer {
    pub fn new(pool: DbPool) -> WsServer {
        WsServer {
            // TODO: work on replacing the id here...
            id: String::from("lobby"),
            sessions: HashMap::new(),
            games_users: HashMap::new(),
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
