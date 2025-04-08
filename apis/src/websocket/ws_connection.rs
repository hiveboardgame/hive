use super::WebsocketData;
use super::{messages::MessageDestination, server_handlers::request_handler::RequestHandler};
use crate::common::{ClientRequest, ExternalServerError, ServerResult};
use crate::websocket::server_handlers::request_handler::RequestHandlerError;
use crate::websocket::{
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
use indoc::printdoc;
use shared_types::SimpleUser;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct WsConnection {
    user_uid: Uuid,
    username: String,
    authed: bool,
    admin: bool,
    data: Arc<WebsocketData>,
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
        data: Arc<WebsocketData>,
        pool: DbPool,
    ) -> WsConnection {
        let id = user_uid.unwrap_or(Uuid::new_v4());
        let name = username.unwrap_or(id.to_string());
        let admin = admin.unwrap_or_default();
        WsConnection {
            user_uid: id,
            username: name,
            admin,
            data: data.clone(),
            authed: user_uid.is_some(),
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
                let request: Result<ClientRequest, _> = MsgpackSerdeCodec::decode(&s);
                if let Ok(request) = request {
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
                    let addr = ctx.address().recipient();
                    let data = Arc::clone(&self.data);
                    let future = async move {
                        let handler = RequestHandler::new(request.clone(), data, addr, user, pool);
                        let handler_result = handler.handle().await;
                        match handler_result {
                            Ok(messages) => {
                                for message in messages {
                                    let serialized = ServerResult::Ok(Box::new(message.message));
                                    if let Ok(serialized) = MsgpackSerdeCodec::encode(&serialized) {
                                        let cam = ClientActorMessage {
                                            destination: message.destination,
                                            serialized,
                                            from: Some(user_id),
                                        };
                                        lobby.do_send(cam);
                                    };
                                }
                            }
                            Err(err) => {
                                let status_code = match err {
                                    RequestHandlerError::AuthError(_) => {
                                        http::StatusCode::UNAUTHORIZED
                                    }
                                    _ => http::StatusCode::NOT_IMPLEMENTED,
                                };
                                printdoc! {r#"
                                    -----------------ERROR-----------------
                                      Request: {:?}
                                      Error:   {:?}
                                      User:    {} {}
                                    ------------------END------------------
                                    "#,
                                    request, err, username, user_id
                                };
                                let message = ServerResult::Err(ExternalServerError {
                                    user_id,
                                    field: "foo".to_string(),
                                    reason: format!("{err}"),
                                    status_code,
                                });
                                if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                                    let cam = ClientActorMessage {
                                        destination: MessageDestination::User(user_id),
                                        serialized,
                                        from: Some(user_id),
                                    };
                                    lobby.do_send(cam);
                                };
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
