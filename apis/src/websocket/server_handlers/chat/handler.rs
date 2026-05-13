use std::{collections::HashSet, sync::Arc};

use anyhow::{Context, Result};

use super::{metrics, persist::PersistableChatMessage};
use crate::{
    common::ServerMessage,
    websocket::{
        messages::{InternalServerMessage, MessageDestination},
        WebsocketData,
    },
};
use db_lib::{
    get_conn,
    helpers::{get_user_ids_who_muted_tournament, insert_chat_message},
    models::{Game, Tournament},
    DbPool,
};
use shared_types::{ChatDestination, ChatMessageContainer, PersistentChannelKey, TournamentId};
use uuid::Uuid;

pub struct ChatHandler {
    container: ChatMessageContainer,
    channel_key: PersistentChannelKey,
    data: Arc<WebsocketData>,
    pool: DbPool,
}

impl ChatHandler {
    pub fn new(
        mut container: ChatMessageContainer,
        channel_key: PersistentChannelKey,
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
                for user_id in self.tournament_chat_recipients(tournament_id).await? {
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::User(user_id),
                        message: ServerMessage::Chat(vec![self.container.to_owned()]),
                    });
                }
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

        let persistable =
            PersistableChatMessage::from_container(&self.container, &self.channel_key);
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
        self.data.chat_storage.push_recent(
            self.channel_key.channel_type.as_str(),
            &self.channel_key.channel_id,
            self.container.clone(),
        );

        Ok(messages)
    }

    async fn tournament_chat_recipients(&self, tournament_id: &TournamentId) -> Result<Vec<Uuid>> {
        let mut conn = get_conn(&self.pool)
            .await
            .context("loading DB connection for tournament chat fanout")?;
        let tournament = Tournament::from_nanoid(&tournament_id.0, &mut conn)
            .await
            .context("loading tournament for chat fanout")?;
        let muted_ids = get_user_ids_who_muted_tournament(&mut conn, tournament.id)
            .await
            .context("loading tournament chat mutes")?;
        let sender_id = self.container.message.user_id;
        let mut user_ids = HashSet::new();

        for player in tournament
            .players(&mut conn)
            .await
            .context("loading tournament players for chat fanout")?
        {
            user_ids.insert(player.id);
        }
        for organizer in tournament
            .organizers(&mut conn)
            .await
            .context("loading tournament organizers for chat fanout")?
        {
            user_ids.insert(organizer.id);
        }
        user_ids.insert(sender_id);

        Ok(user_ids
            .into_iter()
            .filter(|user_id| *user_id == sender_id || !muted_ids.contains(user_id))
            .collect())
    }
}
