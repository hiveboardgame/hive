use db_lib::{
    db_error::DbError,
    helpers::{
        get_game_chat_participants_and_finished,
        get_tournament_chat_capabilities,
        is_blocked,
    },
    models::{Game, User},
    DbConn,
};
use shared_types::{
    ConversationKey,
    GameChatCapabilities,
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

fn game_chat_capabilities(game: &Game, user_id: Uuid) -> GameChatCapabilities {
    GameChatCapabilities::new(
        user_id == game.white_id || user_id == game.black_id,
        game.finished,
    )
}

pub async fn load_game_chat_capabilities(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    game_id: &GameId,
) -> Result<Option<GameChatCapabilities>, DbError> {
    match get_game_chat_participants_and_finished(conn, game_id).await {
        Ok((white_id, black_id, finished)) => Ok(Some(GameChatCapabilities::new(
            user_id == white_id || user_id == black_id,
            finished,
        ))),
        Err(DbError::NotFound { .. }) => Ok(None),
        Err(error) => Err(error),
    }
}

pub async fn can_user_read_chat(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    conversation_key: &ConversationKey,
) -> Result<bool, DbError> {
    match conversation_key {
        ConversationKey::Direct(other_user_id) => Ok(*other_user_id != user_id),
        ConversationKey::Global => Ok(true),
        ConversationKey::Tournament(tournament_id) => {
            Ok(
                get_tournament_chat_capabilities(conn, user_id, &tournament_id.0)
                    .await?
                    .can_read(),
            )
        }
        ConversationKey::Game { game_id, thread } => {
            Ok(load_game_chat_capabilities(conn, user_id, game_id)
                .await?
                .is_some_and(|access| access.can_read(*thread)))
        }
    }
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
                is_blocked(conn, *other_id, sender_id).await.map_err(|e| {
                    ChatSendAccessError::Internal {
                        context: "checking direct-message block status",
                        error: e,
                    }
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
            let access = get_tournament_chat_capabilities(conn, sender_id, &tournament_id.0)
                .await
                .map_err(|error| {
                    map_not_found(
                        error,
                        "Tournament not found",
                        "loading tournament chat capabilities",
                    )
                })?;
            if !access.can_send() {
                return Err(ChatSendAccessError::Forbidden(
                    "Only tournament participants and organizers can send messages",
                ));
            }
            Ok(resolved_channel_key)
        }
        ConversationKey::Game { game_id, thread } => {
            let game = load_game_or_404(conn, game_id).await?;
            let access = game_chat_capabilities(&game, sender_id);
            if !access.can_send(*thread) {
                let message = if *thread == GameThread::Players {
                    "Only players can send to players chat"
                } else {
                    "Players cannot send to spectators chat while the game is ongoing"
                };
                return Err(ChatSendAccessError::Forbidden(message));
            }
            Ok(resolved_channel_key)
        }
    }
}
