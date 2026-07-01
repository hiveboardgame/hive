use super::game::Game;
use crate::{db_error::DbError, schema::game_hashes, DbConn};
use chrono::{DateTime, Utc};
use diesel::{
    prelude::*,
    sql_types::{Array, BigInt, Bool, Double, Integer, Nullable, Text, Uuid as SqlUuid},
};
use diesel_async::RunQueryDsl;
use hudsoni::{Color, GameResult, GameStatus};
use shared_types::{ExplorerFilters, ExplorerMove};
use uuid::Uuid;

/// The three decisive `result` strings stored in `game_hashes.result`, derived from the engine
/// so they always match what the finish path writes (`Finished(1-0)` / `(0-1)` / `(½-½)`).
fn result_strings() -> (String, String, String) {
    (
        GameStatus::Finished(GameResult::Winner(Color::White)).to_string(),
        GameStatus::Finished(GameResult::Winner(Color::Black)).to_string(),
        GameStatus::Finished(GameResult::Draw).to_string(),
    )
}

/// Raw-SQL row for the next-moves / single-position aggregates.
#[derive(QueryableByName, Debug)]
struct AggRow {
    #[diesel(sql_type = BigInt)]
    next_hash: i64,
    #[diesel(sql_type = Text)]
    piece: String,
    #[diesel(sql_type = Text)]
    position: String,
    #[diesel(sql_type = BigInt)]
    total: i64,
    #[diesel(sql_type = BigInt)]
    white_wins: i64,
    #[diesel(sql_type = BigInt)]
    black_wins: i64,
    #[diesel(sql_type = BigInt)]
    draws: i64,
    #[diesel(sql_type = Nullable<Double>)]
    avg_rating: Option<f64>,
}

impl From<AggRow> for ExplorerMove {
    fn from(r: AggRow) -> Self {
        ExplorerMove {
            next_hash: r.next_hash,
            piece: r.piece,
            position: r.position,
            total: r.total,
            white_wins: r.white_wins,
            black_wins: r.black_wins,
            draws: r.draws,
            avg_rating: r.avg_rating,
        }
    }
}

#[derive(QueryableByName, Debug)]
struct GameIdRow {
    #[diesel(sql_type = SqlUuid)]
    game_id: Uuid,
}

#[derive(Queryable, Insertable, Debug)]
#[diesel(table_name = game_hashes)]
pub struct GameHash {
    pub hash: i64,
    pub game_id: Uuid,
    pub turn: i32,
    pub rating: Option<f64>,
    pub result: String,
    pub speed: String,
    pub game_type: String,
    pub rated: bool,
    pub played_at: DateTime<Utc>,
    /// The move (piece + position notation) that produced this position. Display label only —
    /// suggested moves are keyed by the next turn's `hash`, which is canonical.
    pub move_piece: String,
    pub move_position: String,
    /// Total number of turns in the game, denormalized for filtering out ultra-short games.
    pub game_length: i32,
}

pub struct GameFinishContext {
    pub white_rating: Option<f64>,
    pub black_rating: Option<f64>,
    pub result: String,
    pub speed: String,
    pub game_type: String,
    pub rated: bool,
    pub played_at: DateTime<Utc>,
}

impl GameFinishContext {
    pub fn from_finished_game(game: &Game) -> Self {
        Self {
            white_rating: game
                .white_rating
                .zip(game.white_rating_change)
                .map(|(r, c)| r + c),
            black_rating: game
                .black_rating
                .zip(game.black_rating_change)
                .map(|(r, c)| r + c),
            result: game.game_status.clone(),
            speed: game.speed.clone(),
            game_type: game.game_type.clone(),
            rated: game.rated,
            played_at: game.updated_at,
        }
    }
}

impl GameHash {
    /// Build one row per position. `moves[turn]` is the `(piece, position)` notation that
    /// produced the position at that turn (parallel to `hashes`); `game_length` is the total
    /// number of turns, denormalized onto every row.
    pub fn from_engine_hashes(
        game_id: Uuid,
        hashes: &[u64],
        moves: &[(String, String)],
        ctx: &GameFinishContext,
    ) -> Vec<Self> {
        let game_length = hashes.len() as i32;
        hashes
            .iter()
            .enumerate()
            .map(|(turn, &h)| {
                let (piece, position) = moves.get(turn).cloned().unwrap_or_default();
                Self {
                    hash: h as i64,
                    game_id,
                    turn: turn as i32,
                    rating: if turn % 2 == 0 {
                        ctx.white_rating
                    } else {
                        ctx.black_rating
                    },
                    result: ctx.result.clone(),
                    speed: ctx.speed.clone(),
                    game_type: ctx.game_type.clone(),
                    rated: ctx.rated,
                    played_at: ctx.played_at,
                    move_piece: piece,
                    move_position: position,
                    game_length,
                }
            })
            .collect()
    }

