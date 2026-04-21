//! Block list and tournament chat mutes. Single source of truth for "is_blocked" and "is_muted".

use crate::{
    db_error::DbError,
    models::{NewUserBlock, NewUserTournamentChatMute, Tournament},
    schema::{user_blocks, user_tournament_chat_mutes, users},
    DbConn,
};
use diesel::{dsl::exists, prelude::*, select};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

async fn ensure_user_exists(conn: &mut DbConn<'_>, user_id: Uuid) -> Result<(), DbError> {
    let exists = select(exists(users::table.filter(users::id.eq(user_id))))
        .get_result(conn)
        .await
        .map_err(DbError::from)?;

    if exists {
        Ok(())
    } else {
        Err(DbError::NotFound {
            reason: "User not found.".to_string(),
        })
    }
}

async fn get_tournament_id_by_nanoid(
    conn: &mut DbConn<'_>,
    tournament_nanoid: &str,
) -> Result<Uuid, DbError> {
    Tournament::from_nanoid(tournament_nanoid, conn)
        .await
        .map(|tournament| tournament.id)
}

/// Add a block: blocker_id will not receive DMs from blocked_id. Idempotent.
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

    ensure_user_exists(conn, blocked_id).await?;

    diesel::insert_into(user_blocks::table)
        .values(NewUserBlock {
            blocker_id,
            blocked_id,
        })
        .on_conflict((user_blocks::blocker_id, user_blocks::blocked_id))
        .do_nothing()
        .execute(conn)
        .await
        .map_err(DbError::from)?;
    Ok(())
}

/// Remove a block.
pub async fn unblock_user(
    conn: &mut DbConn<'_>,
    blocker_id: Uuid,
    blocked_id: Uuid,
) -> Result<(), DbError> {
    diesel::delete(
        user_blocks::table.filter(
            user_blocks::blocker_id
                .eq(blocker_id)
                .and(user_blocks::blocked_id.eq(blocked_id)),
        ),
    )
    .execute(conn)
    .await
    .map_err(DbError::from)?;
    Ok(())
}

/// True if blocker has blocked blocked_id (so blocker should not receive messages from blocked_id).
pub async fn is_blocked(
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

/// All user IDs that this user has blocked. Used by chat surfaces to hide messages by default.
pub async fn get_blocked_user_ids(
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

/// Mute tournament lobby chat for this user. Idempotent. tournament_nanoid is the tournament's nanoid.
pub async fn mute_tournament_chat(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournament_nanoid: &str,
) -> Result<(), DbError> {
    let tournament_id = get_tournament_id_by_nanoid(conn, tournament_nanoid).await?;
    diesel::insert_into(user_tournament_chat_mutes::table)
        .values(NewUserTournamentChatMute {
            user_id,
            tournament_id,
        })
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

/// Unmute tournament lobby chat.
pub async fn unmute_tournament_chat(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournament_nanoid: &str,
) -> Result<(), DbError> {
    let tournament_id = get_tournament_id_by_nanoid(conn, tournament_nanoid).await?;
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

/// User IDs who have muted this tournament (by tournament UUID). Used to exclude from live delivery.
pub async fn get_user_ids_who_muted_tournament(
    conn: &mut DbConn<'_>,
    tournament_id: Uuid,
) -> Result<std::collections::HashSet<Uuid>, DbError> {
    let ids: Vec<Uuid> = user_tournament_chat_mutes::table
        .filter(user_tournament_chat_mutes::tournament_id.eq(tournament_id))
        .select(user_tournament_chat_mutes::user_id)
        .load(conn)
        .await
        .map_err(DbError::from)?;
    Ok(ids.into_iter().collect())
}
