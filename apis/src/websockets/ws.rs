use cfg_if::cfg_if;

cfg_if! { if #[cfg(feature = "ssr")] {

use crate::common::client_message::ClientMessage;
use crate::common::game_action::GameAction;
use crate::websockets::{
    lobby::Lobby,
    messages::{ClientActorMessage, Connect, Disconnect, WsMessage},
};
use actix::{fut, ActorContext, ActorFutureExt, ContextFutureSpawner, WrapFuture};
use actix::{Actor, Addr, Running, StreamHandler};
use actix::{AsyncContext, Handler};
use actix_web_actors::ws;
use actix_web_actors::ws::Message::Text;
use db_lib::{models::game::Game, DbPool};
use hive_lib::game_type::GameType;
use hive_lib::{game_status::GameStatus, history::History, state::State, turn::Turn};
use std::str::FromStr;
use std::time::{Duration, Instant};
use uuid::Uuid;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct WsConn {
    user_uid: Uuid,
    username: String,
    authed: bool,
    lobby_addr: Addr<Lobby>,
    hb: Instant, // websocket heartbeat
    pool: DbPool,
}

impl WsConn {
    pub fn new(
        user_uid: Option<Uuid>,
        username: String,
        lobby: Addr<Lobby>,
        pool: DbPool,
    ) -> WsConn {
        WsConn {
            user_uid: user_uid.unwrap_or(Uuid::new_v4()),
            username,
            authed: user_uid.is_some(),
            hb: Instant::now(),
            lobby_addr: lobby,
            pool,
        }
    }
}

impl Actor for WsConn {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
        let addr = ctx.address();
        self.lobby_addr
            .send(Connect {
                addr: addr.recipient(),
                game_id: String::from("lobby"), // self.game_id
                user_id: self.user_uid,
                username: self.username.clone(),
            })
            .into_actor(self)
            .then(|res, _, ctx| {
                match res {
                    Ok(_res) => (),
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.lobby_addr.do_send(Disconnect {
            user_id: self.user_uid,
            game_id: String::from("lobby"),
            username: self.username.clone(),
        });
        Running::Stop
    }
}

impl WsConn {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }
            ctx.ping(b"hi");
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConn {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        // let game_id = self.game.clone();
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Binary(bin)) => {
                println!("Got bin message is {:?}", bin);
                ctx.binary(bin)
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Continuation(_)) => {
                ctx.stop();
            }
            Ok(ws::Message::Nop) => (),
            Ok(Text(s)) => {
                println!("WS message is {:?}", s);
                let m: ClientMessage = serde_json::from_str(&s.to_string()).unwrap();
                let addr = ctx.address();
                let game_id = m.game_id;
                let pool = self.pool.clone();
                let lobby = self.lobby_addr.clone();
                let user_id = self.user_uid.clone();
                let username = self.username.clone();
                // TODO: @leex you hate this code and know what to do make non self functions and
                // move code into WsConn::non_self_fn's
                let future = async move {
                    let cam = match m.game_action {
                        GameAction::Move(turn) => {
                            let game = Game::find_by_nanoid(&game_id, &pool).await.unwrap();
                            let cam = if !((game.turn % 2 == 0 && game.white_id == user_id)
                                || (game.turn % 2 == 1 && game.black_id == user_id))
                            {
                                Some(
                                    ClientActorMessage::new(
                                        GameAction::Error(format!(
                                            "{username} is not allowed to play"
                                        )),
                                        &game_id,
                                        user_id,
                                        &username,
                                        &pool,
                                    )
                                    .await
                                    .expect("Failed to construct ClientActorMessage"),
                                )
                            } else {
                                if let GameStatus::Finished(_) =
                                    GameStatus::from_str(&game.game_status).unwrap()
                                {
                                    Some(
                                        ClientActorMessage::new(
                                            GameAction::Error(format!("Can't play on a finished game")),
                                            &game_id,
                                            user_id,
                                            &username,
                                            &pool,
                                        )
                                        .await
                                        .expect("Failed to construct ClientActorMessage"),
                                    )
                                } else {
                                    None
                                }
                            };
                            // TODO: the unwraps...
                            println!("Playing move {:?}", turn);
                            match cam {
                                None => {
                                    let (piece, position) = match turn {
                                        Turn::Move(piece, position) => (piece, position),
                                        Turn::Spawn(piece, position) => (piece, position),
                                        _ => unreachable!(),
                                    };
                                    let history = History::new_from_str(game.history.clone()).unwrap();
                                    let mut state = State::new_from_history(&history).unwrap();
                                    state.game_type = GameType::from_str(&game.game_type).unwrap();
                                    let current_turn = state.turn;
                                    state.play_turn_from_position(piece, position).unwrap();
                                    let (piece, pos) = state.history.moves.get(current_turn).unwrap();
                                    let board_move = format!("{piece} {pos}");
                                    game.make_move(format!("{piece} {pos}"), state.game_status.clone(), &pool)
                                        .await
                                        .unwrap();
                                    if "pass" == state.history.moves.last().expect("There needs to be a move here").0 {
                                        game.make_move(String::from("pass "), state.game_status.clone(), &pool)
                                            .await
                                            .unwrap();
                                    }
                                    if let Some(error_cam) = cam {
                                        error_cam
                                    } else {
                                        ClientActorMessage::new(
                                            GameAction::Move(turn),
                                            &game_id,
                                            user_id,
                                            &username,
                                            &pool,
                                        )
                                        .await
                                        .expect("Failed to construct ClientActorMessage")
                                    }
                                },
                                Some(cam) => cam,
                            }
                        }
                        GameAction::Control(control) => {
                            //   - get the game from the db
                            //   - control the game
                            //   - send message back with result
                            println!("Got GameControl {:?}", control);
                            ClientActorMessage::new(
                                GameAction::Control(control),
                                &game_id,
                                user_id,
                                &username,
                                &pool,
                            )
                            .await
                            .expect("Failed to construct ClientActorMessage")
                        }
                        GameAction::Join => {
                            println!("Got join");
                            lobby.do_send(Connect {
                                addr: addr.recipient(),
                                game_id: game_id.clone(),
                                user_id,
                                username: username.clone(),
                            });
                            ClientActorMessage::new(
                                GameAction::Join,
                                &game_id,
                                user_id,
                                &username,
                                &pool,
                            )
                            .await
                            .expect("Failed to construct ClientActorMessage")
                        }
                        GameAction::Chat(msg) => ClientActorMessage::new(
                            GameAction::Chat(msg),
                            &game_id,
                            user_id,
                            &username,
                            &pool,
                        )
                        .await
                        .expect("Failed to construct ClientActorMessage"),
                        GameAction::Error(msg) => ClientActorMessage::new(
                            GameAction::Error(msg),
                            &game_id,
                            user_id,
                            &username,
                            &pool,
                        )
                        .await
                        .expect("Failed to construct ClientActorMessage"),
                    };
                    // if let GameAction::Error(error) = cam.game_action {
                    //     println!("Server got an error message, what's up with that? {error}");
                    // };
                    lobby.do_send(cam);
                };
                let actor_future = future.into_actor(self);
                ctx.wait(actor_future);
            }
            Err(e) => {
                println!("Got error in WS parsing");
                std::panic::panic_any(e)
            }
        }
    }
}

impl Handler<WsMessage> for WsConn {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}
}}
