use super::target::{lookup_channel_id, DbChatTarget};
use crate::{db_error::DbError, schema::chat_channels};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

pub(super) async fn ensure_chat_channel(
    conn: &mut AsyncPgConnection,
    target: &DbChatTarget,
) -> Result<i64, DbError> {
    let (direct_user_low_id, direct_user_high_id, game_id, tournament_id) = match target {
        DbChatTarget::Direct {
            low_id, high_id, ..
        } => (Some(*low_id), Some(*high_id), None, None),
        DbChatTarget::Game { game, .. } => (None, None, Some(game.id), None),
        DbChatTarget::Tournament { id, .. } => (None, None, None, Some(*id)),
        DbChatTarget::Global { .. } => (None, None, None, None),
    };
    let kind = target.kind();

    if let Some(id) = diesel::insert_into(chat_channels::table)
        .values((
            chat_channels::kind.eq(kind.as_str()),
            chat_channels::direct_user_low_id.eq(direct_user_low_id),
            chat_channels::direct_user_high_id.eq(direct_user_high_id),
            chat_channels::game_id.eq(game_id),
            chat_channels::tournament_id.eq(tournament_id),
        ))
        .on_conflict_do_nothing()
        .returning(chat_channels::id)
        .get_result(conn)
        .await
        .optional()
        .map_err(DbError::from)?
    {
        return Ok(id);
    }

    lookup_channel_id(conn, target)
        .await?
        .ok_or_else(|| DbError::NotFound {
            reason: "Chat channel disappeared after a conflicting insert".to_string(),
        })
}
