use crate::providers::chat::Chat;
use leptos::*;
use shared_types::chat_message::ChatMessageContainer;
use leptos::logging::log;

pub fn handle_chat(container: ChatMessageContainer) {
    let mut chat = expect_context::<Chat>();
    log!("Handle chat got: {}", container.message.message);
    chat.recv(&container);
}
