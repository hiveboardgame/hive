use anyhow::{Context, Result};

use super::{metrics, persist::PersistableChatMessage};
use crate::{
    chat::access::ResolvedChatChannel,
    common::ServerMessage,
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use db_lib::{helpers::insert_chat_message, DbConn};
use shared_types::{ChatDestination, ChatMessageContainer};

pub struct ChatHandler {
    container: ChatMessageContainer,
    resolved_channel: ResolvedChatChannel,
}

impl ChatHandler {
    pub fn new(
        mut container: ChatMessageContainer,
        resolved_channel: ResolvedChatChannel,
    ) -> Self {
        container.time();
        let original_body = container.message.message.clone();
        container.message.normalize();
        if container.message.message != original_body {
            metrics::record_message_normalization();
        }
        Self {
            container,
            resolved_channel,
        }
    }

    pub async fn handle(&self, conn: &mut DbConn<'_>) -> Result<Vec<InternalServerMessage>> {
        let destinations = self.destinations()?;

        let persistable = PersistableChatMessage::from_container(
            &self.container,
            &self.resolved_channel.channel_key,
            self.resolved_channel.game.as_ref().map(|game| game.id),
        );
        metrics::record_persist_attempt();
        if let Err(error) = insert_chat_message(conn, persistable.as_new()).await {
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

        let message = ServerMessage::Chat(vec![self.container.to_owned()]);
        Ok(destinations
            .into_iter()
            .map(|destination| InternalServerMessage {
                destination,
                message: message.clone(),
            })
            .collect())
    }

    fn destinations(&self) -> Result<Vec<MessageDestination>> {
        Ok(match &self.container.destination {
            ChatDestination::TournamentLobby(tournament_id) => {
                vec![MessageDestination::Tournament(
                    tournament_id.clone(),
                    Some(self.container.message.user_id),
                )]
            }
            ChatDestination::GamePlayers(_) => {
                let game = self
                    .resolved_channel
                    .game
                    .as_ref()
                    .context("missing players chat fanout metadata")?;
                vec![
                    MessageDestination::User(game.white_id),
                    MessageDestination::User(game.black_id),
                ]
            }
            ChatDestination::GameSpectators(game_id) => {
                let game = self
                    .resolved_channel
                    .game
                    .as_ref()
                    .context("missing spectators chat fanout metadata")?;
                let mut destinations = Vec::new();
                if game.finished {
                    destinations.push(MessageDestination::User(game.white_id));
                    destinations.push(MessageDestination::User(game.black_id));
                }
                destinations.push(MessageDestination::GameSpectators(
                    game_id.clone(),
                    game.white_id,
                    game.black_id,
                ));
                destinations
            }
            ChatDestination::User((other_id, _username)) => vec![
                MessageDestination::User(*other_id),
                MessageDestination::User(self.container.message.user_id),
            ],
            ChatDestination::Global => vec![MessageDestination::Global],
        })
    }
}
