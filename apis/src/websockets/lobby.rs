use crate::websockets::messages::{ClientActorMessage, Connect, Disconnect, WsMessage};
use actix::prelude::{Actor, Context, Handler, Recipient};
use db_lib::DbPool;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

type Socket = Recipient<WsMessage>;

#[derive(Debug)]
pub struct Lobby {
    #[allow(dead_code)]
    id: String,
    sessions: HashMap<Uuid, Socket>, // user_id to (socket_)id
    games_users: HashMap<String, HashSet<Uuid>>, // game_id to set of users
    users_games: HashMap<Uuid, HashSet<String>>,
    #[allow(dead_code)]
    pool: DbPool,
}

impl Lobby {
    pub fn new(pool: DbPool) -> Lobby {
        Lobby {
            id: String::from("lobby"),
            sessions: HashMap::new(),
            games_users: HashMap::new(),
            users_games: HashMap::new(),
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
            if let Some(games) = self.users_games.remove(&msg.user_id) {
                for game in games.iter() {
                    self.games_users
                        .get(game)
                        .expect("Game exists")
                        .iter()
                        .filter(|conn_id| *conn_id.to_owned() != msg.user_id)
                        .for_each(|user_id| {
                            self.send_message(&format!("{} disconnected.", &msg.user_id), user_id)
                        });
                    if let Some(game_users) = self.games_users.get_mut(game) {
                        if game_users.len() > 1 {
                            game_users.remove(&msg.user_id);
                        } else {
                            //only one in the game, remove it entirely
                            self.games_users.remove(game);
                        }
                    }
                }
            }
        }
    }
}

impl Handler<Connect> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: Connect, _ctx: &mut Context<Self>) -> Self::Result {
        self.games_users
            .entry(msg.game_id.clone())
            .or_default()
            .insert(msg.user_id);
        self.users_games
            .entry(msg.user_id.clone())
            .or_default()
            .insert(msg.game_id.clone());
        self.games_users
            .get(&msg.game_id)
            .expect("Uuid exists")
            .iter()
            .filter(|conn_id| *conn_id.to_owned() != msg.user_id)
            .for_each(|conn_id| {
                self.send_message(&format!("{} just joined!", msg.user_id), conn_id)
            });
        self.sessions.insert(msg.user_id, msg.addr.clone());

        if msg.game_id == "lobby" {
            self.send_message(&format!("You joined {}", msg.game_id), &msg.user_id);
            return ();
        }
    }
}

impl Handler<ClientActorMessage> for Lobby {
    type Result = ();

    fn handle(&mut self, cam: ClientActorMessage, _ctx: &mut Context<Self>) -> Self::Result {
        self.games_users
            .entry(cam.game_id.clone())
            .or_default()
            .insert(cam.user_id);
        self.games_users
            .get(&cam.game_id)
            .expect("Uuid exists")
            .iter()
            .for_each(|client| self.send_message(&cam.serialized, client));
    }
}
