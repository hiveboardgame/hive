use db_lib::{
    db_error::DbError,
    helpers::{is_user_blocked, resolve_chat_target, DbChatTarget},
    models::User,
    DbConn,
};
use shared_types::{ConversationKey, GameThread};
use uuid::Uuid;

#[derive(Debug)]
pub enum ChatAccessError {
    Denied,
    Internal {
        context: &'static str,
        error: DbError,
    },
}

fn map_resolve_error(error: DbError) -> ChatAccessError {
    match error {
        DbError::InvalidInput { .. } | DbError::NotFound { .. } | DbError::Unauthorized => {
            ChatAccessError::Denied
        }
        other => ChatAccessError::Internal {
            context: "resolving chat target",
            error: other,
        },
    }
}

pub fn allows_anonymous_chat_read(key: &ConversationKey) -> bool {
    matches!(
        key,
        ConversationKey::Game {
            thread: GameThread::Spectators,
            ..
        }
    )
}

pub async fn authorize_chat_read(
    conn: &mut DbConn<'_>,
    user_id: Option<Uuid>,
    key: &ConversationKey,
) -> Result<DbChatTarget, ChatAccessError> {
    if user_id.is_none() && !allows_anonymous_chat_read(key) {
        return Err(ChatAccessError::Denied);
    }
    resolve_chat_target(conn, user_id, key)
        .await
        .map_err(map_resolve_error)
}

pub async fn authorize_chat_send(
    conn: &mut DbConn<'_>,
    sender_id: Uuid,
    sender_is_admin: bool,
    key: &ConversationKey,
) -> Result<DbChatTarget, ChatAccessError> {
    let target = authorize_chat_read(conn, Some(sender_id), key).await?;

    match key {
        ConversationKey::Direct(other_id) => {
            User::find_active_by_uuid(other_id, conn)
                .await
                .map_err(map_resolve_error)?;
            let blocked = is_user_blocked(conn, *other_id, sender_id)
                .await
                .map_err(|error| ChatAccessError::Internal {
                    context: "checking direct-message block status",
                    error,
                })?;
            (!blocked).then_some(target).ok_or(ChatAccessError::Denied)
        }
        ConversationKey::Global => sender_is_admin
            .then_some(target)
            .ok_or(ChatAccessError::Denied),
        ConversationKey::Tournament(_) | ConversationKey::Game { .. } => Ok(target),
    }
}
