use db_lib::{
    db_error::DbError,
    helpers::{can_user_read_target, resolve_chat_target, DbChatTarget},
    models::UserBlock,
    DbConn,
};
use shared_types::{ConversationKey, GameThread};
use uuid::Uuid;

#[derive(Debug)]
pub enum ChatAccessError {
    BadRequest(&'static str),
    Forbidden(&'static str),
    NotFound(&'static str),
    Internal {
        context: &'static str,
        error: DbError,
    },
}

fn map_resolve_error(error: DbError, not_found: &'static str) -> ChatAccessError {
    match error {
        DbError::InvalidInput { .. } => ChatAccessError::BadRequest("Invalid chat channel"),
        DbError::NotFound { .. } => ChatAccessError::NotFound(not_found),
        other => ChatAccessError::Internal {
            context: "resolving chat target",
            error: other,
        },
    }
}

pub fn allows_anonymous_chat_read(key: &ConversationKey) -> bool {
    matches!(
        key,
        ConversationKey::Global
            | ConversationKey::Game {
                thread: GameThread::Spectators,
                ..
            }
    )
}

pub async fn can_anonymous_read_chat(
    conn: &mut DbConn<'_>,
    key: &ConversationKey,
) -> Result<(bool, DbChatTarget), ChatAccessError> {
    let user_id = Uuid::nil();
    let target = resolve_chat_target(conn, user_id, key)
        .await
        .map_err(|error| map_resolve_error(error, "Chat not found"))?;
    Ok((can_user_read_target(user_id, &target), target))
}

pub async fn can_user_read_chat(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    key: &ConversationKey,
) -> Result<(bool, DbChatTarget), ChatAccessError> {
    let target = resolve_chat_target(conn, user_id, key)
        .await
        .map_err(|error| map_resolve_error(error, "Chat not found"))?;
    Ok((can_user_read_target(user_id, &target), target))
}

pub async fn authorize_chat_send(
    conn: &mut DbConn<'_>,
    sender_id: Uuid,
    sender_is_admin: bool,
    key: &ConversationKey,
) -> Result<DbChatTarget, ChatAccessError> {
    let target = resolve_chat_target(conn, sender_id, key)
        .await
        .map_err(|error| map_resolve_error(error, "Chat not found"))?;

    match key {
        ConversationKey::Direct(other_id) => {
            let blocked = UserBlock::is_blocked(conn, *other_id, sender_id)
                .await
                .map_err(|error| ChatAccessError::Internal {
                    context: "checking direct-message block status",
                    error,
                })?;
            if blocked {
                Err(ChatAccessError::Forbidden(
                    "You cannot send messages to this user",
                ))
            } else {
                Ok(target)
            }
        }
        ConversationKey::Global => {
            if sender_is_admin {
                Ok(target)
            } else {
                Err(ChatAccessError::Forbidden("Global chat requires admin"))
            }
        }
        ConversationKey::Tournament(_) => {
            if target
                .tournament
                .as_ref()
                .is_some_and(|tournament| tournament.access.can_send())
            {
                Ok(target)
            } else {
                Err(ChatAccessError::Forbidden(
                    "Only tournament participants and organizers can send messages",
                ))
            }
        }
        ConversationKey::Game { thread, .. } => {
            if can_user_read_target(sender_id, &target) {
                Ok(target)
            } else {
                let reason = if *thread == GameThread::Players {
                    "Only players can send to players chat"
                } else {
                    "Players cannot send to spectators chat while the game is ongoing"
                };
                Err(ChatAccessError::Forbidden(reason))
            }
        }
    }
}
