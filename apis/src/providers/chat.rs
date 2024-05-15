use super::{
    api_requests::ApiRequests, auth_context::AuthContext, game_state::GameStateSignal,
    navigation_controller::NavigationControllerSignal,
};
use leptos::*;
use shared_types::{ChatDestination, ChatMessage, ChatMessageContainer};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Copy, Clone, Debug)]
pub struct Chat {
    pub users_messages: RwSignal<HashMap<Uuid, Vec<ChatMessage>>>, // Uuid -> Messages
    pub users_new_messages: RwSignal<HashMap<Uuid, bool>>,
    pub games_private_messages: RwSignal<HashMap<String, Vec<ChatMessage>>>, // game_id -> Messages
    pub games_private_new_messages: RwSignal<HashMap<String, bool>>,
    pub games_public_messages: RwSignal<HashMap<String, Vec<ChatMessage>>>, // game_id -> Messages
    pub games_public_new_messages: RwSignal<HashMap<String, bool>>,
    pub tournament_lobby_messages: RwSignal<HashMap<String, Vec<ChatMessage>>>, // tournament_id -> Messages
    pub tournament_lobby_new_messages: RwSignal<HashMap<String, bool>>,
    pub typed_message: RwSignal<String>,
}

impl Default for Chat {
    fn default() -> Self {
        Self::new()
    }
}

impl Chat {
    pub fn new() -> Self {
        Self {
            users_messages: RwSignal::new(HashMap::new()),
            users_new_messages: RwSignal::new(HashMap::new()),
            games_private_messages: RwSignal::new(HashMap::new()),
            games_private_new_messages: RwSignal::new(HashMap::new()),
            games_public_messages: RwSignal::new(HashMap::new()),
            games_public_new_messages: RwSignal::new(HashMap::new()),
            tournament_lobby_messages: RwSignal::new(HashMap::new()),
            tournament_lobby_new_messages: RwSignal::new(HashMap::new()),
            typed_message: RwSignal::new(String::new()),
        }
    }

    pub fn has_messages(&self) -> bool {
        let navi = expect_context::<NavigationControllerSignal>();

        if let Some(nanoid) = navi.signal.get_untracked().nanoid {
            self.games_public_new_messages
                .get()
                .get(&nanoid)
                .map_or(false, |v| *v)
                || self
                    .games_private_new_messages
                    .get()
                    .get(&nanoid)
                    .map_or(false, |v| *v)
        } else {
            false
        }
    }

    pub fn seen_messages(&self) {
        let navi = expect_context::<NavigationControllerSignal>();
        batch(move || {
            if let Some(nanoid) = navi.signal.get_untracked().nanoid {
                self.games_public_new_messages.update(|m| {
                    m.entry(nanoid.clone())
                        .and_modify(|b| *b = false)
                        .or_insert(false);
                });
                self.games_private_new_messages.update(|m| {
                    m.entry(nanoid).and_modify(|b| *b = false).or_insert(false);
                });
            }
        })
    }

    pub fn send(&self, message: &str, destination: ChatDestination) {
        let auth_context = expect_context::<AuthContext>();
        let gamestate = expect_context::<GameStateSignal>();
        if let Some(Ok(Some(account))) = untrack(auth_context.user) {
            let id = account.user.uid;
            let name = account.user.username;
            let turn = match destination {
                ChatDestination::GamePlayers(_, _, _)
                | ChatDestination::GameSpectators(_, _, _) => {
                    Some(gamestate.signal.get_untracked().state.turn)
                }
                _ => None,
            };
            let msg = ChatMessage::new(name, id, message, None, turn);
            let container = ChatMessageContainer::new(destination, &msg);
            ApiRequests::new().chat(&container);
        }
    }

    pub fn recv(&mut self, containers: &[ChatMessageContainer]) {
        if let Some(last_message) = containers.last() {
            match &last_message.destination {
                ChatDestination::TournamentLobby(id) => {
                    if let Some(messages) = self.tournament_lobby_messages.get_untracked().get(id) {
                        if let Some(last_vec_message) = messages.last() {
                            if last_message.message == *last_vec_message {
                                return;
                            }
                        }
                    }
                    batch(move || {
                        self.tournament_lobby_messages.update(|tournament| {
                            tournament.entry(id.clone()).or_default().extend(
                                containers.iter().map(|container| container.message.clone()),
                            );
                        });
                        self.tournament_lobby_new_messages.update(|m| {
                            m.entry(id.clone())
                                .and_modify(|value| *value = true)
                                .or_insert(true);
                        });
                    });
                }

                ChatDestination::User((id, _name)) => {
                    if let Some(messages) = self.users_messages.get_untracked().get(id) {
                        if let Some(last_vec_message) = messages.last() {
                            if last_message.message == *last_vec_message {
                                return;
                            }
                        }
                    }

                    batch(move || {
                        self.users_messages.update(|users| {
                            users.entry(*id).or_default().extend(
                                containers.iter().map(|container| container.message.clone()),
                            );
                        });
                        self.users_new_messages.update(|m| {
                            m.entry(*id)
                                .and_modify(|value| *value = true)
                                .or_insert(true);
                        });
                    });
                }
                ChatDestination::GamePlayers(id, ..) => {
                    if let Some(messages) = self.games_private_messages.get_untracked().get(id) {
                        if let Some(last_vec_message) = messages.last() {
                            if last_message.message == *last_vec_message {
                                return;
                            }
                        }
                    }

                    self.games_private_messages.update(|games| {
                        games
                            .entry(id.clone())
                            .or_default()
                            .extend(containers.iter().map(|container| container.message.clone()));
                    });
                    self.games_private_new_messages.update(|m| {
                        m.entry(id.clone())
                            .and_modify(|value| *value = true)
                            .or_insert(true);
                    });
                }
                ChatDestination::GameSpectators(id, ..) => {
                    if let Some(messages) = self.games_public_messages.get_untracked().get(id) {
                        if let Some(last_vec_message) = messages.last() {
                            if last_message.message == *last_vec_message {
                                return;
                            }
                        }
                    }

                    self.games_public_messages.update(|games| {
                        games
                            .entry(id.clone())
                            .or_default()
                            .extend(containers.iter().map(|container| container.message.clone()));
                    });
                    self.games_public_new_messages.update(|m| {
                        m.entry(id.clone())
                            .and_modify(|value| *value = true)
                            .or_insert(true);
                    });
                }
            }
        }
    }
}

pub fn provide_chat() {
    provide_context(Chat::new())
}
