use crate::providers::chat::Chat;
use leptos::*;
use shared_types::ChatMessageContainer;

pub fn handle_chat(containers: Vec<ChatMessageContainer>) {
    let mut chat = expect_context::<Chat>();
    chat.recv(&containers);
}