    pub async fn insert_batch(entries: &[GameHash], conn: &mut DbConn<'_>) -> Result<(), DbError> {
        if entries.is_empty() {
            return Ok(());
        }
        diesel::insert_into(game_hashes::table)
            .values(entries)
            .on_conflict_do_nothing()
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn insert_for_game(
        game_id: Uuid,
        hashes: &[u64],
        moves: &[(String, String)],
        ctx: &GameFinishContext,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        let entries = Self::from_engine_hashes(game_id, hashes, moves, ctx);
        Self::insert_batch(&entries, conn).await
    }

    pub async fn find_by_hash(hash: u64, conn: &mut DbConn<'_>) -> Result<Vec<GameHash>, DbError> {
        Ok(game_hashes::table
            .filter(game_hashes::hash.eq(hash as i64))
            .load(conn)
            .await?)
    }

    pub async fn best(
        hash: u64,
        limit: Option<i64>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<GameHash>, DbError> {
        Ok(game_hashes::table
            .filter(game_hashes::hash.eq(hash as i64))
            .filter(game_hashes::rating.is_not_null())
            .order(game_hashes::rating.desc())
            .limit(limit.unwrap_or(10))
            .load(conn)
            .await?)
    }

    /// Aggregate the best next moves from the position `hash`, across all games that passed
    /// through it. Self-joins each matching row to the next turn's row; suggestions are keyed
    /// by the resulting position hash (canonical, so rotations/transpositions merge). Counts
    /// distinct games (not row occurrences, which a repetition could inflate). Ordered by
    /// popularity, capped at `limit` (default 8).
    pub async fn next_moves(
        hash: i64,
        filters: &ExplorerFilters,
        limit: Option<i64>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<ExplorerMove>, DbError> {
        let (white_res, black_res, draw_res) = result_strings();
        let rows: Vec<AggRow> = diesel::sql_query(
            r#"
            SELECT
                nxt.hash AS next_hash,
                COALESCE((array_agg(nxt.move_piece))[1], '') AS piece,
                COALESCE((array_agg(nxt.move_position))[1], '') AS position,
                COUNT(DISTINCT nxt.game_id) AS total,
                COUNT(DISTINCT nxt.game_id) FILTER (WHERE nxt.result = $6) AS white_wins,
                COUNT(DISTINCT nxt.game_id) FILTER (WHERE nxt.result = $7) AS black_wins,
                COUNT(DISTINCT nxt.game_id) FILTER (WHERE nxt.result = $8) AS draws,
                AVG(nxt.rating) AS avg_rating
            FROM game_hashes cur
            JOIN game_hashes nxt
                ON nxt.game_id = cur.game_id AND nxt.turn = cur.turn + 1
            WHERE cur.hash = $1
                AND nxt.game_type = $2
                AND (cardinality($3) = 0 OR nxt.speed = ANY($3))
                AND ($4 IS NULL OR nxt.rated = $4)
                AND ($5 IS NULL OR nxt.game_length >= $5)
            GROUP BY nxt.hash
            ORDER BY total DESC
            LIMIT $9
            "#,
        )
        .bind::<BigInt, _>(hash)
        .bind::<Text, _>(filters.game_type.to_string())
        .bind::<Array<Text>, _>(
            filters
                .speeds
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        )
        .bind::<Nullable<Bool>, _>(filters.rated)
        .bind::<Nullable<Integer>, _>(filters.min_game_length)
        .bind::<Text, _>(white_res)
        .bind::<Text, _>(black_res)
        .bind::<Text, _>(draw_res)
        .bind::<BigInt, _>(limit.unwrap_or(8))
        .load(conn)
        .await?;
        Ok(rows.into_iter().map(ExplorerMove::from).collect())
    }

    /// Aggregate stats for a single position `hash` (the explorer header, and each opening
    /// root). Returns a zeroed `ExplorerMove` (with the queried hash) when no games match.
    /// `piece`/`position` are left empty; callers that need a label (opening roots) supply it.
    pub async fn aggregate_one(
        hash: i64,
        filters: &ExplorerFilters,
        conn: &mut DbConn<'_>,
    ) -> Result<ExplorerMove, DbError> {
        let (white_res, black_res, draw_res) = result_strings();
        let row: AggRow = diesel::sql_query(
            r#"
            SELECT
                $1 AS next_hash,
                '' AS piece,
                '' AS position,
                COUNT(DISTINCT game_id) AS total,
                COUNT(DISTINCT game_id) FILTER (WHERE result = $6) AS white_wins,
                COUNT(DISTINCT game_id) FILTER (WHERE result = $7) AS black_wins,
                COUNT(DISTINCT game_id) FILTER (WHERE result = $8) AS draws,
                AVG(rating) AS avg_rating
            FROM game_hashes
            WHERE hash = $1
                AND game_type = $2
                AND (cardinality($3) = 0 OR speed = ANY($3))
                AND ($4 IS NULL OR rated = $4)
                AND ($5 IS NULL OR game_length >= $5)
            "#,
        )
        .bind::<BigInt, _>(hash)
        .bind::<Text, _>(filters.game_type.to_string())
        .bind::<Array<Text>, _>(
            filters
                .speeds
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        )
        .bind::<Nullable<Bool>, _>(filters.rated)
        .bind::<Nullable<Integer>, _>(filters.min_game_length)
        .bind::<Text, _>(white_res)
        .bind::<Text, _>(black_res)
        .bind::<Text, _>(draw_res)
        .get_result(conn)
        .await?;
        Ok(row.into())
    }

    /// Game ids of the most recently played games that passed through `hash`, honoring the
    /// explorer filters. Ordered by most recent play date desc. Used for the "recent games" list.
    pub async fn recent_game_ids(
        hash: i64,
        filters: &ExplorerFilters,
        limit: Option<i64>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Uuid>, DbError> {
        let rows: Vec<GameIdRow> = diesel::sql_query(
            r#"
            SELECT game_id
            FROM game_hashes
            WHERE hash = $1
                AND game_type = $2
                AND (cardinality($3) = 0 OR speed = ANY($3))
                AND ($4 IS NULL OR rated = $4)
                AND ($5 IS NULL OR game_length >= $5)
            GROUP BY game_id
            ORDER BY MAX(played_at) DESC
            LIMIT $6
            "#,
        )
        .bind::<BigInt, _>(hash)
        .bind::<Text, _>(filters.game_type.to_string())
        .bind::<Array<Text>, _>(
            filters
                .speeds
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        )
        .bind::<Nullable<Bool>, _>(filters.rated)
        .bind::<Nullable<Integer>, _>(filters.min_game_length)
        .bind::<BigInt, _>(limit.unwrap_or(4))
        .load(conn)
        .await?;
        Ok(rows.into_iter().map(|r| r.game_id).collect())
    }

    /// Game ids of the strongest games (highest rating at the position) that passed through
    /// `hash`, honoring the explorer filters. Ordered by rating desc. Used for the "top games"
    /// list — callers turn these into full game responses.
    pub async fn top_game_ids(
        hash: i64,
        filters: &ExplorerFilters,
        limit: Option<i64>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Uuid>, DbError> {
        let rows: Vec<GameIdRow> = diesel::sql_query(
            r#"
            SELECT game_id
            FROM game_hashes
            WHERE hash = $1
                AND game_type = $2
                AND rating IS NOT NULL
                AND (cardinality($3) = 0 OR speed = ANY($3))
                AND ($4 IS NULL OR rated = $4)
                AND ($5 IS NULL OR game_length >= $5)
            GROUP BY game_id
            ORDER BY MAX(rating) DESC
            LIMIT $6
            "#,
        )
        .bind::<BigInt, _>(hash)
        .bind::<Text, _>(filters.game_type.to_string())
        .bind::<Array<Text>, _>(
            filters
                .speeds
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        )
        .bind::<Nullable<Bool>, _>(filters.rated)
        .bind::<Nullable<Integer>, _>(filters.min_game_length)
        .bind::<BigInt, _>(limit.unwrap_or(8))
        .load(conn)
        .await?;
        Ok(rows.into_iter().map(|r| r.game_id).collect())
    }
}
