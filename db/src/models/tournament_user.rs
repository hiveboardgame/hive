use crate::{
    db_error::DbError,
    models::{tournament::Tournament, user::User},
    schema::tournaments_users::{self, dsl::tournaments_users as tournament_user_table},
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{prelude::*, Identifiable, Insertable, Queryable, Selectable};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[derive(Insertable, Identifiable, Selectable, Queryable, Associations, Debug, Clone)]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(belongs_to(Tournament))]
#[diesel(table_name = tournaments_users)]
#[diesel(primary_key(tournament_id, user_id))]
pub struct TournamentUser {
    pub tournament_id: Uuid,
    pub user_id: Uuid,
    /// The player's pairing number, `0..player_count`, assigned once when the
    /// tournament starts. It is also the index the pairing engine knows the
    /// player by, so it must never change afterwards.
    pub seed: Option<i32>,
    /// Rating snapshot taken at start, for the tournament's game speed. Kept so
    /// the seeding stays reproducible after ratings move on.
    pub rating: Option<f64>,
    /// When the player entered. Only an arena admits players after the start,
    /// and its pairing depends on how long each has been waiting, so the
    /// instant matters there rather than just the order.
    pub joined_at: Option<DateTime<Utc>>,
}

impl TournamentUser {
    pub fn new(tournament_id: Uuid, user_id: Uuid) -> Self {
        Self {
            tournament_id,
            user_id,
            seed: None,
            rating: None,
            joined_at: Some(Utc::now()),
        }
    }

    /// Joins an arena that is already running: the seed has to be handed out
    /// now, in arrival order, because `ArenaTournament::join` indexes its
    /// player table directly and rejects a non-sequential id.
    pub fn new_arena_entrant(
        tournament_id: Uuid,
        user_id: Uuid,
        seed: i32,
        rating: Option<f64>,
    ) -> Self {
        Self {
            tournament_id,
            user_id,
            seed: Some(seed),
            rating,
            joined_at: Some(Utc::now()),
        }
    }

    pub async fn insert(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        self.insert_into(tournament_user_table)
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn delete(
        tournament_id: Uuid,
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        diesel::delete(tournaments_users::table.find((tournament_id, user_id)))
            .execute(conn)
            .await?;
        Ok(())
    }
}
