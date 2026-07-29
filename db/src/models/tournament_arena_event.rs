use crate::{
    db_error::DbError,
    schema::tournament_arena_events::{
        self,
        at as at_column,
        id as id_column,
        tournament_id as tournament_id_column,
    },
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{prelude::*, Insertable, Queryable, Selectable};
use diesel_async::RunQueryDsl;
use shared_types::ArenaEventKind;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = tournament_arena_events)]
pub struct NewTournamentArenaEvent {
    pub tournament_id: Uuid,
    pub user_id: Uuid,
    pub kind: String,
    pub at: DateTime<Utc>,
}

#[derive(Selectable, Queryable, Debug, Clone)]
#[diesel(table_name = tournament_arena_events)]
pub struct TournamentArenaEvent {
    pub id: i32,
    pub tournament_id: Uuid,
    pub user_id: Uuid,
    pub kind: String,
    pub at: DateTime<Utc>,
}

impl TournamentArenaEvent {
    pub fn kind(&self) -> Result<ArenaEventKind, DbError> {
        ArenaEventKind::from_str(&self.kind).map_err(|_| DbError::InvalidAction {
            info: format!("{} is not a valid arena event", self.kind),
        })
    }

    pub(crate) async fn record(
        tournament: Uuid,
        user: Uuid,
        kind: ArenaEventKind,
        at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        Ok(diesel::insert_into(tournament_arena_events::table)
            .values(NewTournamentArenaEvent {
                tournament_id: tournament,
                user_id: user,
                kind: kind.to_string(),
                at,
            })
            .get_result(conn)
            .await?)
    }

    /// Removes an event so replay never sees it. Used for reinstating a
    /// withdrawn arena player: the engine has no un-withdraw, so the only way
    /// back in is for the withdrawal not to have happened as far as the replay
    /// is concerned.
    pub(crate) async fn forget(
        tournament: Uuid,
        user: Uuid,
        kind: ArenaEventKind,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        Ok(diesel::delete(
            tournament_arena_events::table
                .filter(tournament_id_column.eq(tournament))
                .filter(tournament_arena_events::user_id.eq(user))
                .filter(tournament_arena_events::kind.eq(kind.to_string())),
        )
        .execute(conn)
        .await?)
    }

    /// In replay order. `id` breaks ties so two events in the same millisecond
    /// still replay the way they were written.
    pub async fn for_tournament(
        tournament: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        Ok(tournament_arena_events::table
            .filter(tournament_id_column.eq(tournament))
            .order((at_column.asc(), id_column.asc()))
            .select(Self::as_select())
            .get_results(conn)
            .await?)
    }
}
