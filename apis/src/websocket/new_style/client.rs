use crate::{
    common::{ClientRequest, ServerMessage},
    websocket::new_style::websocket_fn::websocket_fn,
};
use futures::channel::mpsc::{Receiver, Sender};
use futures::StreamExt;
use leptos::prelude::{GetValue, RwSignal, StoredValue};
use server_fn::ServerFnError;

type ClientResult = Result<ClientRequest, ServerFnError>;

#[derive(Clone)]
pub struct ClientApi {
    //client api holds the client mpsc sender
    //and the latest message received from the server
    sender: StoredValue<Sender<ClientResult>>,
    pub latest: RwSignal<Result<ServerMessage, ServerFnError>>,
}
impl ClientApi {
    pub fn new(sender: Sender<ClientResult>) -> Self {
        Self {
            sender: StoredValue::new(sender),
            latest: RwSignal::new(Ok(ServerMessage::Error("".into()))),
        }
    }
    pub fn send(&self, client_request: ClientRequest) {
        let mut sender = self.sender.get_value();

        let _ = sender.try_send(Ok(client_request));
    }
}

pub async fn client_handler(
    rx: Receiver<Result<ClientRequest, ServerFnError>>,
    client_api: ClientApi,
) {
    match websocket_fn(rx.into()).await {
        Ok(mut messages) => {
            while let Some(msg) = messages.next().await {
                //Debug
                leptos::logging::log!("{msg:?}");
                match msg {
                    Ok(msg) => match msg {
                        ServerMessage::Ping { nonce, value } => {
                            client_api.send(ClientRequest::Pong(nonce));
                        }
                        ServerMessage::Error(e) => {}
                        _ => todo!(),
                    },
                    Err(e) => {
                        todo!()
                    }
                }
            }
        }
        Err(e) => leptos::logging::warn!("{e}"),
    }
}
