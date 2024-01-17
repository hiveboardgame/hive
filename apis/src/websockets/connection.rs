use crate::{
    common::{
        client_message::ClientRequest,
        server_result::{ExternalServerError, MessageDestination, ServerResult},
    },
    websockets::{
        lobby::Lobby,
        messages::{ClientActorMessage, Connect, Disconnect, WsMessage},
        request_handler::RequestHandler,
    },
};
use actix::{
    fut, Actor, ActorContext, ActorFutureExt, Addr, AsyncContext, ContextFutureSpawner, Handler,
    Running, StreamHandler, WrapFuture,
};
use actix_web_actors::ws::{self, Message::Text};
use anyhow::Result;
use db_lib::DbPool;
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
                let request: ClientRequest =
                    serde_json::from_str(s.as_ref()).expect("ClientMessage from string worked");
                let pool = self.pool.clone();
                let lobby = self.lobby_addr.clone();
                let user_id = self.user_uid;
                let username = self.username.clone();
                let authed = self.authed;

                let future = async move {
                    let handler = RequestHandler::new(request, user_id, &username, authed, pool);
                    let handler_result = handler.handle().await;
                    match handler_result {
                        Ok(messages) => {
                            for message in messages {
                                let serialized =
                                    serde_json::to_string(&ServerResult::Ok(message.message))
                                        .expect("Failed to serialize a server message");
                                let cam = ClientActorMessage {
                                    destination: message.destination,
                                    serialized,
                                    from: user_id,
                                };
                                lobby.do_send(cam);
                            }
                        }
                        Err(err) => {
                            // TODO: @leex the error here needs to be nicer
                            let message = ServerResult::Err(ExternalServerError {
                                user_id,
                                field: "foo".to_string(),
                                reason: format!("{err}"),
                                status_code: http::StatusCode::NOT_IMPLEMENTED,
                            });
                            let serialized = serde_json::to_string(&message)
                                .expect("Failed to serialize a server message");
                            let cam = ClientActorMessage {
                                destination: MessageDestination::Direct(user_id),
                                serialized,
                                from: user_id,
                            };
                            lobby.do_send(cam);
                        }
                    }
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
