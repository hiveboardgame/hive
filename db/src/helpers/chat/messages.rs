use super::{
    channels::ensure_chat_channel,
    read_receipts::advance_chat_read_in_channel,
    target::DbChatTarget,
    user_display_map,
};
use crate::{
    db_error::DbError,
    models::ChatMessage,
    schema::{chat_channels, chat_messages, users},
    DbConn,
};
use diesel::{dsl::max, prelude::*};
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use shared_types::{ChatHistoryPage, ChatMessage as SharedChatMessage};
use uuid::Uuid;

fn persisted_chat_turn(turn: Option<usize>) -> Result<Option<i32>, DbError> {
    turn.map(i32::try_from)
        .transpose()
        .map_err(|error| DbError::InvalidInput {
            info: "Chat turn is out of range".to_string(),
            error: error.to_string(),
        })
}

fn shared_message(row: ChatMessage, username: String) -> SharedChatMessage {
    SharedChatMessage {
        id: row.id,
        user_id: row.sender_id,
        username,
        timestamp: row.created_at,
        message: row.body,
        turn: row.turn.map(|turn| turn as usize),
    }
}

pub async fn insert_chat_message(
    conn: &mut DbConn<'_>,
    sender_id: Uuid,
    client_id: Uuid,
    target: &DbChatTarget,
    body: &str,
    turn: Option<usize>,
) -> Result<(ChatMessage, bool), DbError> {
    let turn = persisted_chat_turn(turn)?;
    (**conn)
        .transaction::<_, DbError, _>(async move |conn| {
            lock_active_chat_sender(conn, sender_id).await?;
            let channel_id = resolve_channel_id(conn, target).await?;
            insert_chat_message_in_channel(conn, channel_id, sender_id, client_id, body, turn).await
        })
        .await
}

async fn lock_active_chat_sender(
    conn: &mut AsyncPgConnection,
    sender_id: Uuid,
) -> Result<(), DbError> {
    users::table
        .filter(users::id.eq(sender_id))
        .filter(users::deleted.eq(false))
        .select(users::id)
        .for_update()
        .first::<Uuid>(conn)
        .await
        .map(|_| ())
        .map_err(DbError::from)
}

async fn resolve_channel_id(
    conn: &mut AsyncPgConnection,
    target: &DbChatTarget,
) -> Result<i64, DbError> {
    Ok(match target.channel_id() {
        Some(channel_id) => channel_id,
        None => ensure_chat_channel(conn, target).await?,
    })
}

async fn insert_chat_message_in_channel(
    conn: &mut AsyncPgConnection,
    channel_id: i64,
    sender_id: Uuid,
    client_id: Uuid,
    body: &str,
    turn: Option<i32>,
) -> Result<(ChatMessage, bool), DbError> {
    let inserted = diesel::insert_into(chat_messages::table)
        .values((
            chat_messages::channel_id.eq(channel_id),
            chat_messages::sender_id.eq(sender_id),
            chat_messages::body.eq(body),
            chat_messages::turn.eq(turn),
            chat_messages::client_id.eq(client_id),
        ))
        .on_conflict((chat_messages::sender_id, chat_messages::client_id))
        .do_nothing()
        .get_result::<ChatMessage>(conn)
        .await
        .optional()
        .map_err(DbError::from)?;
    if let Some(inserted) = inserted {
        return Ok((inserted, true));
    }

    let existing = chat_messages::table
        .filter(chat_messages::sender_id.eq(sender_id))
        .filter(chat_messages::client_id.eq(client_id))
        .first::<ChatMessage>(conn)
        .await
        .map_err(DbError::from)?;
    if existing.channel_id == channel_id && existing.body.as_str() == body && existing.turn == turn
    {
        Ok((existing, false))
    } else {
        Err(DbError::ChatClientIdConflict)
    }
}

pub async fn insert_chat_message_and_mark_sender_read(
    conn: &mut DbConn<'_>,
    sender_id: Uuid,
    client_id: Uuid,
    target: &DbChatTarget,
    body: &str,
    turn: Option<usize>,
) -> Result<(ChatMessage, bool), DbError> {
    let turn = persisted_chat_turn(turn)?;
    (**conn)
        .transaction::<_, DbError, _>(async move |conn| {
            lock_active_chat_sender(conn, sender_id).await?;
            let channel_id = resolve_channel_id(conn, target).await?;
            // Receipts use message IDs as read-through boundaries. Locking before allocating an
            // ID makes message ID order match commit order for receipt-tracked conversations.
            chat_channels::table
                .find(channel_id)
                .select(chat_channels::id)
                .for_no_key_update()
                .first::<i64>(conn)
                .await
                .map_err(DbError::from)?;
            let (row, inserted) =
                insert_chat_message_in_channel(conn, channel_id, sender_id, client_id, body, turn)
                    .await?;
            advance_chat_read_in_channel(conn, sender_id, row.channel_id, row.id).await?;
            Ok((row, inserted))
        })
        .await
}

pub async fn load_chat_history(
    conn: &mut DbConn<'_>,
    target: &DbChatTarget,
    before_message_id: Option<i64>,
    page_size: i64,
) -> Result<ChatHistoryPage, DbError> {
    let Some(channel_id) = target.channel_id() else {
        return Ok(ChatHistoryPage::default());
    };
    let mut query = chat_messages::table
        .filter(chat_messages::channel_id.eq(channel_id))
        .into_boxed();
    if let Some(before_message_id) = before_message_id {
        query = query.filter(chat_messages::id.lt(before_message_id));
    }
    let mut rows = query
        .order(chat_messages::id.desc())
        .limit(page_size.saturating_add(1))
        .load::<ChatMessage>(conn)
        .await
        .map_err(DbError::from)?;
    let has_more = rows.len() > page_size.max(0) as usize;
    rows.truncate(page_size.max(0) as usize);
    let next_before_message_id = if has_more {
        rows.last().map(|row| row.id)
    } else {
        None
    };
    rows.reverse();

    let users = user_display_map(conn, rows.iter().map(|row| row.sender_id)).await?;
    let messages = rows
        .into_iter()
        .map(|row| {
            let username = users
                .get(&row.sender_id)
                .cloned()
                .ok_or_else(|| DbError::NotFound {
                    reason: format!(
                        "Missing sender {} for chat message {}",
                        row.sender_id, row.id
                    ),
                })?;
            Ok(shared_message(row, username))
        })
        .collect::<Result<Vec<_>, DbError>>()?;
    Ok(ChatHistoryPage {
        messages,
        next_before_message_id,
        initial_unread_count: None,
    })
}

pub async fn latest_message_id_for_target(
    conn: &mut DbConn<'_>,
    target: &DbChatTarget,
) -> Result<i64, DbError> {
    let Some(channel_id) = target.channel_id() else {
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

#[cfg(test)]
mod tests {
    use super::persisted_chat_turn;
    use crate::db_error::DbError;

    #[test]
    fn chat_turn_conversion_rejects_overflow() {
        assert_eq!(
            persisted_chat_turn(Some(i32::MAX as usize)).unwrap(),
            Some(i32::MAX)
        );
        assert!(matches!(
            persisted_chat_turn(Some(usize::MAX)),
            Err(DbError::InvalidInput { info, .. }) if info == "Chat turn is out of range"
        ));
    }
}
