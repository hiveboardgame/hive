use crate::{
    common::server_result::ServerMessage,
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use shared_types::chat_message::ChatMessageContainer;

pub struct ChatHandler {
    message: ChatMessageContainer,
}

impl ChatHandler {
    pub fn new(mut message: ChatMessageContainer) -> Self {
        message.time();
        Self { message }
    }

    pub fn handle(&self) -> Vec<InternalServerMessage> {
        vec![InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Chat(
            self.message.to_owned()),
        }]
    }
}
