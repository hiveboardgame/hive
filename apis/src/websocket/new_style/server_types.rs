use std::{collections::HashMap, ops::DerefMut, sync::Arc};

use crate::{
    common::ServerMessage,
    responses::AccountResponse,
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use db_lib::{get_conn, DbPool};
use futures::channel::mpsc;
use server_fn::ServerFnError;
use tokio::sync::{watch, RwLock};
use uuid::Uuid;
pub struct ServerData {
    //using a watch channel to send messages to all clients (clonable receiver)
    //only retans the last message
    sender: watch::Sender<InternalServerMessage>,
    online_users: RwLock<HashMap<Uuid, AccountResponse>>,
}
impl Default for ServerData {
    fn default() -> Self {
        let (sender, _) = watch::channel(InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Error("Server notifications initialized".to_string()),
        });
        Self {
            sender,
            online_users: RwLock::new(HashMap::new()),
        }
    }
}
impl ServerData {
    pub fn receiver(&self) -> watch::Receiver<InternalServerMessage> {
        self.sender.subscribe()
    }
    pub fn sender(&self) -> watch::Sender<InternalServerMessage> {
        self.sender.clone()
    }
}

type ClientResult = Result<ServerMessage, ServerFnError>;

#[derive(Clone)]
pub struct ClientSender {
    sender: mpsc::Sender<ClientResult>,
    pool: DbPool,
    pub id: Arc<RwLock<Option<Uuid>>>,
}

impl ClientSender {
    pub fn new(sender: mpsc::Sender<ClientResult>, pool: DbPool, id: Option<Uuid>) -> Self {
        ClientSender {
            sender,
            pool,
            id: Arc::new(RwLock::new(id)),
        }
    }
    pub async fn send(
        &mut self,
        request: ServerMessage,
        server_data: &ServerData,
    ) -> Result<(), ServerFnError> {
        let ret = self.sender.try_send(Ok(request));
        if ret.is_err() {
            //Handle client disconection
            let mut conn = get_conn(&self.pool).await?;
            let account = if let Some(id) = *self.id.read().await {
                AccountResponse::from_uuid(&id, &mut conn).await.ok()
            } else {
                None
            };

            let err = if let Some(account) = account {
                let mut users = server_data.online_users.write().await;
                users.deref_mut().remove(&account.id);
                format!("Client with username {} disconected", account.username)
            } else {
                "Anonymous client disconected".to_string()
            };
            Err(ServerFnError::new(err))
        } else {
            Ok(())
        }
    }
}
