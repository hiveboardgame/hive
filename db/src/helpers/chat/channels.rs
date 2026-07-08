use super::target::DbChatTarget;
use crate::{
    db_error::DbError,
    models::{ChatChannelKind, NewChatChannel},
    schema::chat_channels,
    DbConn,
};
use chrono::Utc;
use diesel::{prelude::*, upsert::excluded};
use diesel_async::RunQueryDsl;

pub(super) async fn ensure_chat_channel(
    conn: &mut DbConn<'_>,
    target: &DbChatTarget,
) -> Result<i64, DbError> {
    let (direct_user_low_id, direct_user_high_id) = match &target.direct {
        Some(direct) => (Some(direct.low_id), Some(direct.high_id)),
        None => (None, None),
    };
    let game_id = match target.kind {
        ChatChannelKind::Game(_) => target.game.as_ref().map(|game| game.id),
        _ => None,
    };
    let tournament_id = match target.kind {
        ChatChannelKind::TournamentLobby => {
            target.tournament.as_ref().map(|tournament| tournament.id)
        }
        _ => None,
    };

    diesel::insert_into(chat_channels::table)
        .values(NewChatChannel {
            kind: target.kind.as_str(),
            lookup_key: &target.lookup_key,
            direct_user_low_id,
            direct_user_high_id,
            game_id,
            tournament_id,
            created_at: Utc::now(),
        })
        .on_conflict((chat_channels::kind, chat_channels::lookup_key))
        .do_update()
        .set((
            chat_channels::direct_user_low_id.eq(excluded(chat_channels::direct_user_low_id)),
            chat_channels::direct_user_high_id.eq(excluded(chat_channels::direct_user_high_id)),
            chat_channels::game_id.eq(excluded(chat_channels::game_id)),
            chat_channels::tournament_id.eq(excluded(chat_channels::tournament_id)),
        ))
        .returning(chat_channels::id)
        .get_result(conn)
        .await
        .map_err(DbError::from)
}
