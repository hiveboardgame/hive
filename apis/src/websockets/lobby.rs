use crate::websockets::messages::{ClientActorMessage, Connect, Disconnect, WsMessage};
use actix::prelude::{Actor, Context, Handler, Recipient};
use actix::AsyncContext;
use actix::{fut, ActorContext, ActorFutureExt, ContextFutureSpawner, WrapFuture};
use db_lib::{models::game::Game, DbPool};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

type Socket = Recipient<WsMessage>;

#[derive(Debug)]
pub struct Lobby {
    #[allow(dead_code)]
    id: String,
    sessions: HashMap<Uuid, Socket>,       // user_id to (socket_)id
    games: HashMap<String, HashSet<Uuid>>, // game_id to set of users
    pool: DbPool,
}

impl Lobby {
    pub fn new(pool: DbPool) -> Lobby {
        Lobby {
            id: String::from("lobby"),
            sessions: HashMap::new(),
            games: HashMap::new(),
            pool,
        }
    }
}

impl Lobby {
    fn send_message(&self, message: &str, id_to: &Uuid) {
        if let Some(socket_recipient) = self.sessions.get(id_to) {
            socket_recipient.do_send(WsMessage(message.to_owned()));
        } else {
            println!("attempting to send message but couldn't find user id.");
        }
    }
}

impl Actor for Lobby {
    type Context = Context<Self>;
}

impl Handler<Disconnect> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        if self.sessions.remove(&msg.user_id).is_some() {
            println!("Client {} disconnected", msg.user_id);
            self.games
                .get(&msg.game_id)
                .unwrap()
                .iter()
                .filter(|conn_id| *conn_id.to_owned() != msg.user_id)
                .for_each(|user_id| {
                    self.send_message(&format!("{} disconnected.", &msg.user_id), user_id)
                });
            if let Some(game) = self.games.get_mut(&msg.game_id) {
                if game.len() > 1 {
                    game.remove(&msg.user_id);
                } else {
                    //only one in the game, remove it entirely
                    self.games.remove(&msg.game_id);
                }
            }
        }
    }
}

impl Handler<Connect> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: Connect, ctx: &mut Context<Self>) -> Self::Result {
        println!("Client {} connected", msg.user_id);
        self.games
            .entry(msg.game_id.clone())
            .or_default()
            .insert(msg.user_id);
        self.games
            .get(&msg.game_id)
            .unwrap()
            .iter()
            .filter(|conn_id| *conn_id.to_owned() != msg.user_id)
            .for_each(|conn_id| {
                self.send_message(&format!("{} just joined!", msg.user_id), conn_id)
            });
        self.sessions.insert(msg.user_id, msg.addr.clone());

        // TODO: send the gamestate to the newly joined user
        if msg.game_id == "lobby" {
            self.send_message(&format!("You joined {}", msg.game_id), &msg.user_id);
            return ();
        }
        let pool = self.pool.clone();
        let addr = msg.addr.clone();
        let future = async move {
            let game: Game = Game::find_by_nanoid(&msg.game_id, &pool)
                .await
                .expect("Could not find game");
            addr.do_send(WsMessage(format!("You joined {:?}", game)));
        };
        let actor_future = future.into_actor(self);
        ctx.wait(actor_future);
        //println!("Lobby is {:?}", self);
    }
}

impl Handler<ClientActorMessage> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: ClientActorMessage, _ctx: &mut Context<Self>) -> Self::Result {
        println!(
            "Got message {:?} in {:?} with users: {:?}",
            &msg.msg,
            &msg.game_id,
            self.games.get(&msg.game_id)
        );
        // TODO: change message from clientmesssage to servermessage
        self.games
            .get(&msg.game_id)
            .unwrap()
            .iter()
            .for_each(|client| self.send_message(&msg.msg, client));
    }
}
