use crate::{
    common::ServerMessage, responses::AccountResponse, websocket::{lag_tracking::PingStats, new_style::server::ServerData}
};
use server_fn::ServerFnError;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use std::sync::{RwLock, Arc};
use futures::{channel::mpsc, SinkExt};

type ClientResult = Result<ServerMessage, ServerFnError>;

struct InternalClientData {
    pub cancel: CancellationToken,
    pub pings: RwLock<PingStats>,
    pub account: Option<AccountResponse>,
    pub id: Uuid
}

#[derive(Clone)]
pub struct ClientData {
    data: Arc<InternalClientData>,
    sender: mpsc::Sender<ClientResult>,

}

impl ClientData {
    pub fn new(sender: mpsc::Sender<ClientResult>, account: Option<AccountResponse>, cancel: CancellationToken) -> Self {
        let data = InternalClientData {
            id: Uuid::new_v4(),
            pings: RwLock::new(PingStats::default()),
            cancel,
            account
        };
        ClientData {
            data: Arc::new(data),
            sender,

        }
    }

    pub fn account(&self) ->Option<&AccountResponse> {
        self.data.account.as_ref()
    }
    pub fn uuid(&self) ->&Uuid {
        &self.data.id
    }
    pub fn is_cancelled(&self) -> bool {
        self.data.cancel.is_cancelled()
    }
    pub fn close(&self, server_data: &ServerData) {
         if let Some(user) = self.data.account.as_ref() {
            server_data.remove_user(user.clone());
        }        
        self.data.cancel.cancel();
    }
    pub fn update_pings(&self, nonce: u64) {
        let mut pings = self.data.pings.write().unwrap();
        pings.update(nonce);
    }
    pub fn pings_value(&self) -> f64 {
        let pings = self.data.pings.read().unwrap();
        pings.value()
    }
    pub async fn send(&self, request: ServerMessage, server_data: &ServerData) {
        let mut sender = self.sender.clone();
        let ret = sender.send(Ok(request.clone())).await;
        if ret.is_err() {
            //println!("Failed sending {request:?}");
            self.close(server_data);
        }
    }
}
