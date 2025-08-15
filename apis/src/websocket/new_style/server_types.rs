use crate::{
    common::ServerMessage,
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use tokio::sync::watch;

pub struct ServerNotifications {
    //using a watch channel to send messages to all clients (clonable receiver)
    //only retans the last message
    sender: watch::Sender<InternalServerMessage>,
}
impl Default for ServerNotifications {
    fn default() -> Self {
        let (sender, _) = watch::channel(InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Error("Server notifications initialized".to_string()),
        });
        Self { sender }
    }
}
impl ServerNotifications {
    pub fn receiver(&self) -> watch::Receiver<InternalServerMessage> {
        self.sender.subscribe()
    }
    pub fn sender(&self) -> watch::Sender<InternalServerMessage> {
        self.sender.clone()
    }
}
