use crate::common::{ClientRequest, GameAction};
use futures::{
    channel::mpsc::{self, Sender},
    SinkExt,
};
use hive_lib::{GameControl, Turn};
use leptos::{
    logging,
    prelude::{GetValue, ReadSignal, RwSignal, Set, SetValue, StoredValue},
};
use server_fn::ServerFnError;
use shared_types::GameId;
pub type ClientResult = Result<ClientRequest, ServerFnError>;

#[derive(Clone, Copy)]
pub struct ClientApi {
    ws_restart: RwSignal<()>,
    //client api holds the client mpsc sender
    //and the latest message received from the server
    sender: StoredValue<Sender<ClientResult>>,
}
impl Default for ClientApi {
    fn default() -> Self {
        let (tx, _rx) = mpsc::channel(1);
        Self {
            ws_restart: RwSignal::new(()),
            sender: StoredValue::new(tx),
        }
    }
}
impl ClientApi {
    pub fn set_sender(&self, sender: Sender<ClientResult>) {
        self.sender.set_value(sender);
    }
    pub fn restart_ws(&self) {
        self.ws_restart.set(());
    }
    pub fn signal_restart_ws(&self) -> ReadSignal<()> {
        self.ws_restart.read_only()
    }
    async fn send(&self, client_request: ClientRequest) {
        let mut sender = self.sender.get_value();
        let ret = sender.send(Ok(client_request.clone())).await;
        if ret.is_err() {
            logging::log!("Msg: {client_request:?} Error: {ret:?}");
        }
    }
    pub async fn join_game(&self, id: GameId) {
        let req = ClientRequest::Game {
            game_id: id,
            action: GameAction::Join,
        };
        self.send(req).await;
    }
    pub async fn turn(&self, game_id: GameId, turn: Turn) {
        let msg = ClientRequest::Game {
            game_id,
            action: GameAction::Turn(turn),
        };
        self.send(msg).await;
    }
    pub async fn pong(&self, nonce: u64) {
        let msg = ClientRequest::Pong(nonce);
        self.send(msg).await;
    }
    pub async fn game_control(&self, game_id: GameId, gc: GameControl) {
        let msg = ClientRequest::Game {
            game_id,
            action: GameAction::Control(gc),
        };
        self.send(msg).await;
    }

}
