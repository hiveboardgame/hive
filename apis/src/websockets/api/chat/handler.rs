use crate::{
    common::server_result::ServerMessage,
    websockets::{
        chat::{Chats, UserToUser},
        internal_server_message::{InternalServerMessage, MessageDestination},
    },
};
use shared_types::chat_message::{ChatDestination, ChatMessageContainer};

pub struct ChatHandler {
    message: ChatMessageContainer,
    chat_storage: actix_web::web::Data<Chats>,
}

impl ChatHandler {
    pub fn new(
        mut message: ChatMessageContainer,
        chat_storage: actix_web::web::Data<Chats>,
    ) -> Self {
        message.time();
        Self {
            message,
            chat_storage,
        }
    }

    pub fn handle(&self) -> Vec<InternalServerMessage> {
        let mut messages = Vec::new();
        match &self.message.destination {
            ChatDestination::TournamentLobby(tournament) => messages.push(InternalServerMessage {
                destination: MessageDestination::Tournament(tournament.clone()),
                message: ServerMessage::Chat(vec![self.message.to_owned()]),
            }),
            ChatDestination::GamePlayers(game, white_id, black_id) => {
                let mut games_private = self.chat_storage.games_private.write().unwrap();
                let entry = games_private.entry(game.clone()).or_default();
                entry.push(self.message.clone());
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(*white_id),
                    message: ServerMessage::Chat(vec![self.message.to_owned()]),
                });
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(*black_id),
                    message: ServerMessage::Chat(vec![self.message.to_owned()]),
                });
            }
            ChatDestination::GameSpectators(game, white_id, black_id) => {
                let mut games_public = self.chat_storage.games_public.write().unwrap();
                let entry = games_public.entry(game.clone()).or_default();
                entry.push(self.message.clone());
                messages.push(InternalServerMessage {
                    destination: MessageDestination::GameSpectators(
                        game.to_string(),
                        *white_id,
                        *black_id,
                    ),
                    message: ServerMessage::Chat(vec![self.message.to_owned()]),
                })
            }
            ChatDestination::User((id, _username)) => {
                let sender = self.message.message.user_id;
                self.chat_storage
                    .insert_or_update_direct_lookup(sender, *id);
                let user_to_user = UserToUser::new(*id, sender);
                let mut direct = self.chat_storage.direct.write().unwrap();
                let entry = direct.entry(user_to_user).or_default();
                entry.push(self.message.clone());
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(*id),
                    message: ServerMessage::Chat(vec![self.message.to_owned()]),
                })
            }
        };
        messages
    }
}
