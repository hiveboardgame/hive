use crate::{db_error::DbError, models::Game, schema::games, DbConn};
use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, QueryDsl, Queryable, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;
use shared_types::Conclusion;
use uuid::Uuid;

/// Tournament facts exclude the move/control histories and clock payloads.
#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = games)]
pub(crate) struct TournamentGame {
    pub(crate) id: Uuid,
    pub(crate) nanoid: String,
    pub(crate) white_id: Uuid,
    pub(crate) black_id: Uuid,
    pub(crate) turn: i32,
    pub(crate) finished: bool,
    pub(crate) game_status: String,
    pub(crate) game_start: String,
    pub(crate) speed: String,
    pub(crate) white_rating: Option<f64>,
    pub(crate) black_rating: Option<f64>,
    pub(crate) white_berserked: bool,
    pub(crate) black_berserked: bool,
    pub(crate) finished_at: Option<DateTime<Utc>>,
    pub(crate) conclusion: String,
    pub(crate) tournament_game_result: String,
    pub(crate) tournament_slot_id: Option<Uuid>,
    pub(crate) arena_ordinal: Option<i64>,
}

impl TournamentGame {
    pub(crate) async fn load(
        tournament_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        Ok(games::table
            .filter(games::tournament_id.eq(tournament_id))
            .select(Self::as_select())
            .load(conn)
            .await?)
    }

    pub(crate) fn is_arena_no_start(&self) -> bool {
        self.finished && self.turn < 2 && self.conclusion == Conclusion::Timeout.to_string()
    }
}

impl From<&Game> for TournamentGame {
    fn from(game: &Game) -> Self {
        Self {
            id: game.id,
            nanoid: game.nanoid.clone(),
            white_id: game.white_id,
            black_id: game.black_id,
            turn: game.turn,
            finished: game.finished,
            game_status: game.game_status.clone(),
            game_start: game.game_start.clone(),
            speed: game.speed.clone(),
            white_rating: game.white_rating,
            black_rating: game.black_rating,
            white_berserked: game.white_berserked,
            black_berserked: game.black_berserked,
            finished_at: game.finished_at,
            conclusion: game.conclusion.clone(),
            tournament_game_result: game.tournament_game_result.clone(),
            tournament_slot_id: game.tournament_slot_id,
            arena_ordinal: game.arena_ordinal,
        }
    }
}
