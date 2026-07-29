use crate::{
    db_error::DbError,
    schema::tournament_byes::{
        self,
        round as round_column,
        tournament_id as tournament_id_column,
        user_id as user_id_column,
    },
    DbConn,
};
use diesel::{prelude::*, Insertable, Queryable, Selectable};
use diesel_async::RunQueryDsl;
use shared_types::ByeKind;
use std::str::FromStr;
use uuid::Uuid;

/// A player who sat out a round. Swiss byes cannot be recovered from the games
/// table — a bye produces no game — and they cannot be inferred from "who has
/// no game this round" either, because a withdrawal looks identical and is not
/// timestamped anywhere. The pairing engine needs them back verbatim to replay
/// a round, so they are recorded when the round is paired.
#[derive(Insertable, Selectable, Queryable, Debug, Clone)]
#[diesel(table_name = tournament_byes)]
#[diesel(primary_key(tournament_id, round, user_id))]
pub struct TournamentBye {
    pub tournament_id: Uuid,
    pub round: i32,
    pub user_id: Uuid,
    pub kind: String,
}

impl TournamentBye {
    pub fn new(tournament_id: Uuid, round: i32, user_id: Uuid, kind: ByeKind) -> Self {
        Self {
            tournament_id,
            round,
            user_id,
            kind: kind.to_string(),
        }
    }

    pub fn kind(&self) -> Result<ByeKind, DbError> {
        ByeKind::from_str(&self.kind).map_err(|error| DbError::InvalidAction {
            info: error.to_string(),
        })
    }

    pub async fn insert_many(byes: &[Self], conn: &mut DbConn<'_>) -> Result<(), DbError> {
        if byes.is_empty() {
            return Ok(());
        }
        diesel::insert_into(tournament_byes::table)
            .values(byes)
            .on_conflict_do_nothing()
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn for_round(
        tournament: Uuid,
        round: i32,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        Ok(tournament_byes::table
            .filter(tournament_id_column.eq(tournament))
            .filter(round_column.eq(round))
            .order(user_id_column.asc())
            .select(Self::as_select())
            .get_results(conn)
            .await?)
    }

    pub async fn for_tournament(
        tournament: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        Ok(tournament_byes::table
            .filter(tournament_id_column.eq(tournament))
            .order((round_column.asc(), user_id_column.asc()))
            .select(Self::as_select())
            .get_results(conn)
            .await?)
    }
}
