use crate::common::client_message::ClientMessage;
use crate::common::game_action::GameAction;
use crate::common::server_result::GameActionResponse;
use crate::common::server_result::{ExternalServerError, ServerOk::GameUpdate, ServerResult};
use crate::functions::games::game_response::GameStateResponse;
use crate::websockets::server_error::ServerError;
use crate::websockets::{
    lobby::Lobby,
    messages::{ClientActorMessage, Connect, Disconnect, WsMessage},
};
use actix::{
    fut, Actor, ActorContext, ActorFutureExt, Addr, AsyncContext, ContextFutureSpawner, Handler,
    Running, StreamHandler, WrapFuture,
};
use actix_web_actors::ws::{self, Message::Text};
use db_lib::{models::game::Game, DbPool};
use hive_lib::{game_control::GameControl, game_status::GameStatus, state::State, turn::Turn};
use std::str::FromStr;
use std::time::{Duration, Instant};
use uuid::Uuid;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct WsConnection {
    user_uid: Uuid,
    username: String,
    #[allow(dead_code)]
    authed: bool,
    lobby_addr: Addr<Lobby>,
    hb: Instant, // websocket heartbeat
    pool: DbPool,
}

impl Actor for WsConnection {
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

impl WsConnection {
    pub fn new(
        user_uid: Option<Uuid>,
        username: String,
        lobby: Addr<Lobby>,
        pool: DbPool,
    ) -> WsConnection {
        WsConnection {
            user_uid: user_uid.unwrap_or(Uuid::new_v4()),
            username,
            authed: user_uid.is_some(),
            hb: Instant::now(),
            lobby_addr: lobby,
            pool,
        }
    }

    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }
            ctx.ping(b"hi");
        });
    }

    fn users_turn(game: &Game, user_id: Uuid, username: &str) -> Result<(), ServerError> {
        if !((game.turn % 2 == 0 && game.white_id == user_id)
            || (game.turn % 2 == 1 && game.black_id == user_id))
        {
            return Err(ServerError::UserInputError {
                field: format!(
                    "{username} can't play on {} at turn {}",
                    game.nanoid, game.turn
                ),
                reason: "It is not their turn".to_string(),
            });
        }
        Ok(())
    }

    fn not_finished(game: &Game, username: &str) -> Result<(), ServerError> {
        if let GameStatus::Finished(_) = GameStatus::from_str(&game.game_status).unwrap() {
            return Err(ServerError::UserInputError {
                field: format!("{username} can't play on {}", game.nanoid),
                reason: "Game is over".to_string(),
            });
        }
        Ok(())
    }

    fn ensure_previous_gc_present(
        game: &Game,
        current_game_control: &GameControl,
    ) -> Result<(), ServerError> {
        let opposite_color = current_game_control.color().opposite_color();
        let should_be_gc = match current_game_control {
            GameControl::TakebackAccept(_) => GameControl::TakebackRequest(opposite_color),
            GameControl::TakebackReject(_) => GameControl::TakebackRequest(opposite_color),
            GameControl::DrawReject(_) => GameControl::DrawOffer(opposite_color),
            GameControl::DrawAccept(_) => GameControl::DrawOffer(opposite_color),
            _ => unreachable!(),
        };
        if let Some(last_gc) = game.last_game_control() {
            if last_gc == should_be_gc {
                return Ok(());
            }
        }
        Err(ServerError::UserInputError {
            field: format!("{current_game_control}"),
            reason: "Not allowed".to_string(),
        })?
    }

    async fn handle_takeback_request(
        game_control: &GameControl,
        game: &Game,
        pool: &DbPool,
    ) -> Result<Game, ServerError> {
        let game = game.write_game_control(game_control, pool).await?;
        Ok(game)
    }

    async fn handle_takeback_accept(
        game_control: &GameControl,
        game: &Game,
        pool: &DbPool,
    ) -> Result<Game, ServerError> {
        Self::ensure_previous_gc_present(game, game_control)?;
        let game = game.accept_takeback(game_control, pool).await?;
        Ok(game)
    }

    async fn handle_resign(
        game_control: &GameControl,
        game: &Game,
        pool: &DbPool,
    ) -> Result<Game, ServerError> {
        let game = game.resign(game_control, pool).await?;
        Ok(game)
    }

    async fn handle_abort(game: &Game, pool: &DbPool) -> Result<(), ServerError> {
        Ok(game.delete(pool).await?)
    }

    async fn handle_draw_reject(
        game_control: &GameControl,
        game: &Game,
        pool: &DbPool,
    ) -> Result<Game, ServerError> {
        Self::ensure_previous_gc_present(game, game_control)?;
        let game = game.write_game_control(game_control, pool).await?;
        Ok(game)
    }

    async fn handle_draw_offer(
        game_control: &GameControl,
        game: &Game,
        pool: &DbPool,
    ) -> Result<Game, ServerError> {
        let game = game.write_game_control(game_control, pool).await?;
        Ok(game)
    }

    async fn handle_draw_accept(
        game_control: &GameControl,
        game: &Game,
        pool: &DbPool,
    ) -> Result<Game, ServerError> {
        Self::ensure_previous_gc_present(game, game_control)?;
        let game = game.accept_draw(game_control, pool).await?;
        Ok(game)
    }

    async fn handle_takeback_reject(
        game_control: &GameControl,
        game: &Game,
        pool: &DbPool,
    ) -> Result<Game, ServerError> {
        Self::ensure_previous_gc_present(game, game_control)?;
        let game = game.write_game_control(game_control, pool).await?;
        Ok(game)
    }

    async fn match_control(
        game_control: &GameControl,
        game: &Game,
        pool: &DbPool,
    ) -> Result<Game, ServerError> {
        Ok(match game_control {
            GameControl::Abort(_) => {
                let game = game.clone();
                Self::handle_abort(&game, pool).await?;
                game
            }
            GameControl::Resign(_) => Self::handle_resign(game_control, &game, pool).await?,
            GameControl::DrawOffer(_) => Self::handle_draw_offer(game_control, &game, pool).await?,
            GameControl::DrawAccept(_) => {
                Self::handle_draw_accept(game_control, &game, pool).await?
            }
            GameControl::DrawReject(_) => {
                Self::handle_draw_reject(game_control, &game, pool).await?
            }
            GameControl::TakebackRequest(_) => {
                Self::handle_takeback_request(game_control, &game, pool).await?
            }
            GameControl::TakebackAccept(_) => {
                Self::handle_takeback_accept(game_control, &game, pool).await?
            }
            GameControl::TakebackReject(_) => {
                Self::handle_takeback_reject(game_control, &game, pool).await?
            }
        })
    }

    fn ensure_gc_allowed_for_turn(control: &GameControl, turn: i32) -> Result<(), ServerError> {
        if turn == 0 {
            Err(ServerError::UserInputError {
                field: format!("{control}"),
                reason: "Not not allowed on turn 0".to_string(),
            })?
        }
        Ok(())
    }

    fn ensure_gc_color(
        user_id: Uuid,
        game: &Game,
        control: &GameControl,
    ) -> Result<(), ServerError> {
        if let Some(color) = game.user_color(user_id) {
            if color == control.color() {
                return Ok(());
            }
        }
        Err(ServerError::UserInputError {
            field: format!("{}", control.color()),
            reason: format!("Cannot play for {}", control.color().opposite_color()),
        })?
    }

    fn ensure_user_is_player(user_id: Uuid, game: &Game) -> Result<(), ServerError> {
        if !game.user_is_player(user_id) {
            Err(ServerError::UserInputError {
                field: format!("{user_id}"),
                reason: "Is not a player at the game".to_string(),
            })?
        }
        Ok(())
    }

    async fn handle_control(
        control: GameControl,
        game_id: &str,
        user_id: Uuid,
        username: &str,
        pool: &DbPool,
    ) -> Result<GameActionResponse, ServerError> {
        let game = Game::find_by_nanoid(&game_id, &pool).await?;
        // make sure the game hasn't finished
        WsConnection::not_finished(&game, username)?;
        // checks: user is player at the game
        Self::ensure_user_is_player(user_id, &game)?;
        // the GC can be played this turn
        Self::ensure_gc_allowed_for_turn(&control, game.turn)?;
        // the GC(color) matches the user color
        Self::ensure_gc_color(user_id, &game, &control)?;
        let game = Self::match_control(&control, &game, pool).await?;
        // TODO: this error needs to be fixed
        Ok(GameActionResponse {
            game_id: game_id.to_owned(),
            game: GameStateResponse::new_from_db(&game, pool)
                .await
                .map_err(|_| ServerError::GenericError {
                    reason: "foo".to_string(),
                })?,
            game_action: GameAction::Control(control),
            user_id,
            username: username.to_owned(),
        })
    }

    async fn handle_join(
        game_id: &str,
        user_id: Uuid,
        username: &str,
        pool: &DbPool,
    ) -> Result<GameActionResponse, ServerError> {
        let game = Game::find_by_nanoid(&game_id, &pool).await?;
        Ok(GameActionResponse {
            game_id: game_id.to_owned(),
            game: GameStateResponse::new_from_db(&game, pool)
                .await
                .map_err(|_| ServerError::GenericError {
                    reason: "foo".to_string(),
                })?,
            game_action: GameAction::Join,
            user_id,
            username: username.to_owned(),
        })
    }

    async fn handle_move(
        turn: Turn,
        game_id: &str,
        user_id: Uuid,
        username: &str,
        pool: &DbPool,
    ) -> Result<GameActionResponse, ServerError> {
        let mut game = Game::find_by_nanoid(&game_id, &pool).await?;
        WsConnection::not_finished(&game, username)?;
        WsConnection::users_turn(&game, user_id, username)?;
        let (piece, position) = match turn {
            Turn::Move(piece, position) => (piece, position),
            Turn::Spawn(piece, position) => (piece, position),
            _ => unreachable!(),
        };

        let mut state = State::new_from_str(&game.history, &game.game_type)?;
        let current_turn = state.turn;
        state.play_turn_from_position(piece, position)?;
        let (piece, pos) = state
            .history
            .moves
            .get(current_turn)
            .expect("No moves in history after a move has been played.");
        // TODO: @leex making 2 DB inserts is a bit ugly, maybe we should have:
        // make_move and make_moves?
        game = game
            .make_move(format!("{piece} {pos}"), state.game_status.clone(), &pool)
            .await?;
        if state
            .history
            .moves
            .last()
            .expect("There needs to be a move here")
            .0
            == "pass"
        {
            game = game
                .make_move(String::from("pass "), state.game_status.clone(), &pool)
                .await?;
        }
        Ok(GameActionResponse {
            game_id: game_id.to_owned(),
            game: GameStateResponse::new_from_db(&game, pool)
                .await
                .map_err(|_| ServerError::GenericError {
                    reason: "foo".to_string(),
                })?,
            game_action: GameAction::Move(turn),
            user_id,
            username: username.to_owned(),
        })
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConnection {
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
                let future = async move {
                    let action_result = match m.game_action.clone() {
                        GameAction::Move(turn) => {
                            println!("Handling move");
                            WsConnection::handle_move(turn, &game_id, user_id, &username, &pool)
                                .await
                        }
                        GameAction::Join => {
                            println!("Got join");
                            lobby.do_send(Connect {
                                addr: addr.recipient(),
                                game_id: game_id.clone(),
                                user_id,
                                username: username.clone(),
                            });
                            WsConnection::handle_join(&game_id, user_id, &username, &pool).await
                        }
                        GameAction::Control(control) => {
                            // TODO: Check for authed
                            println!("Got GameControl {:?}", control);
                            WsConnection::handle_control(
                                control, &game_id, user_id, &username, &pool,
                            )
                            .await
                        }
                        _ => unimplemented!(),
                        // GameAction::Chat(msg) => ClientActorMessage::new(
                        //     &game_id, "foo", // GameAction::Chat(msg),
                        //     user_id,
                        // )
                        // .await
                        // .expect("Failed to construct ClientActorMessage"),
                        // GameAction::Error(msg) => ClientActorMessage::new(
                        //     &game_id, "foo", //GameAction::Error(msg),
                        //     user_id,
                        // )
                        // .await
                        // .expect("Failed to construct ClientActorMessage"),
                    };
                    let server_result = match action_result {
                        Err(err) => ServerResult::Err(ExternalServerError {
                            field: format!("{}", m.game_action),
                            reason: format!("{err}"),
                            status_code: err.status_code(),
                        }),
                        Ok(gar) => ServerResult::Ok(GameUpdate(gar)),
                    };
                    let serialized = serde_json::to_string(&server_result).unwrap();
                    let cam = ClientActorMessage::new(&game_id, &serialized, user_id);
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

impl Handler<WsMessage> for WsConnection {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}
