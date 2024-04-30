use super::{api_requests::ApiRequests, auth_context::AuthContext};
use leptos::*;
use shared_types::chat_message::{ChatDestination, ChatMessage, ChatMessageContainer};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Copy)]
pub struct Chat {
    pub users_messages: RwSignal<HashMap<Uuid, Vec<ChatMessage>>>, // Uuid -> Messages
    pub users_new_messages: RwSignal<bool>,
    pub games_private_messages: RwSignal<HashMap<String, Vec<ChatMessage>>>, // game_id -> Messages
    pub games_private_new_messages: RwSignal<bool>,
    pub games_public_messages: RwSignal<HashMap<String, Vec<ChatMessage>>>, // game_id -> Messages
    pub games_public_new_messages: RwSignal<bool>,
    pub tournament_lobby_messages: RwSignal<HashMap<String, Vec<ChatMessage>>>, // tournament_id -> Messages
    pub tournament_lobby_new_messages: RwSignal<bool>,
}

impl Default for Chat {
    fn default() -> Self {
        Self::new()
    }
}

impl Chat {
    pub fn new() -> Self {
        Self {
            users_messages: create_rw_signal(HashMap::new()),
            users_new_messages: create_rw_signal(false),
            games_private_messages: create_rw_signal(HashMap::new()),
            games_private_new_messages: create_rw_signal(false),
            games_public_messages: create_rw_signal(HashMap::new()),
            games_public_new_messages: create_rw_signal(false),
            tournament_lobby_messages: create_rw_signal(HashMap::new()),
            tournament_lobby_new_messages: create_rw_signal(false),
        }
    }

    pub fn reset(&mut self) {
        batch(move || {
            self.users_messages.update(|h| h.clear());
            self.users_new_messages.set(false);
            self.games_private_messages.update(|h| h.clear());
            self.games_private_new_messages.set(false);
            self.games_public_messages.update(|h| h.clear());
            self.games_public_new_messages.set(false);
            self.tournament_lobby_messages.update(|h| h.clear());
            self.tournament_lobby_new_messages.set(false);
        });
    }

    pub fn send(&self, message: &str, destination: ChatDestination) {
        let auth_context = expect_context::<AuthContext>();
        if let Some(Ok(Some(account))) = untrack(auth_context.user) {
            let id = account.user.uid;
            let name = account.user.username;
            let msg = ChatMessage::new(name, id, message, None);
            let container = ChatMessageContainer::new(destination, &msg);
            ApiRequests::new().chat(&container);
        }
    }

    pub fn recv(&mut self, containers: &Vec<ChatMessageContainer>) {
        for container in containers {
            match &container.destination {
                ChatDestination::TournamentLobby(tournament_id) => {
                    self.tournament_lobby_new_messages.set(true);
                    self.tournament_lobby_messages.update(|tournament| {
                        tournament
                            .entry(tournament_id.clone())
                            .or_default()
                            .push(container.message.to_owned())
                    })
                }
                ChatDestination::User((id, _name)) => self.users_messages.update(|users| {
                    self.users_new_messages.set(true);
                    users
                        .entry(*id)
                        .or_default()
                        .push(container.message.to_owned())
                }),
                ChatDestination::GamePlayers(game_id, ..) => {
                    self.games_private_new_messages.set(true);
                    self.games_private_messages.update(|games| {
                        games
                            .entry(game_id.to_owned())
                            .or_default()
                            .push(container.message.to_owned())
                    })
                }
                ChatDestination::GameSpectators(game_id, ..) => {
                    self.games_public_new_messages.set(true);
                    self.games_public_messages.update(|games| {
                        games
                            .entry(game_id.to_owned())
                            .or_default()
                            .push(container.message.to_owned())
                    })
                }
            }
        }
    }
}

pub fn provide_chat() {
    provide_context(Chat::new())
}
