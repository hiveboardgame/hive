use crate::messages::{ClientActorMessage, Connect, Disconnect, WsMessage};
use actix::prelude::{Actor, Context, Handler, Recipient};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

type Socket = Recipient<WsMessage>;

#[derive(Debug)]
pub struct Lobby {
    id: Uuid,
    sessions: HashMap<Uuid, Socket>,
    rooms: HashMap<Uuid, HashSet<Uuid>>,
}

impl Default for Lobby {
    fn default() -> Lobby {
        Lobby {
            id: Uuid::new_v4(),
            sessions: HashMap::new(),
            rooms: HashMap::new(),
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
        if self.sessions.remove(&msg.id).is_some() {
            self.rooms
                .get(&msg.room_id)
                .unwrap()
                .iter()
                .filter(|conn_id| *conn_id.to_owned() != msg.id)
                .for_each(|user_id| {
                    self.send_message(&format!("{} disconnected.", &msg.id), user_id)
                });
            if let Some(lobby) = self.rooms.get_mut(&msg.room_id) {
                if lobby.len() > 1 {
                    lobby.remove(&msg.id);
                } else {
                    //only one in the lobby, remove it entirely
                    self.rooms.remove(&msg.room_id);
                }
            }
        }
    }
}

impl Handler<Connect> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("Self is {:?}", self);
        println!("msg is {:?}", msg);
        println!("rooms are: {:?}", self.rooms);
        self.rooms
            .entry(msg.lobby_id)
            .or_default()
            .insert(msg.self_id);
        println!("rooms are: {:?}", self.rooms);
        println!("room is {:?}", self.rooms.get(&msg.lobby_id));

        self.rooms
            .get(&msg.lobby_id)
            .unwrap()
            .iter()
            .filter(|conn_id| *conn_id.to_owned() != msg.self_id)
            .for_each(|conn_id| {
                self.send_message(&format!("{} just joined!", msg.self_id), conn_id)
            });

        self.sessions.insert(msg.self_id, msg.addr);

        self.send_message(&format!("your id is {}", msg.self_id), &msg.self_id);
    }
}

impl Handler<ClientActorMessage> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: ClientActorMessage, _ctx: &mut Context<Self>) -> Self::Result {
        if msg.msg.starts_with("\\w") {
            if let Some(id_to) = msg.msg.split(' ').collect::<Vec<&str>>().get(1) {
                self.send_message(&msg.msg, &Uuid::parse_str(id_to).unwrap());
            }
        } else {
            println!("Room id is: {:?}", &msg.room_id);
            println!("Users in rooms are: {:?}", self.rooms.get(&msg.room_id).unwrap());
            println!("Message is: {:?}", &msg.msg);
            self.rooms
                .get(&msg.room_id)
                .unwrap()
                .iter()
                .for_each(|client| self.send_message(&msg.msg, client));
        }
    }
}
