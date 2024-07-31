use super::tournament_game_start::TournamentGameStart;
use super::{api::request_handler::RequestHandler, internal_server_message::MessageDestination};
use crate::common::{CommonMessage, ExternalServerError, ServerResult};
use crate::lag_tracking::lags::Lags;
use crate::ping::pings::Pings;
use crate::websockets::{
    chat::Chats,
    messages::{ClientActorMessage, Connect, Disconnect, WsMessage},
    ws_server::WsServer,
};
use actix::{
    fut, Actor, ActorContext, ActorFutureExt, Addr, AsyncContext, ContextFutureSpawner, Handler,
    Running, StreamHandler, WrapFuture,
};
use actix_web_actors::ws::{self};
use anyhow::Result;
use codee::{binary::MsgpackSerdeCodec, Decoder, Encoder};
use db_lib::DbPool;
use shared_types::SimpleUser;
use std::time::{Duration, Instant};
use uuid::Uuid;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct WsConnection {
    user_uid: Uuid,
    username: String,
    authed: bool,
    admin: bool,
    chat_storage: actix_web::web::Data<Chats>,
    game_start: actix_web::web::Data<TournamentGameStart>,
    pings: actix_web::web::Data<Pings>,
    lags: actix_web::web::Data<Lags>,
    wss_addr: Addr<WsServer>,
    hb: Instant,
    pool: DbPool,
}

impl Actor for WsConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
        let addr = ctx.address();
        self.wss_addr
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

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        self.wss_addr.do_send(Disconnect {
            user_id: self.user_uid,
            game_id: String::from("lobby"),
            addr: ctx.address().recipient(),
            username: self.username.clone(),
        });
        Running::Stop
    }
}

impl WsConnection {
    pub fn new(
        user_uid: Option<Uuid>,
        username: Option<String>,
        admin: Option<bool>,
        lobby: Addr<WsServer>,
        chat_storage: actix_web::web::Data<Chats>,
        game_start: actix_web::web::Data<TournamentGameStart>,
        pings: actix_web::web::Data<Pings>,
        lags: actix_web::web::Data<Lags>,
        pool: DbPool,
    ) -> WsConnection {
        let id = user_uid.unwrap_or(Uuid::new_v4());
        let name = username.unwrap_or(id.to_string());
        let admin = admin.unwrap_or_default();
        WsConnection {
            user_uid: id,
            username: name,
            admin,
            game_start,
            pings,
            lags,
            authed: user_uid.is_some(),
            chat_storage,
            hb: Instant::now(),
            wss_addr: lobby,
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
            Ok(ws::Message::Text(bin)) => {
                println!("Got text message, we don't do these here...");
                ctx.text(bin)
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Continuation(_)) => {
                ctx.stop();
            }
            Ok(ws::Message::Nop) => (),
            Ok(ws::Message::Binary(s)) => {
                let request: Result<CommonMessage, _> = MsgpackSerdeCodec::decode(&s);
                if let Ok(CommonMessage::Client(request)) = request {
                    let pool = self.pool.clone();
                    let lobby = self.wss_addr.clone();
                    let user_id = self.user_uid;
                    let username = self.username.clone();
                    let user = SimpleUser {
                        user_id,
                        username: username.clone(),
                        authed: self.authed,
                        admin: self.admin,
                    };
                    let chat_storage = self.chat_storage.clone();
                    let pings = self.pings.clone();
                    let lags = self.lags.clone();
                    let game_start = self.game_start.clone();
                    let addr = ctx.address().recipient();

                    let future = async move {
                        let handler = RequestHandler::new(
                            request.clone(),
                            chat_storage,
                            game_start,
                            pings,
                            lags,
                            addr,
                            user,
                            pool,
                        );
                        let handler_result = handler.handle().await;
                        match handler_result {
                            Ok(messages) => {
                                for message in messages {
                                    let serialized = CommonMessage::Server(ServerResult::Ok(
                                        Box::new(message.message),
                                    ));
                                    let serialized = MsgpackSerdeCodec::encode(&serialized)
                                        .expect("Failed to serialize a server message");
                                    let cam = ClientActorMessage {
                                        destination: message.destination,
                                        serialized,
                                        from: Some(user_id),
                                    };
                                    lobby.do_send(cam);
                                }
                            }
                            Err(err) => {
                                // TODO: @leex the error here needs to be nicer
                                println!(
                                    "---------------------------------------\n
                                                ERROR\n
                                Request:\n  {request:?}\n
                                Error:\n  {err:?}\n
                                User:\n  {username} {user_id}\n
                                ---------------------------------------",
                                );
                                let message = ServerResult::Err(ExternalServerError {
                                    user_id,
                                    field: "foo".to_string(),
                                    reason: format!("{err}"),
                                    status_code: http::StatusCode::NOT_IMPLEMENTED,
                                });
                                let serialized = CommonMessage::Server(message);
                                let serialized = MsgpackSerdeCodec::encode(&serialized)
                                    .expect("Failed to serialize a server message");
                                let cam = ClientActorMessage {
                                    destination: MessageDestination::User(user_id),
                                    serialized,
                                    from: Some(user_id),
                                };
                                lobby.do_send(cam);
                            }
                        }
                    };

                    let actor_future = future.into_actor(self);
                    ctx.wait(actor_future);
                }
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
        ctx.binary(msg.0);
    }
}
