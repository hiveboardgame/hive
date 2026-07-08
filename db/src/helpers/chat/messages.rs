use super::{channels::ensure_chat_channel, target::DbChatTarget, user_display_map};
use crate::{
    db_error::DbError,
    models::{ChatMessage, NewChatMessage},
    schema::chat_messages,
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{dsl::max, prelude::*};
use diesel_async::RunQueryDsl;
use shared_types::ChatMessage as SharedChatMessage;
use uuid::Uuid;

fn shared_message(row: ChatMessage, username: String) -> SharedChatMessage {
    SharedChatMessage {
        id: Some(row.id),
        user_id: row.sender_id,
        username,
        timestamp: Some(row.created_at),
        message: row.body,
        turn: row.turn.map(|turn| turn as usize),
    }
}

pub async fn insert_chat_message(
    conn: &mut DbConn<'_>,
    sender_id: Uuid,
    target: &DbChatTarget,
    body: &str,
    turn: Option<usize>,
    created_at: DateTime<Utc>,
) -> Result<ChatMessage, DbError> {
    let channel_id = ensure_chat_channel(conn, target).await?;
    let turn = turn.and_then(|turn| i32::try_from(turn).ok());
    let new = NewChatMessage {
        channel_id,
        sender_id,
        body,
        turn,
        created_at,
    };

    diesel::insert_into(chat_messages::table)
        .values(new)
        .get_result(conn)
        .await
        .map_err(DbError::from)
}

pub async fn load_chat_history(
    conn: &mut DbConn<'_>,
    target: &DbChatTarget,
    limit: i64,
) -> Result<Vec<SharedChatMessage>, DbError> {
    let Some(channel_id) = target.channel_id else {
        return Ok(Vec::new());
    };
    let mut rows = chat_messages::table
        .filter(chat_messages::channel_id.eq(channel_id))
        .order(chat_messages::id.desc())
        .limit(limit)
        .load::<ChatMessage>(conn)
        .await
        .map_err(DbError::from)?;
    rows.reverse();

    let users = user_display_map(conn, rows.iter().map(|row| row.sender_id)).await?;
    rows.into_iter()
        .map(|row| {
            let username = users
                .get(&row.sender_id)
                .ok_or_else(|| DbError::NotFound {
                    reason: format!(
                        "Missing sender {} for chat message {}",
                        row.sender_id, row.id
                    ),
                })?
                .display_name();
            Ok(shared_message(row, username))
        })
        .collect()
}

pub async fn latest_message_id_for_target(
    conn: &mut DbConn<'_>,
    target: &DbChatTarget,
) -> Result<i64, DbError> {
    let Some(channel_id) = target.channel_id else {
        return Ok(0);
    };
    chat_messages::table
        .filter(chat_messages::channel_id.eq(channel_id))
        .select(max(chat_messages::id))
        .first::<Option<i64>>(conn)
        .await
        .map(|latest| latest.unwrap_or(0))
        .map_err(DbError::from)
}
