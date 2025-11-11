use std::sync::Arc;

use crate::{
    common::ServerMessage,
    websocket::{
        ServerData, chat::UserToUser, messages::{InternalServerMessage, MessageDestination}
    },
};
use shared_types::{ChatDestination, ChatMessageContainer};

pub struct ChatHandler {
    container: ChatMessageContainer,
    data: Arc<ServerData>,
}

impl ChatHandler {
    pub fn new(mut container: ChatMessageContainer, data: Arc<ServerData>) -> Self {
        container.time();
        Self { container, data }
    }

    pub fn handle(&self) -> Vec<InternalServerMessage> {
        let mut messages = Vec::new();
        match &self.container.destination {
            ChatDestination::TournamentLobby(tournament_id) => {
                let mut tournament_lobby = self.data.chat_storage.tournament.write().unwrap();
                let entry = tournament_lobby.entry(tournament_id.clone()).or_default();
                entry.push(self.container.clone());
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Tournament(tournament_id.clone()),
                    message: ServerMessage::Chat(vec![self.container.to_owned()]),
                })
            }
            ChatDestination::GamePlayers(game_id, white_id, black_id) => {
                let mut games_private = self.data.chat_storage.games_private.write().unwrap();
                let entry = games_private.entry(game_id.clone()).or_default();
                entry.push(self.container.clone());
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(*white_id),
                    message: ServerMessage::Chat(vec![self.container.to_owned()]),
                });
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(*black_id),
                    message: ServerMessage::Chat(vec![self.container.to_owned()]),
                });
            }
            ChatDestination::GameSpectators(game, white_id, black_id) => {
                let mut games_public = self.data.chat_storage.games_public.write().unwrap();
                let entry = games_public.entry(game.clone()).or_default();
                entry.push(self.container.clone());
                messages.push(InternalServerMessage {
                    destination: MessageDestination::GameSpectators(
                        game.clone(),
                        *white_id,
                        *black_id,
                    ),
                    message: ServerMessage::Chat(vec![self.container.to_owned()]),
                })
            }
            ChatDestination::User((id, _username)) => {
                let sender = self.container.message.user_id;
                self.data
                    .chat_storage
                    .insert_or_update_direct_lookup(sender, *id);
                let user_to_user = UserToUser::new(*id, sender);
                let mut direct = self.data.chat_storage.direct.write().unwrap();
                let entry = direct.entry(user_to_user).or_default();
                entry.push(self.container.clone());
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(*id),
                    message: ServerMessage::Chat(vec![self.container.to_owned()]),
                })
            }
            ChatDestination::Global => messages.push(InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Chat(vec![self.container.to_owned()]),
            }),
        };
        messages
    }
}
