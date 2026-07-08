use super::{metrics, persist::PersistableChatMessage};
use crate::{
    common::ServerMessage,
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::{bail, Context, Result};
use db_lib::{helpers::DbChatTarget, DbConn};
use shared_types::{ChatDestination, ChatMessageContainer, ConversationKey};

pub struct ChatHandler {
    container: ChatMessageContainer,
    target: DbChatTarget,
}

impl ChatHandler {
    pub fn new(mut container: ChatMessageContainer, target: DbChatTarget) -> Self {
        container.time();
        let original_body = container.message.message.clone();
        container.message.normalize();
        if container.message.message != original_body {
            metrics::record_message_normalization();
        }
        Self { container, target }
    }

    pub async fn handle(&mut self, conn: &mut DbConn<'_>) -> Result<Vec<InternalServerMessage>> {
        if self.container.message.message.trim().is_empty() {
            bail!("normalized chat message is empty");
        }
        let persistable = PersistableChatMessage::from_container(&self.container);
        metrics::record_persist_attempt();
        let row = match persistable.insert(conn, &self.target).await {
            Ok(row) => {
                metrics::record_persist_success();
                row
            }
            Err(error) => {
                metrics::record_persist_failure();
                let snapshot = metrics::snapshot();
                log::error!(
                    "chat persist failed (attempts_total={}, successes_total={}, failures_total={}, normalizations_total={}): {}",
                    snapshot.persist_attempts_total,
                    snapshot.persist_successes_total,
                    snapshot.persist_failures_total,
                    snapshot.message_normalizations_total,
                    error,
                );
                return Err(error.into());
            }
        };
        self.container.message.id = Some(row.id);
        self.container.message.timestamp = Some(row.created_at);
        let message = ServerMessage::Chat(self.container.clone());
        Ok(self
            .destinations()?
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
                    .target
                    .game
                    .as_ref()
                    .context("missing game metadata for players chat")?;
                vec![
                    MessageDestination::User(game.white_id),
                    MessageDestination::User(game.black_id),
                ]
            }
            ChatDestination::GameSpectators(game_id) => {
                let game = self
                    .target
                    .game
                    .as_ref()
                    .context("missing game metadata for spectator chat")?;
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
                destinations.push(MessageDestination::ChatSubscribers(
                    ConversationKey::game_spectators(game_id),
                ));
                destinations
            }
            ChatDestination::User((other_id, _)) => vec![
                MessageDestination::User(*other_id),
                MessageDestination::User(self.container.message.user_id),
            ],
            ChatDestination::Global => vec![MessageDestination::Global],
        })
    }
}
