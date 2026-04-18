use db_lib::{
    db_error::DbError,
    helpers::{is_blocked, is_tournament_participant},
    models::{Game, Tournament, User},
    DbConn,
};
use shared_types::{
    ConversationKey,
    GameId,
    GameThread,
    PersistentChannelKey,
};
use uuid::Uuid;

#[derive(Debug)]
pub enum ChatSendAccessError {
    BadRequest(&'static str),
    Forbidden(&'static str),
    NotFound(&'static str),
    Internal {
        context: &'static str,
        error: DbError,
    },
}

fn map_not_found(
    err: DbError,
    not_found_message: &'static str,
    context: &'static str,
) -> ChatSendAccessError {
    match err {
        DbError::NotFound { .. } => ChatSendAccessError::NotFound(not_found_message),
        other => ChatSendAccessError::Internal {
            context,
            error: other,
        },
    }
}

async fn load_game_or_404(
    conn: &mut DbConn<'_>,
    game_id: &GameId,
) -> Result<Game, ChatSendAccessError> {
    Game::find_by_game_id(game_id, conn)
        .await
        .map_err(|e| map_not_found(e, "Game not found", "loading game"))
}

pub async fn authorize_chat_send_and_resolve_channel_key(
    conn: &mut DbConn<'_>,
    sender_id: Uuid,
    sender_is_admin: bool,
    conversation_key: &ConversationKey,
) -> Result<PersistentChannelKey, ChatSendAccessError> {
    let resolved_channel_key = conversation_key
        .persistent_key(Some(sender_id))
        .ok_or(ChatSendAccessError::BadRequest("Invalid channel"))?;
    match conversation_key {
        ConversationKey::Direct(other_id) => {
            if *other_id == sender_id {
                return Err(ChatSendAccessError::BadRequest(
                    "Direct messages to yourself are not supported",
                ));
            }
            User::find_by_uuid(other_id, conn)
                .await
                .map_err(|e| match e {
                    DbError::NotFound { .. } => {
                        ChatSendAccessError::Forbidden("You cannot send messages to this user")
                    }
                    other => ChatSendAccessError::Internal {
                        context: "checking direct-message recipient",
                        error: other,
                    },
                })?;

            let recipient_blocked_sender =
                is_blocked(conn, *other_id, sender_id)
                    .await
                    .map_err(|e| ChatSendAccessError::Internal {
                        context: "checking direct-message block status",
                        error: e,
                    })?;
            if recipient_blocked_sender {
                return Err(ChatSendAccessError::Forbidden(
                    "You cannot send messages to this user",
                ));
            }

            Ok(resolved_channel_key)
        }
        ConversationKey::Global => {
            if !sender_is_admin {
                return Err(ChatSendAccessError::Forbidden("Global chat requires admin"));
            }
            Ok(resolved_channel_key)
        }
        ConversationKey::Tournament(tournament_id) => {
            Tournament::from_nanoid(&tournament_id.0, conn)
                .await
                .map_err(|e| map_not_found(e, "Tournament not found", "loading tournament"))?;

            let can_chat = sender_is_admin
                || is_tournament_participant(conn, sender_id, &tournament_id.0)
                    .await
                    .map_err(|e| ChatSendAccessError::Internal {
                        context: "checking tournament membership",
                        error: e,
                    })?;
            if !can_chat {
                return Err(ChatSendAccessError::Forbidden(
                    "Only tournament participants and organizers can send messages",
                ));
            }
            Ok(resolved_channel_key)
        }
        ConversationKey::Game { game_id, thread } => {
            let game = load_game_or_404(conn, game_id).await?;
            let is_player = sender_id == game.white_id || sender_id == game.black_id;
            match thread {
                GameThread::Players => {
                    if !is_player {
                        return Err(ChatSendAccessError::Forbidden(
                            "Only players can send to players chat",
                        ));
                    }
                }
                GameThread::Spectators => {
                    if is_player && !game.finished {
                        return Err(ChatSendAccessError::Forbidden(
                            "Players cannot send to spectators chat while the game is ongoing",
                        ));
                    }
                }
            }
            Ok(resolved_channel_key)
        }
    }
}
