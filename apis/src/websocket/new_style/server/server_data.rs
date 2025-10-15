use std::collections::HashMap;

use crate::{
    common::{ServerMessage, UserStatus, UserUpdate},
    responses::{AccountResponse, UserResponse},
    websocket::{messages::{InternalServerMessage, MessageDestination}, new_style::server::ClientData},
};
use shared_types::GameId;
use tokio::{sync::watch};
use std::sync::RwLock;
use uuid::Uuid;

pub struct ServerData {
    //using a watch channel to send messages to all clients (clonable receiver)
    //only retans the last message
    sender: watch::Sender<InternalServerMessage>,
    receiver: watch::Receiver<InternalServerMessage>,
    online_users: RwLock<HashMap<Uuid, (AccountResponse, i32)>>,
    game_subscribers: RwLock<HashMap<GameId, Vec<ClientData>>>
}

impl ServerData {
    pub fn new(channel: (watch::Sender<InternalServerMessage>, watch::Receiver<InternalServerMessage>)) -> Self { 
        Self {
            sender: channel.0,
            receiver: channel.1,
            online_users: RwLock::new(HashMap::new()),
            game_subscribers: RwLock::new(HashMap::new()),
        }
    }
    pub fn receiver(&self) -> watch::Receiver<InternalServerMessage> {
        self.receiver.clone()
    }
    fn send(&self, message: InternalServerMessage) -> Result<(), String>{
        self.sender.send(message).map_err(|e| e.to_string())
    }

    fn notify_user_status(&self, user: UserResponse, status: UserStatus) {
        let message = InternalServerMessage { 
            destination: MessageDestination::Global,
             message: ServerMessage::UserStatus(UserUpdate {
                user,
                status
             })
        };
        self.send(message).expect("Failed to send update status notification")

    }
    pub fn remove_user(&self, account: AccountResponse) {
        let mut users = self.online_users.write().unwrap();
        let value = users.get(&account.id).map(|(_, c)| *c);
        let user = &account.username;
        if let Some(count) = value {
            if count > 1 {
                println!("{user} closed a tab");
                users.insert(account.id, (account.clone(), count - 1));
            } else {
                println!("{user} disconnected");
                users.remove(&account.id);
                self.notify_user_status(account.user.clone(),UserStatus::Offline)
            }
        };
    }
    pub fn add_user(&self, account: AccountResponse) {
        let mut users = self.online_users.write().unwrap();
        let (_, count) = *users.entry(account.id).and_modify(|(_, count)| {
            *count+=1;
        }).or_insert((account.clone(),1));
        if count >1 {
            println!("{} opened a tab", account.username);
        } else {
            println!("{} connected", account.username);
            self.notify_user_status(account.user.clone(),UserStatus::Online)
        };
    }
    pub fn get_online_users(&self) -> Vec<UserResponse> {
        self.online_users.read().unwrap().values().map(|(a,_)| a.user.clone()).collect()
    }
    pub fn subscribe_client_to(&self, client: ClientData, game_id: GameId) {
        let mut subscribers = self.game_subscribers.write().unwrap();
        subscribers.entry(game_id.clone()).and_modify(|v| {v.push(client.clone());} ).or_insert(vec![client.clone()]);
    }
    pub fn game_subscribers(&self, game_id: &GameId) -> Vec<ClientData>{
        let mut subscribers = self.game_subscribers.write().unwrap();
        if let Some(v) = subscribers.get_mut(game_id){
            v.retain(|c| !c.is_cancelled());
            v.clone()
        } else {
            Vec::new()
        }
    } 
}
