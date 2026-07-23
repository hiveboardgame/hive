use crate::{
    common::{ChatSendRequest, ServerMessage},
    notifications::{notify, ChatNotifyContext, Event},
    websocket::{
        messages::{
            GameSpectatorAudience,
            InternalServerMessage,
            MessageDestination,
            TournamentAudience,
        },
        WsHub,
        WsTelemetry,
    },
};
use anyhow::{Context, Result};
#[cfg(test)]
use chrono::Utc;
use db_lib::{
    db_error::DbError,
    helpers::{
        blockers_of_user,
        insert_chat_message,
        insert_chat_message_and_mark_sender_read,
        muted_tournament_chat_user_ids,
        DbChatTarget,
    },
    models::{ChatMessage as DbChatMessage, NotificationPreferences, Tournament},
    DbConn,
};
use shared_types::{ChatMessage, ChatMessageContainer, ConversationKey, GameThread, CHANNEL_PUSH};
use std::{collections::HashSet, sync::Arc};
use thiserror::Error;
use uuid::Uuid;

const CHAT_PREVIEW_MAX_CHARS: usize = 140;

fn chat_preview(body: &str) -> String {
    if body.chars().count() <= CHAT_PREVIEW_MAX_CHARS {
        body.to_string()
    } else {
        let truncated: String = body.chars().take(CHAT_PREVIEW_MAX_CHARS).collect();
        format!("{truncated}\u{2026}")
    }
}

