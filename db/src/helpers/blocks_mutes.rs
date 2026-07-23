use super::chat::authorize_tournament_chat_access;
use crate::{
    db_error::DbError,
    models::User,
    schema::{tournaments, user_blocks, user_tournament_chat_mutes},
    DbConn,
};
use diesel::{dsl::exists, prelude::*, select};
use diesel_async::RunQueryDsl;
use shared_types::TournamentId;
use uuid::Uuid;

pub async fn block_user(
    conn: &mut DbConn<'_>,
    blocker_id: Uuid,
    blocked_id: Uuid,
) -> Result<(), DbError> {
    if blocker_id == blocked_id {
        return Err(DbError::InvalidInput {
            info: "Cannot block yourself".to_string(),
            error: "blocker_id == blocked_id".to_string(),
        });
    }
    User::find_active_by_uuid(&blocked_id, conn).await?;

    diesel::insert_into(user_blocks::table)
        .values((
            user_blocks::blocker_id.eq(blocker_id),
            user_blocks::blocked_id.eq(blocked_id),
        ))
        .on_conflict((user_blocks::blocker_id, user_blocks::blocked_id))
        .do_nothing()
        .execute(conn)
        .await
        .map_err(DbError::from)?;
    Ok(())
}

pub async fn unblock_user(
    conn: &mut DbConn<'_>,
    blocker_id: Uuid,
    blocked_id: Uuid,
) -> Result<(), DbError> {
    diesel::delete(
        user_blocks::table
            .filter(user_blocks::blocker_id.eq(blocker_id))
            .filter(user_blocks::blocked_id.eq(blocked_id)),
    )
    .execute(conn)
    .await
    .map_err(DbError::from)?;
    Ok(())
}

pub async fn is_user_blocked(
    conn: &mut DbConn<'_>,
    blocker_id: Uuid,
    blocked_id: Uuid,
) -> Result<bool, DbError> {
    select(exists(
        user_blocks::table
            .filter(user_blocks::blocker_id.eq(blocker_id))
            .filter(user_blocks::blocked_id.eq(blocked_id)),
    ))
    .get_result(conn)
    .await
    .map_err(DbError::from)
}

pub async fn blocked_user_ids(
    conn: &mut DbConn<'_>,
    blocker_id: Uuid,
) -> Result<Vec<Uuid>, DbError> {
    user_blocks::table
        .filter(user_blocks::blocker_id.eq(blocker_id))
        .select(user_blocks::blocked_id)
        .load(conn)
        .await
        .map_err(DbError::from)
}

/// Every user who has blocked `blocked_id`, in one query — for filtering a
/// recipient list (e.g. chat-notification fan-out) instead of awaiting
/// `is_user_blocked` once per candidate.
pub async fn blockers_of_user(
    conn: &mut DbConn<'_>,
    blocked_id: Uuid,
) -> Result<Vec<Uuid>, DbError> {
    user_blocks::table
        .filter(user_blocks::blocked_id.eq(blocked_id))
        .select(user_blocks::blocker_id)
        .load(conn)
        .await
        .map_err(DbError::from)
}

pub async fn is_tournament_chat_muted(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournament_id: Uuid,
) -> Result<bool, DbError> {
    select(exists(
        user_tournament_chat_mutes::table
            .filter(user_tournament_chat_mutes::user_id.eq(user_id))
            .filter(user_tournament_chat_mutes::tournament_id.eq(tournament_id)),
    ))
    .get_result(conn)
    .await
    .map_err(DbError::from)
}

pub async fn muted_tournament_ids_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<TournamentId>, DbError> {
    user_tournament_chat_mutes::table
        .inner_join(tournaments::table)
        .filter(user_tournament_chat_mutes::user_id.eq(user_id))
        .select(tournaments::nanoid)
        .load::<String>(conn)
        .await
        .map(|rows| rows.into_iter().map(TournamentId).collect())
        .map_err(DbError::from)
}

/// Every user who has muted `tournament_id`'s chat, in one query — for
/// filtering a recipient list instead of awaiting `is_tournament_chat_muted`
/// once per candidate.
pub async fn muted_tournament_chat_user_ids(
    conn: &mut DbConn<'_>,
    tournament_id: Uuid,
) -> Result<Vec<Uuid>, DbError> {
    user_tournament_chat_mutes::table
        .filter(user_tournament_chat_mutes::tournament_id.eq(tournament_id))
        .select(user_tournament_chat_mutes::user_id)
        .load(conn)
        .await
        .map_err(DbError::from)
}

async fn mute_tournament_chat(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournament_id: Uuid,
) -> Result<(), DbError> {
    diesel::insert_into(user_tournament_chat_mutes::table)
        .values((
            user_tournament_chat_mutes::user_id.eq(user_id),
            user_tournament_chat_mutes::tournament_id.eq(tournament_id),
        ))
        .on_conflict((
            user_tournament_chat_mutes::user_id,
            user_tournament_chat_mutes::tournament_id,
        ))
        .do_nothing()
        .execute(conn)
        .await
        .map_err(DbError::from)?;
    Ok(())
}

async fn unmute_tournament_chat(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournament_id: Uuid,
) -> Result<(), DbError> {
    diesel::delete(
        user_tournament_chat_mutes::table
            .filter(user_tournament_chat_mutes::user_id.eq(user_id))
            .filter(user_tournament_chat_mutes::tournament_id.eq(tournament_id)),
    )
    .execute(conn)
    .await
    .map_err(DbError::from)?;
    Ok(())
}

pub async fn set_tournament_chat_muted(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournament_nanoid: &str,
    muted: bool,
) -> Result<(), DbError> {
    let tournament = authorize_tournament_chat_access(conn, user_id, tournament_nanoid).await?;
    if muted {
        mute_tournament_chat(conn, user_id, tournament.id).await
    } else {
        unmute_tournament_chat(conn, user_id, tournament.id).await
    }
}
