use std::sync::Arc;

use anyhow::{Context, Result};

use crate::{
    chat::ChannelKey,
    common::ServerMessage,
    websocket::{
        messages::{InternalServerMessage, MessageDestination},
        server_handlers::chat::{metrics, persist::PersistableChatMessage},
        WebsocketData,
    },
};
use db_lib::{get_conn, helpers::insert_chat_message, models::Game, DbPool};
use shared_types::{ChatDestination, ChatMessageContainer};

pub struct ChatHandler {
    container: ChatMessageContainer,
    channel_key: ChannelKey,
    data: Arc<WebsocketData>,
    pool: DbPool,
}

impl ChatHandler {
    pub fn new(
        mut container: ChatMessageContainer,
        channel_key: ChannelKey,
        data: Arc<WebsocketData>,
        pool: DbPool,
    ) -> Self {
        container.time();
        let original_body = container.message.message.clone();
        container.message.normalize();
        if container.message.message != original_body {
            metrics::record_message_normalization();
        }
        Self {
            container,
            channel_key,
            data,
            pool,
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut messages = Vec::new();
        match &self.container.destination {
            ChatDestination::TournamentLobby(tournament_id) => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Tournament(tournament_id.clone()),
                    message: ServerMessage::Chat(vec![self.container.to_owned()]),
                })
            }
            ChatDestination::GamePlayers(game_id) => {
                let mut conn = get_conn(&self.pool)
                    .await
                    .context("loading DB connection for players chat fanout")?;
                let game = Game::find_by_game_id(game_id, &mut conn)
                    .await
                    .context("loading game for players chat fanout")?;
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(game.white_id),
                    message: ServerMessage::Chat(vec![self.container.to_owned()]),
                });
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(game.black_id),
                    message: ServerMessage::Chat(vec![self.container.to_owned()]),
                });
            }
            ChatDestination::GameSpectators(game_id) => {
                let mut conn = get_conn(&self.pool)
                    .await
                    .context("loading DB connection for spectators chat fanout")?;
                let game = Game::find_by_game_id(game_id, &mut conn)
                    .await
                    .context("loading game for spectators chat fanout")?;
                messages.push(InternalServerMessage {
                    destination: MessageDestination::GameSpectators(
                        game_id.clone(),
                        game.white_id,
                        game.black_id,
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

        let persistable = PersistableChatMessage::from_container(
            &self.container,
            &self.channel_key,
        );
        metrics::record_persist_attempt();
        let mut conn = get_conn(&self.pool)
            .await
            .context("getting chat persistence connection")?;
        if let Err(error) = insert_chat_message(&mut conn, persistable.as_new()).await {
            metrics::record_persist_failure();
            let snapshot = metrics::snapshot();
            log::error!(
                "chat persist failed (attempts_total={}, successes_total={}, failures_total={}, normalizations_total={}): {}",
                snapshot.persist_attempts_total,
                snapshot.persist_successes_total,
                snapshot.persist_failures_total,
                snapshot.message_normalizations_total,
                error
            );
            return Err(error.into());
        }
        metrics::record_persist_success();

        // Update the in-memory recent cache only after persistence succeeds.
        self.data
            .chat_storage
            .push_recent(
                self.channel_key.channel_type.as_str(),
                &self.channel_key.channel_id,
                self.container.clone(),
            );

        Ok(messages)
    }
}
