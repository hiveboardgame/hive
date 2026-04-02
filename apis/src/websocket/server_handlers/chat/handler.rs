use std::sync::Arc;

use crate::{
    common::ServerMessage,
    websocket::{
        messages::{InternalServerMessage, MessageDestination},
        server_handlers::chat::persist::PersistableChatMessage,
        WebsocketData,
    },
};
use db_lib::{get_conn, helpers::insert_chat_message, DbPool};
use shared_types::{chat_channel, ChatDestination, ChatMessageContainer};

pub struct ChatHandler {
    container: ChatMessageContainer,
    data: Arc<WebsocketData>,
    pool: DbPool,
}

impl ChatHandler {
    pub fn new(
        mut container: ChatMessageContainer,
        data: Arc<WebsocketData>,
        pool: DbPool,
    ) -> Self {
        container.time();
        Self {
            container,
            data,
            pool,
        }
    }

    pub fn handle(&self) -> Vec<InternalServerMessage> {
        let mut messages = Vec::new();
        match &self.container.destination {
            ChatDestination::TournamentLobby(tournament_id) => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Tournament(tournament_id.clone()),
                    message: ServerMessage::Chat(vec![self.container.to_owned()]),
                })
            }
            ChatDestination::GamePlayers(_game_id, white_id, black_id) => {
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
                messages.push(InternalServerMessage {
                    destination: MessageDestination::GameSpectators(
                        game.clone(),
                        *white_id,
                        *black_id,
                    ),
                    message: ServerMessage::Chat(vec![self.container.to_owned()]),
                })
            }
            ChatDestination::User((other_id, _username)) => {
                // Recipient
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(*other_id),
                    message: ServerMessage::Chat(vec![self.container.to_owned()]),
                });
                // Sender (echo so their thread updates immediately)
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(self.container.message.user_id),
                    message: ServerMessage::Chat(vec![self.container.to_owned()]),
                });
            }
            ChatDestination::Global => messages.push(InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Chat(vec![self.container.to_owned()]),
            }),
        };

        // Update in-memory recent cache (last 50 per channel)
        let (channel_type, channel_id) = chat_channel(&self.container.destination, self.container.message.user_id);
        self.data.chat_storage.push_recent(channel_type, &channel_id, self.container.clone());

        // Persist to Postgres in a spawned task (do not block the response)
        let persistable = PersistableChatMessage::from_container(&self.container);
        let pool = self.pool.clone();
        actix_rt::spawn(async move {
            if let Ok(mut conn) = get_conn(&pool).await {
                if let Err(e) = insert_chat_message(&mut conn, persistable.as_new()).await {
                    log::error!("chat persist: insert failed: {}", e);
                }
            } else {
                log::error!("chat persist: failed to get connection");
            }
        });

        messages
    }
}
