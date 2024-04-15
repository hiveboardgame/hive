use super::{api_requests::ApiRequests, auth_context::AuthContext};
use leptos::logging::log;
use leptos::*;
use shared_types::chat_message::{ChatDestination, ChatMessage, ChatMessageContainer};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Copy)]
pub struct Chat {
    pub users: RwSignal<HashMap<Uuid, Vec<ChatMessage>>>, // Uuid -> Messages
    pub games: RwSignal<HashMap<String, Vec<ChatMessage>>>, // game_id -> Messages
    pub lobby: RwSignal<Vec<ChatMessage>>,
}

impl Default for Chat {
    fn default() -> Self {
        Self::new()
    }
}

impl Chat {
    pub fn new() -> Self {
        Self {
            users: create_rw_signal(HashMap::new()),
            games: create_rw_signal(HashMap::new()),
            lobby: create_rw_signal(Vec::new()),
        }
    }

    pub fn send(&self, message: &str, destination: ChatDestination) {
        log!("Message in send: {message}");
        let auth_context = expect_context::<AuthContext>();
        if let Some(Ok(Some(account))) = untrack(auth_context.user) {
            let id = account.user.uid;
            let name = account.user.username;
            let msg = ChatMessage::new(name, id, message, None);
            let container = ChatMessageContainer::new(destination, &msg);
            ApiRequests::new().chat(&container);
        }
    }

    pub fn recv(&mut self, container: &ChatMessageContainer) {
        match &container.destination {
            ChatDestination::Lobby => self
                .lobby
                .update(|lobby| lobby.push(container.message.to_owned())),
            ChatDestination::User((id, _name)) => self.users.update(|users| {
                users
                    .entry(*id)
                    .or_default()
                    .push(container.message.to_owned())
            }),
            ChatDestination::Game(game_id) => self.games.update(|games| {
                games
                    .entry(game_id.to_owned())
                    .or_default()
                    .push(container.message.to_owned())
            }),
        }
    }
}

pub fn provide_chat() {
    provide_context(Chat::new())
}