#[derive(Debug, Error)]
pub enum ChatHandlerError {
    #[error("Chat client ID conflicts with an existing message")]
    ClientIdConflict,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

fn is_persist_failure(error: &DbError) -> bool {
    !matches!(
        error,
        DbError::ChatClientIdConflict | DbError::InvalidInput { .. }
    )
}

pub struct ChatHandler<'a> {
    request: ChatSendRequest,
    sender: (&'a str, Uuid),
    target: DbChatTarget,
    hub: Arc<WsHub>,
}

impl<'a> ChatHandler<'a> {
    pub fn new(
        request: ChatSendRequest,
        sender: (&'a str, Uuid),
        target: DbChatTarget,
        hub: Arc<WsHub>,
    ) -> Self {
        Self {
            request,
            sender,
            target,
            hub,
        }
    }

    pub async fn handle(
        &self,
        conn: &mut DbConn<'_>,
        telemetry: &WsTelemetry,
    ) -> std::result::Result<Vec<InternalServerMessage>, ChatHandlerError> {
        if self.request.body.trim().is_empty() {
            return Err(anyhow::anyhow!("normalized chat message is empty").into());
        }
        let (persisted, inserted) = match self.insert(conn).await {
            Ok(persisted) => persisted,
            Err(DbError::ChatClientIdConflict) => return Err(ChatHandlerError::ClientIdConflict),
            Err(error) => {
                if is_persist_failure(&error) {
                    telemetry.record_chat_persist_failure();
                }
                return Err(anyhow::Error::new(error)
                    .context("persisting chat message")
                    .into());
            }
        };
        if inserted {
            self.dispatch_notifications(conn).await;
        }
        self.messages_for_persisted(persisted, inserted)
    }

    /// The key a *recipient* (not the sender) would be subscribed under for
    /// this conversation. Symmetric for every target except `Direct`, whose
    /// `ConversationKey` names "the other party" and is therefore different
    /// depending on which side of the conversation is looking.
    fn conversation_key_for_recipient(&self) -> ConversationKey {
        match &self.target {
            DbChatTarget::Direct { .. } => ConversationKey::Direct(self.sender.1),
            _ => self.request.key.clone(),
        }
    }

    /// Push-notify this message's recipients, skipping anyone who blocked the
    /// sender, muted this tournament's chat, or has a browser tab currently
    /// subscribed-and-focused on this conversation.
    async fn dispatch_notifications(&self, conn: &mut DbConn<'_>) {
        let preview = chat_preview(&self.request.body);
        let recipient_key = self.conversation_key_for_recipient();

        let recipients: Vec<(Uuid, Option<ChatNotifyContext>)> = match &self.target {
            DbChatTarget::Direct { other_user_id, .. } => vec![(*other_user_id, None)],
            DbChatTarget::Game {
                thread: GameThread::Players,
                game,
                game_id,
                ..
            } => [game.white_id, game.black_id]
                .into_iter()
                .filter(|id| *id != self.sender.1)
                .map(|id| {
                    (
                        id,
                        Some(ChatNotifyContext::GamePlayers {
                            game_nanoid: game_id.0.clone(),
                            opponent: self.sender.0.to_string(),
                        }),
                    )
                })
                .collect(),
            // Spectators are an ephemeral, unauthenticated-allowed audience
            // with no persisted roster (unlike players/tournament/DM
            // participants), so there is no stable recipient list to push to.
            DbChatTarget::Game {
                thread: GameThread::Spectators,
                ..
            } => Vec::new(),
            DbChatTarget::Tournament {
                tournament_id,
                id: tournament_uuid,
                ..
            } => match self
                .tournament_recipients(tournament_id, *tournament_uuid, conn)
                .await
            {
                Ok(recipients) => recipients,
                Err(error) => {
                    log::warn!(
                        "chat notify: tournament lookup for {tournament_id:?} failed: {error}"
                    );
                    Vec::new()
                }
            },
            DbChatTarget::Global { .. } => {
                match NotificationPreferences::user_ids_with_general_chat_channel(
                    CHANNEL_PUSH,
                    conn,
                )
                .await
                {
                    Ok(ids) => ids
                        .into_iter()
                        .filter(|id| *id != self.sender.1)
                        .map(|id| (id, Some(ChatNotifyContext::Global)))
                        .collect(),
                    Err(error) => {
                        log::warn!(
                            "chat notify: general_chat push recipients lookup failed: {error}"
                        );
                        Vec::new()
                    }
                }
            }
        };

        let blockers: HashSet<Uuid> = blockers_of_user(conn, self.sender.1)
            .await
            .unwrap_or_else(|error| {
                log::warn!("chat notify: blockers lookup for {} failed: {error}", self.sender.1);
                Vec::new()
            })
            .into_iter()
            .collect();

        for (recipient, context) in recipients {
            if blockers.contains(&recipient) {
                continue;
            }
            if self.hub.has_focused_subscriber(recipient, &recipient_key) {
                continue;
            }
            match context {
                None => notify(Event::DirectMessage {
                    recipient,
                    sender: self.sender.0.to_string(),
                    preview: preview.clone(),
                }),
                Some(context) => notify(Event::ChatMessage {
                    recipient,
                    sender: self.sender.0.to_string(),
                    preview: preview.clone(),
                    context,
                }),
            }
        }
    }

    async fn tournament_recipients(
        &self,
        tournament_id: &shared_types::TournamentId,
        tournament_uuid: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<(Uuid, Option<ChatNotifyContext>)>, DbError> {
        let tournament = Tournament::find_by_tournament_id(tournament_id, conn).await?;
        let players = tournament.players(conn).await?;
        let muted: HashSet<Uuid> = muted_tournament_chat_user_ids(conn, tournament_uuid)
            .await?
            .into_iter()
            .collect();
        let recipients = players
            .into_iter()
            .filter(|player| player.id != self.sender.1 && !muted.contains(&player.id))
            .map(|player| {
                (
                    player.id,
                    Some(ChatNotifyContext::Tournament {
                        tournament_nanoid: tournament_id.0.clone(),
                        tournament_name: tournament.name.clone(),
                    }),
                )
            })
            .collect();
        Ok(recipients)
    }

    fn messages_for_persisted(
        &self,
        persisted: DbChatMessage,
        inserted: bool,
    ) -> std::result::Result<Vec<InternalServerMessage>, ChatHandlerError> {
        let client_id = persisted.client_id;
        let message = self.authoritative_message(persisted)?;
        let sender_container =
            |message| ChatMessageContainer::new(self.request.key.clone(), message, client_id);
        if !inserted {
            return Ok(vec![InternalServerMessage {
                destination: MessageDestination::User(self.sender.1),
                message: ServerMessage::Chat(sender_container(message)),
            }]);
        }
        Ok(match &self.target {
            DbChatTarget::Direct { other_user_id, .. } => vec![
                InternalServerMessage {
                    destination: MessageDestination::User(*other_user_id),
                    message: ServerMessage::Chat(ChatMessageContainer::new(
                        ConversationKey::Direct(self.sender.1),
                        message.clone(),
                        client_id,
                    )),
                },
                InternalServerMessage {
                    destination: MessageDestination::User(self.sender.1),
                    message: ServerMessage::Chat(sender_container(message)),
                },
            ],
            DbChatTarget::Tournament { tournament_id, .. } => vec![InternalServerMessage {
                destination: MessageDestination::Tournament {
                    tournament_id: tournament_id.clone(),
                    audience: TournamentAudience::Chat {
                        sender_id: self.sender.1,
                    },
                },
                message: ServerMessage::Chat(sender_container(message)),
            }],
            DbChatTarget::Game {
                thread: GameThread::Players,
                game,
                ..
            } => {
                let server_message = ServerMessage::Chat(sender_container(message));
                vec![
                    InternalServerMessage {
                        destination: MessageDestination::User(game.white_id),
                        message: server_message.clone(),
                    },
                    InternalServerMessage {
                        destination: MessageDestination::User(game.black_id),
                        message: server_message,
                    },
                ]
            }
            DbChatTarget::Game {
                game_id,
                thread: GameThread::Spectators,
                game,
                ..
            } => vec![InternalServerMessage {
                destination: MessageDestination::GameSpectators {
                    game_id: game_id.clone(),
                    white_id: game.white_id,
                    black_id: game.black_id,
                    audience: GameSpectatorAudience::SpectatorChat {
                        include_players: game.finished,
                    },
                },
                message: ServerMessage::Chat(sender_container(message)),
            }],
            DbChatTarget::Global { .. } => vec![InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Chat(sender_container(message)),
            }],
        })
    }

    async fn insert(&self, conn: &mut DbConn<'_>) -> Result<(DbChatMessage, bool), DbError> {
        if self.request.key.tracks_read_receipts() {
            insert_chat_message_and_mark_sender_read(
                conn,
                self.sender.1,
                self.request.client_id,
                &self.target,
                &self.request.body,
                self.request.turn,
            )
            .await
        } else {
            insert_chat_message(
                conn,
                self.sender.1,
                self.request.client_id,
                &self.target,
                &self.request.body,
                self.request.turn,
            )
            .await
        }
    }

    fn authoritative_message(&self, row: DbChatMessage) -> Result<ChatMessage> {
        let turn = row
            .turn
            .map(usize::try_from)
            .transpose()
            .context("persisted chat turn is negative")?;
        Ok(ChatMessage {
            id: row.id,
            user_id: row.sender_id,
            username: self.sender.0.to_string(),
            timestamp: row.created_at,
            message: row.body,
            turn,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{common::ChatSendRequest, websocket::WebsocketData};
    use shared_types::{ConversationKey, TournamentId};

    async fn test_hub() -> Arc<WsHub> {
        let pool = db_lib::get_pool("postgresql://test:test@127.0.0.1:9/test")
            .await
            .expect("bb8 pool builds without connecting");
        WsHub::new(Arc::new(WebsocketData::default()), pool)
    }

    fn persisted(sender_id: Uuid, client_id: Uuid) -> DbChatMessage {
        DbChatMessage {
            id: 42,
            channel_id: 7,
            sender_id,
            body: "persisted".to_string(),
            turn: None,
            created_at: Utc::now(),
            client_id,
        }
    }

    #[tokio::test]
    async fn direct_fanout_uses_audience_relative_keys_with_one_persisted_identity() {
        let sender_id = Uuid::new_v4();
        let peer_id = Uuid::new_v4();
        let client_id = Uuid::new_v4();
        let (low_id, high_id) = (sender_id.min(peer_id), sender_id.max(peer_id));
        let handler = ChatHandler::new(
            ChatSendRequest {
                key: ConversationKey::Direct(peer_id),
                client_id,
                body: "persisted".to_string(),
                turn: None,
            },
            ("sender", sender_id),
            DbChatTarget::Direct {
                other_user_id: peer_id,
                channel_id: Some(7),
                low_id,
                high_id,
            },
            test_hub().await,
        );

        let messages = handler
            .messages_for_persisted(persisted(sender_id, client_id), true)
            .expect("build direct fanout");
        let [recipient, sender] = messages.as_slice() else {
            panic!("direct fanout should contain recipient and sender messages");
        };
        assert!(matches!(recipient.destination, MessageDestination::User(id) if id == peer_id));
        assert!(matches!(sender.destination, MessageDestination::User(id) if id == sender_id));
        let ServerMessage::Chat(recipient) = &recipient.message else {
            panic!("recipient should receive chat");
        };
        let ServerMessage::Chat(sender) = &sender.message else {
            panic!("sender should receive chat");
        };
        assert_eq!(recipient.key, ConversationKey::Direct(sender_id));
        assert_eq!(sender.key, ConversationKey::Direct(peer_id));
        assert_eq!(recipient.message.id, sender.message.id);
        assert_eq!(recipient.client_id, sender.client_id);
    }

    #[tokio::test]
    async fn idempotent_retry_acknowledges_only_the_sender() {
        let sender_id = Uuid::new_v4();
        let client_id = Uuid::new_v4();
        let tournament_id = TournamentId("retry-ack".to_string());
        let handler = ChatHandler::new(
            ChatSendRequest {
                key: ConversationKey::Tournament(tournament_id.clone()),
                client_id,
                body: "persisted".to_string(),
                turn: None,
            },
            ("sender", sender_id),
            DbChatTarget::Tournament {
                tournament_id,
                channel_id: Some(7),
                id: Uuid::new_v4(),
            },
            test_hub().await,
        );

        let messages = handler
            .messages_for_persisted(persisted(sender_id, client_id), false)
            .expect("build retry acknowledgement");
        let [ack] = messages.as_slice() else {
            panic!("retry should emit one acknowledgement");
        };
        assert!(matches!(ack.destination, MessageDestination::User(id) if id == sender_id));
    }
}
