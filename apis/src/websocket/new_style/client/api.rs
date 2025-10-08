use crate::common::{ClientRequest, ServerMessage};
use futures::channel::mpsc::{self, Sender};
use leptos::prelude::{GetValue, ReadSignal, RwSignal, Set, SetValue, StoredValue};
use server_fn::ServerFnError;

type ClientResult = Result<ClientRequest, ServerFnError>;

#[derive(Clone)]
pub struct ClientApi {
    ws_restart: RwSignal<()>,
    //client api holds the client mpsc sender
    //and the latest message received from the server
    sender: StoredValue<Sender<ClientResult>>,
    pub latest: RwSignal<Result<ServerMessage, ServerFnError>>,
}
impl Default for ClientApi {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientApi {
    pub fn new() -> Self {
        let (tx, _rx) = mpsc::channel(1);
        Self {
            ws_restart: RwSignal::new(()),
            sender: StoredValue::new(tx),
            latest: RwSignal::new(Ok(ServerMessage::Error("".into()))),
        }
    }
    pub fn set_sender(&self, sender: Sender<ClientResult>) {
        self.sender.set_value(sender);
    }
    pub fn send(&self, client_request: ClientRequest) {
        let mut sender = self.sender.get_value();

        let _ = sender.try_send(Ok(client_request));
    }
    pub fn restart_ws(&self) {
        self.send(ClientRequest::Disconnect);
        self.ws_restart.set(());
    }
    pub fn signal_restart_ws(&self) -> ReadSignal<()> {
        self.ws_restart.read_only()
    }
}
