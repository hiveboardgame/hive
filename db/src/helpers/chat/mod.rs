mod channels;
mod hub;
mod messages;
mod read_receipts;
mod target;

use crate::{db_error::DbError, schema::users, DbConn};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub use hub::{
    get_dm_conversations_for_user,
    get_game_channels_for_user,
    get_tournament_channels_for_user,
    unread_states_for_messages_hub_channels,
};
pub use messages::{insert_chat_message, latest_message_id_for_target, load_chat_history};
pub use read_receipts::mark_chat_read;
pub use target::{
    can_user_read_target,
    get_tournament_chat_capabilities,
    get_tournament_thread_data,
    load_game_chat_capabilities,
    resolve_chat_target,
    DbChatTarget,
};

const HUB_SECTION_LIMIT: i64 = 50;

#[derive(Clone, Debug)]
pub(super) struct UserDisplay {
    pub username: String,
    pub deleted: bool,
}

impl UserDisplay {
    pub fn display_name(&self) -> String {
        if self.deleted {
            "Deleted user".to_string()
        } else {
            self.username.clone()
        }
    }
}

pub(super) async fn user_display_map(
    conn: &mut DbConn<'_>,
    user_ids: impl IntoIterator<Item = Uuid>,
) -> Result<HashMap<Uuid, UserDisplay>, DbError> {
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
                .map(|(id, username, deleted)| (id, UserDisplay { username, deleted }))
                .collect()
        })
        .map_err(DbError::from)
}
