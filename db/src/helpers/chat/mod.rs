mod channels;
mod hub;
mod messages;
mod read_receipts;
mod target;

use crate::{db_error::DbError, schema::users, DbConn};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use shared_types::MESSAGES_HUB_SECTION_LIMIT;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub use hub::{
    chat_inbox_unread_states,
    get_dm_conversations_for_user,
    get_game_channels_for_user,
    get_tournament_channels_for_user,
};
pub use messages::{
    insert_chat_message,
    insert_chat_message_and_mark_sender_read,
    latest_message_id_for_target,
    load_chat_history,
};
pub use read_receipts::{mark_chat_read, unread_chat_count_for_channel};
pub(crate) use target::authorize_tournament_chat_access;
pub use target::{
    get_tournament_thread_data,
    load_game_chat_capabilities,
    resolve_chat_target,
    DbChatTarget,
};

const HUB_SECTION_LIMIT: i64 = MESSAGES_HUB_SECTION_LIMIT as i64;

pub(super) async fn user_display_map(
    conn: &mut DbConn<'_>,
    user_ids: impl IntoIterator<Item = Uuid>,
) -> Result<HashMap<Uuid, String>, DbError> {
    let user_ids: Vec<_> = user_ids
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }

    users::table
        .filter(users::id.eq_any(user_ids))
        .select((users::id, users::username, users::deleted))
        .load::<(Uuid, String, bool)>(conn)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(id, username, deleted)| {
                    (
                        id,
                        if deleted {
                            "Deleted user".to_string()
                        } else {
                            username
                        },
                    )
                })
                .collect()
        })
        .map_err(DbError::from)
}
