use super::{messages::latest_message_id_for_target, target::DbChatTarget};
use crate::{db_error::DbError, schema::chat_read_receipts, DbConn};
use diesel::{
    dsl::sql,
    prelude::*,
    sql_types::{BigInt, Uuid as SqlUuid},
};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

pub async fn mark_chat_read(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    target: &DbChatTarget,
    last_read_message_id: i64,
) -> Result<i64, DbError> {
    let Some(channel_id) = target.channel_id() else {
        return Ok(0);
    };
    let latest_message_id = latest_message_id_for_target(conn, target).await?;
    let last_read_message_id = last_read_message_id.clamp(0, latest_message_id);
    advance_chat_read_in_channel(conn, user_id, channel_id, last_read_message_id).await
}

pub async fn advance_chat_read_in_channel(
    conn: &mut AsyncPgConnection,
    user_id: Uuid,
    channel_id: i64,
    last_read_message_id: i64,
) -> Result<i64, DbError> {
    diesel::insert_into(chat_read_receipts::table)
        .values((
            chat_read_receipts::user_id.eq(user_id),
            chat_read_receipts::channel_id.eq(channel_id),
            chat_read_receipts::last_read_message_id.eq(last_read_message_id),
        ))
        .on_conflict((chat_read_receipts::user_id, chat_read_receipts::channel_id))
        .do_update()
        .set(chat_read_receipts::last_read_message_id.eq(sql::<BigInt>(
            "GREATEST(chat_read_receipts.last_read_message_id, EXCLUDED.last_read_message_id)",
        )))
        .returning(chat_read_receipts::last_read_message_id)
        .get_result(conn)
        .await
        .map_err(DbError::from)
}

pub async fn unread_chat_count_for_channel(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    channel_id: i64,
) -> Result<i64, DbError> {
    #[derive(QueryableByName)]
    struct UnreadCount {
        #[diesel(sql_type = BigInt)]
        count: i64,
    }

    diesel::sql_query(
        r#"
        SELECT COUNT(cm.id) AS count
        FROM chat_messages cm
        WHERE cm.channel_id = $1
          AND cm.sender_id <> $2
          AND cm.id > COALESCE((
              SELECT rr.last_read_message_id
              FROM chat_read_receipts rr
              WHERE rr.user_id = $2
                AND rr.channel_id = $1
          ), 0)
        "#,
    )
    .bind::<BigInt, _>(channel_id)
    .bind::<SqlUuid, _>(user_id)
    .get_result::<UnreadCount>(conn)
    .await
    .map(|row| row.count)
    .map_err(DbError::from)
}
