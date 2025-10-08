use std::sync::Arc;

use crate::{
    common::ServerMessage,
    websocket::{lag_tracking::PingStats, new_style::server::ServerData},
};
use futures::{channel::mpsc, SinkExt};
use server_fn::ServerFnError;
use tokio::sync::RwLock;
use uuid::Uuid;

type ClientResult = Result<ServerMessage, ServerFnError>;

#[derive(Clone)]
pub struct ClientData {
    abort: Arc<RwLock<bool>>,
    pings: Arc<RwLock<PingStats>>,
    sender: mpsc::Sender<ClientResult>,
    pub id: Option<Uuid>,
}

impl ClientData {
    pub fn new(sender: mpsc::Sender<ClientResult>, id: Option<Uuid>) -> Self {
        ClientData {
            pings: Arc::new(RwLock::new(PingStats::default())),
            abort: Arc::new(RwLock::new(false)),
            sender,
            id,
        }
    }
    pub async fn is_closed(&self) -> bool {
        *self.abort.read().await
    }
    pub async fn close(&self, server_data: &ServerData) {
        server_data.remove_user(self.id).await;
        *self.abort.write().await = true;
    }
    pub async fn update_pings(&self, nonce: u64) {
        let mut pings = self.pings.write().await;
        pings.update(nonce);
    }
    pub async fn pings_value(&self) -> f64 {
        let pings = self.pings.write().await;
        pings.value()
    }
    pub async fn send(&mut self, request: ServerMessage, server_data: &ServerData) {
        let ret = self.sender.send(Ok(request)).await;
        if ret.is_err() {
            self.close(server_data).await;
        }
    }
}
