use crate::responses::AccountResponse;

use super::{
    api_requests::ApiRequests, auth_context::AuthContext, AlertType, AlertsContext,
    ApiRequestsProvider,
};
use leptos::prelude::*;
use shared_types::{ChatDestination, ChatMessage, ChatMessageContainer, GameId, TournamentId};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Copy, Clone, Debug)]
pub struct Chat {
    pub users_messages: RwSignal<HashMap<Uuid, Vec<ChatMessage>>>, // Uuid -> Messages
    pub users_new_messages: RwSignal<HashMap<Uuid, bool>>,
    pub games_private_messages: RwSignal<HashMap<GameId, Vec<ChatMessage>>>, // game_id -> Messages
    pub games_private_new_messages: RwSignal<HashMap<GameId, bool>>,
    pub games_public_messages: RwSignal<HashMap<GameId, Vec<ChatMessage>>>, // game_id -> Messages
    pub games_public_new_messages: RwSignal<HashMap<GameId, bool>>,
    pub tournament_lobby_messages: RwSignal<HashMap<TournamentId, Vec<ChatMessage>>>, // tournament_id -> Messages
    pub tournament_lobby_new_messages: RwSignal<HashMap<TournamentId, bool>>,
    pub typed_message: RwSignal<String>,
    user: Signal<Option<AccountResponse>>,
    api: Signal<ApiRequests>,
}

impl Chat {
    pub fn new(user: Signal<Option<AccountResponse>>, api: Signal<ApiRequests>) -> Self {
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
            user,
            api,
        }
    }

    pub fn has_messages(&self, game_id: GameId) -> bool {
        self.games_public_new_messages
            .get()
            .get(&game_id)
            .is_some_and(|v| *v)
            || self
                .games_private_new_messages
                .get()
                .get(&game_id)
                .is_some_and(|v| *v)
    }

    pub fn seen_messages(&self, game_id: GameId) {
        self.games_public_new_messages.update(|m| {
            m.entry(game_id.clone())
                .and_modify(|b| *b = false)
                .or_insert(false);
        });
        self.games_private_new_messages.update(|m| {
            m.entry(game_id).and_modify(|b| *b = false).or_insert(false);
        });
    }

    pub fn send(&self, message: &str, destination: ChatDestination, turn: Option<usize>) {
        if let Some(account) = self.user.get_untracked() {
            let id = account.user.uid;
            let name = account.user.username;
            let turn = match destination {
                ChatDestination::GamePlayers(_, _, _)
                | ChatDestination::GameSpectators(_, _, _) => turn,
                _ => None,
            };
            let msg = ChatMessage::new(name, id, message, None, turn);
            let container = ChatMessageContainer::new(destination, &msg);
            self.api.get().chat(&container);
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
                    self.tournament_lobby_messages.update(|tournament| {
                        tournament
                            .entry(id.clone())
                            .or_default()
                            .extend(containers.iter().map(|container| container.message.clone()));
                    });
                    self.tournament_lobby_new_messages.update(|m| {
                        m.entry(id.clone())
                            .and_modify(|value| *value = true)
                            .or_insert(true);
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

                    self.users_messages.update(|users| {
                        users
                            .entry(*id)
                            .or_default()
                            .extend(containers.iter().map(|container| container.message.clone()));
                    });
                    self.users_new_messages.update(|m| {
                        m.entry(*id)
                            .and_modify(|value| *value = true)
                            .or_insert(true);
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
                ChatDestination::Global => {
                    let alerts = expect_context::<AlertsContext>();
                    alerts.last_alert.update(|v| {
                        *v = Some(AlertType::Warn(last_message.message.message.to_string()))
                    });
                }
            }
        }
    }
}

pub fn provide_chat() {
    let user = expect_context::<AuthContext>().user;
    let api = expect_context::<ApiRequestsProvider>().0;
    provide_context(Chat::new(user, api))
}
