use super::{messages::latest_message_id_for_target, target::DbChatTarget};
use crate::{db_error::DbError, models::NewChatReadReceipt, schema::chat_read_receipts, DbConn};
use chrono::Utc;
use diesel::{dsl::sql, prelude::*, sql_types::BigInt};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

pub async fn mark_chat_read(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    target: &DbChatTarget,
    last_read_message_id: i64,
) -> Result<i64, DbError> {
    let Some(channel_id) = target.channel_id else {
        return Ok(0);
    };
    let latest_message_id = latest_message_id_for_target(conn, target).await?;
    let last_read_message_id = last_read_message_id.clamp(0, latest_message_id);
    let new = NewChatReadReceipt {
        user_id,
        channel_id,
        last_read_message_id,
        updated_at: Utc::now(),
    };

    diesel::insert_into(chat_read_receipts::table)
        .values(new)
        .on_conflict((chat_read_receipts::user_id, chat_read_receipts::channel_id))
        .do_update()
        .set((
            chat_read_receipts::last_read_message_id.eq(sql::<BigInt>(
                "GREATEST(chat_read_receipts.last_read_message_id, EXCLUDED.last_read_message_id)",
            )),
            chat_read_receipts::updated_at.eq(Utc::now()),
        ))
        .returning(chat_read_receipts::last_read_message_id)
        .get_result(conn)
        .await
        .map_err(DbError::from)
}
