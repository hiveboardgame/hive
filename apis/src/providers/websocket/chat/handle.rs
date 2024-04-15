use crate::providers::chat::Chat;
use leptos::logging::log;
use leptos::*;
use shared_types::chat_message::ChatMessageContainer;

pub fn handle_chat(container: ChatMessageContainer) {
    let mut chat = expect_context::<Chat>();
    log!("Handle chat got: {}", container.message.message);
    chat.recv(&container);
}
