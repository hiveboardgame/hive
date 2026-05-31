use crate::providers::chat::Chat;
use leptos::prelude::*;
use shared_types::ChatMessageContainer;

pub fn handle_chat(container: ChatMessageContainer) {
    let chat = expect_context::<Chat>();
    chat.recv(container);
}
