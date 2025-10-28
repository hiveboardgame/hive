use crate::{
    common::ServerMessage,
    responses::AccountResponse,
    websocket::{lag_tracking::PingStats, new_style::server::ServerData},
};
use db_lib::DbPool;
use futures::{channel::mpsc, SinkExt};
use server_fn::ServerFnError;
use shared_types::{GameId, TournamentId};
use std::sync::{Arc, RwLock};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

type ClientResult = Result<ServerMessage, ServerFnError>;

struct Data {
    pub cancel: CancellationToken,
    pub pings: RwLock<PingStats>,
    pub account: Option<AccountResponse>,
    pub id: Uuid,
    pub pool: DbPool,
}

#[derive(Clone)]
pub struct TabData {
    data: Arc<Data>,
    sender: mpsc::Sender<ClientResult>,
    pub subscribed_game: Arc<RwLock<Option<GameId>>>,
    pub subscribed_tournament: Arc<RwLock<Option<TournamentId>>>,
}

impl TabData {
    pub fn new(
        sender: mpsc::Sender<ClientResult>,
        account: Option<AccountResponse>,
        pool: DbPool,
    ) -> Self {
        let data = Data {
            id: Uuid::new_v4(),
            pings: RwLock::new(PingStats::default()),
            cancel: CancellationToken::new(),
            account,
            pool,
        };
        TabData {
            subscribed_game: Arc::new(RwLock::new(None)),
            subscribed_tournament: Arc::new(RwLock::new(None)),
            data: Arc::new(data),
            sender,
        }
    }
    pub fn pool(&self) -> &DbPool {
        &self.data.pool
    }
    pub fn account(&self) -> Option<&AccountResponse> {
        self.data.account.as_ref()
    }

    pub fn is_cancelled(&self) -> bool {
        self.data.cancel.is_cancelled()
    }
    pub fn as_subscriber(&self) -> (Uuid, CancellationToken) {
        (self.data.id, self.data.cancel.clone())
    }
    pub fn token(&self) -> CancellationToken {
        self.data.cancel.clone()
    }
    pub fn close(&self, server_data: &ServerData) {
        if let Some(user) = self.data.account.as_ref() {
            server_data.remove_user(user.user.clone());
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
    pub fn send(&self, request: ServerMessage, server_data: &ServerData) {
        let mut sender = self.sender.clone();
        let ret = sender.try_send(Ok(request.clone()));
        if ret.is_err() {
            println!("Failed sending {request:?} with error {ret:?}");
            self.close(server_data);
        }
    }
}
