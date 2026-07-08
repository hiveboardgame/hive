use crate::{
    db_error::DbError,
    schema::{tournaments, user_tournament_chat_mutes},
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{dsl::exists, prelude::*, select, Insertable, Queryable, Selectable};
use diesel_async::RunQueryDsl;
use shared_types::TournamentId;
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = user_tournament_chat_mutes)]
#[diesel(primary_key(user_id, tournament_id))]
pub struct UserTournamentChatMute {
    pub user_id: Uuid,
    pub tournament_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = user_tournament_chat_mutes)]
pub struct NewUserTournamentChatMute {
    pub user_id: Uuid,
    pub tournament_id: Uuid,
}

impl UserTournamentChatMute {
    pub async fn is_muted(
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

    pub async fn mute(
        conn: &mut DbConn<'_>,
        user_id: Uuid,
        tournament_id: Uuid,
    ) -> Result<(), DbError> {
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

    pub async fn unmute(
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
}
