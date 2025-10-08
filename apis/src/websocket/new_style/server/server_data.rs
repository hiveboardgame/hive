use std::collections::HashMap;

use crate::{
    common::{ServerMessage, UserStatus, UserUpdate},
    responses::{AccountResponse, UserResponse},
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use db_lib::{get_conn, DbPool};
use tokio::sync::{watch, RwLock};
use uuid::Uuid;

pub struct ServerData {
    pool: DbPool,
    //using a watch channel to send messages to all clients (clonable receiver)
    //only retans the last message
    sender: watch::Sender<InternalServerMessage>,
    online_users: RwLock<HashMap<Uuid, (AccountResponse, i32)>>,
}

impl ServerData {
    pub fn new(pool: DbPool) -> Self {
        let (sender, _) = watch::channel(InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Error("Server notifications initialized".to_string()),
        });
        Self {
            pool,
            sender,
            online_users: RwLock::new(HashMap::new()),
        }
    }
    pub fn receiver(&self) -> watch::Receiver<InternalServerMessage> {
        self.sender.subscribe()
    }
    pub fn sender(&self) -> watch::Sender<InternalServerMessage> {
        self.sender.clone()
    }
    async fn account_from(&self, id: Option<Uuid>) -> Option<AccountResponse> {
        if let Some(id) = id {
            let mut conn = get_conn(&self.pool)
                .await
                .expect("Failed to get connection for DB");
            AccountResponse::from_uuid(&id, &mut conn).await.ok()
        } else {
            None
        }
    }
    fn notify_user_status(&self, user: UserResponse, status: UserStatus) {
        let message = InternalServerMessage { 
            destination: MessageDestination::Global,
             message: ServerMessage::UserStatus(UserUpdate {
                user,
                status
             })
        };
        self.sender.send(message).expect("Failed to send user status notification");

    }
    pub async fn remove_user(&self, id: Option<Uuid>) {
        if let Some(account) = self.account_from(id).await {
            let mut users = self.online_users.write().await;
            let value = users.get(&account.id).map(|(_, c)| *c);
            let user = &account.username;
            if let Some(count) = value {
                if count > 1 {
                    println!("{user} closed a tab");
                    users.insert(account.id, (account, count - 1));
                } else {
                    println!("{user} disconnected");
                    users.remove(&account.id);
                    self.notify_user_status(account.user,UserStatus::Offline)
                }
            };
        } else {
            println!("Anonymous disconnected");
        }
    }
    pub async fn add_user(&self, id: Option<Uuid>) {
        if let Some(account) = self.account_from(id).await {
            let mut users = self.online_users.write().await;
            let value = users.get(&account.id).map(|(_, c)| *c);
            let user = &account.username;
            if let Some(count) = value {
                println!("{user} opened a tab");
                users.insert(account.id, (account, count + 1));
            } else {
                println!("{user} connected");
                let user = account.user.clone();
                users.insert(account.id, (account, 1));
                self.notify_user_status(user,UserStatus::Online)
            };
        } else {
            println!("Anonymous connected");
        }
    }
    pub async fn get_online_users(&self) -> Vec<UserResponse> {
        self.online_users.read().await.values().map(|(a,_)| a.user.clone()).collect()
    }
}
