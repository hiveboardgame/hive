use crate::{
    db_error::DbError,
    schema::{
        puzzle_attempts::{
            self,
            created_at as attempt_created_at,
            hinted,
            puzzle_id as attempt_puzzle_id,
            rated,
            rating_after,
            rating_before,
            solved,
            time_ms,
            user_id as attempt_user_id,
        },
        puzzle_stars::{
            self,
            puzzle_id as star_puzzle_id,
            stars as star_value,
            updated_at as star_updated_at,
            user_id as star_user_id,
        },
        puzzles::{
            self,
            collection as puzzle_collection,
            deviation as puzzle_deviation,
            game_type as puzzle_game_type,
            hidden,
            nanoid as puzzle_nanoid,
            plays,
            rating as puzzle_rating,
            solver_moves as puzzle_solver_moves,
            solves,
            source_game,
            stars_count,
            stars_sum,
            themes as puzzle_themes,
            volatility as puzzle_volatility,
        },
        ratings::{
            self,
            deviation as user_deviation,
            lost as user_lost,
            played as user_played,
            rating as user_rating,
            speed as rating_speed,
            updated_at as rating_updated_at,
            user_uid as rating_user,
            volatility as user_volatility,
            won as user_won,
        },
    },
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{dsl::exists, prelude::*, select};
use diesel_async::{AsyncConnection, RunQueryDsl};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use shared_types::{
    GameSpeed,
    PuzzleHistoryFilter,
    PuzzleImport,
    PuzzleSelection,
    AUTO_HIDE_MAX_STARS,
    AUTO_HIDE_MIN_VOTES,
    PUZZLE_START_DEVIATION,
    PUZZLE_START_RATING,
    PUZZLE_START_VOLATILITY,
};
use skillratings::{
    glicko2::{glicko2, Glicko2Config, Glicko2Rating},
    Outcomes,
};
use uuid::Uuid;

#[derive(QueryableByName, Debug, Clone, PartialEq, Eq)]
pub struct PuzzleSetCount {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub key: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub total: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub solved_count: i64,
}

/// Prior for the popularity ranking: a puzzle starts as if this many players gave it this many stars.
const STAR_PRIOR_VOTES: f64 = 5.0;
const STAR_PRIOR_MEAN: f64 = 3.0;
const HISTORY_PAGE: i64 = 50;

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = puzzles)]
#[diesel(primary_key(id))]
pub struct Puzzle {
    pub id: Uuid,
    pub nanoid: String,
    pub kind: String,
    pub game_type: String,
    pub history: Option<String>,
    pub position: Option<String>,
    pub solution: Vec<Option<String>>,
    pub variations: serde_json::Value,
    pub solver_moves: i32,
    pub themes: Vec<Option<String>>,
    pub authors: Vec<Option<String>>,
    pub source: String,
    pub source_game: Option<String>,
    pub collection: Option<String>,
    pub title: Option<String>,
    pub hint: Option<String>,
    pub level: Option<String>,
    pub hash: i64,
    pub rating: f64,
    pub deviation: f64,
    pub volatility: f64,
    pub plays: i32,
    pub solves: i32,
    pub stars_sum: i32,
    pub stars_count: i32,
    pub hidden: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = puzzles)]
struct NewPuzzle {
    nanoid: String,
    kind: String,
    game_type: String,
    history: Option<String>,
    position: Option<String>,
    solution: Vec<Option<String>>,
    variations: serde_json::Value,
    solver_moves: i32,
    themes: Vec<Option<String>>,
    authors: Vec<Option<String>>,
    source: String,
    source_game: Option<String>,
    collection: Option<String>,
    title: Option<String>,
    hint: Option<String>,
    level: Option<String>,
    hash: i64,
    rating: f64,
    deviation: f64,
    volatility: f64,
}

#[derive(Queryable, Selectable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = puzzle_attempts)]
pub struct PuzzleAttempt {
    pub id: i64,
    pub user_id: Uuid,
    pub puzzle_id: Uuid,
    pub solved: bool,
    pub rated: bool,
    pub hinted: bool,
    pub time_ms: Option<i32>,
    pub rating_before: Option<f64>,
    pub rating_after: Option<f64>,
    pub created_at: DateTime<Utc>,
}

/// What an attempt did to both ratings; the `None`s mean it was unrated (a replay).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AttemptOutcome {
    pub rated: bool,
    pub user_before: Option<f64>,
    pub user_after: Option<f64>,
    pub puzzle_before: f64,
    pub puzzle_after: f64,
}

impl Puzzle {
    pub fn lines(&self) -> Vec<Vec<String>> {
        let main: Vec<String> = self.solution.iter().flatten().cloned().collect();
        let variations: Vec<Vec<String>> =
            serde_json::from_value(self.variations.clone()).unwrap_or_default();
        std::iter::once(main).chain(variations).collect()
    }

    pub fn authors(&self) -> Vec<String> {
        self.authors.iter().flatten().cloned().collect()
    }

    /// Early batches tagged the finishing bug (`finishBeetle`); that names the answer, so it is never shown.
    pub fn themes(&self) -> Vec<String> {
        self.themes
            .iter()
            .flatten()
            .filter(|theme| !theme.starts_with("finish"))
            .cloned()
            .collect()
    }

    pub fn average_stars(&self) -> Option<f64> {
        (self.stars_count > 0).then(|| f64::from(self.stars_sum) / f64::from(self.stars_count))
    }

    /// Stores a verified puzzle; `Ok(None)` when its position (same kind) is already stored.
    pub async fn import(
        puzzle: &PuzzleImport,
        hash: i64,
        conn: &mut DbConn<'_>,
    ) -> Result<Option<Puzzle>, DbError> {
        let to_strings = |values: &[String]| {
            values
                .iter()
                .cloned()
                .map(Some)
                .collect::<Vec<Option<String>>>()
        };
        let new = NewPuzzle {
            nanoid: nanoid!(10),
            kind: puzzle.kind.to_string(),
            game_type: puzzle.game_type.clone(),
            history: puzzle.history.clone(),
            position: puzzle.position.clone(),
            solution: to_strings(&puzzle.solution),
            variations: serde_json::to_value(&puzzle.variations).map_err(|e| {
                DbError::InvalidInput {
                    info: "variations".to_string(),
                    error: e.to_string(),
                }
            })?,
            solver_moves: shared_types::solver_moves(&puzzle.solution) as i32,
            themes: puzzle
                .themes
                .iter()
                .filter(|theme| !theme.starts_with("finish"))
                .cloned()
                .map(Some)
                .collect(),
            authors: to_strings(&puzzle.authors),
            source: puzzle.source.clone(),
            source_game: puzzle.source_game.clone(),
            collection: puzzle.collection.clone(),
            title: puzzle.title.clone(),
            hint: puzzle.hint.clone(),
            level: puzzle.level.clone(),
            hash,
            rating: puzzle.rating.unwrap_or(PUZZLE_START_RATING),
            deviation: PUZZLE_START_DEVIATION,
            volatility: PUZZLE_START_VOLATILITY,
        };
        Ok(diesel::insert_into(puzzles::table)
            .values(&new)
            .on_conflict_do_nothing()
            .returning(Puzzle::as_returning())
            .get_result(conn)
            .await
            .optional()?)
    }

    pub async fn find_by_nanoid(nanoid: &str, conn: &mut DbConn<'_>) -> Result<Puzzle, DbError> {
        Ok(puzzles::table
            .filter(puzzle_nanoid.eq(nanoid))
            .select(Puzzle::as_select())
            .first(conn)
            .await?)
    }

    pub async fn ids_for_nanoids(
        nanoids: &[&str],
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Uuid>, DbError> {
        Ok(puzzles::table
            .filter(puzzle_nanoid.eq_any(nanoids))
            .select(puzzles::id)
            .load(conn)
            .await?)
    }

    pub async fn for_source_game(
        nanoid: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Puzzle>, DbError> {
        Ok(puzzles::table
            .filter(source_game.eq(nanoid))
            .filter(hidden.eq(false))
            .select(Puzzle::as_select())
            .load(conn)
            .await?)
    }

    pub async fn visible_count(conn: &mut DbConn<'_>) -> Result<i64, DbError> {
        Ok(puzzles::table
            .filter(hidden.eq(false))
            .count()
            .get_result(conn)
            .await?)
    }

    /// The next puzzle for someone rated `around`: unseen by `user`, within a rating window that
    /// widens until something is found, best-loved first with some shuffling so equals rotate.
    pub async fn next(
        around: f64,
        user: Option<Uuid>,
        selection: &PuzzleSelection,
        exclude: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<Option<Puzzle>, DbError> {
        let around = around + selection.difficulty.rating_offset();
        let popularity = diesel::dsl::sql::<diesel::sql_types::Double>(&format!(
            "((stars_sum + {mean} * {prior}) / (stars_count + {prior})) + random() * 0.75",
            mean = STAR_PRIOR_MEAN,
            prior = STAR_PRIOR_VOTES,
        ));
        // A book is read in order, whatever the rating.
        let in_order = selection.collection.is_some();
        let windows: &[f64] = if in_order {
            &[f64::MAX]
        } else {
            &[75.0, 150.0, 300.0, 600.0, 5000.0]
        };
        for &window in windows {
            let mut query = puzzles::table
                .filter(hidden.eq(false))
                .filter(puzzle_rating.between(around - window, around + window))
                .filter(puzzles::id.ne_all(exclude.to_vec()))
                .select(Puzzle::as_select())
                .into_boxed();
            if let Some(wanted) = selection.game_type.game_type() {
                query = query.filter(puzzle_game_type.eq(wanted));
            }
            if let Some((shortest, longest)) = selection.length.solver_moves() {
                query = query.filter(puzzle_solver_moves.ge(shortest));
                if let Some(longest) = longest {
                    query = query.filter(puzzle_solver_moves.le(longest));
                }
            }
            if let Some(theme) = &selection.theme {
                query = query.filter(puzzle_themes.contains(vec![Some(theme.clone())]));
            }
            if let Some(wanted) = &selection.collection {
                query = query.filter(puzzle_collection.eq(wanted.clone()));
            }
            if let Some(user) = user {
                query = query.filter(diesel::dsl::not(exists(
                    puzzle_attempts::table
                        .filter(attempt_user_id.eq(user))
                        .filter(attempt_puzzle_id.eq(puzzles::id)),
                )));
            }
            let query = if in_order {
                query.order((puzzles::created_at.asc(), puzzles::id.asc()))
            } else {
                query.order(popularity.clone().desc())
            };
            let found = query.first(conn).await.optional()?;
            if found.is_some() {
                return Ok(found);
            }
        }
        Ok(None)
    }

    /// How many visible puzzles each browsable set holds, and how many of them `user` solved on
    /// the first, unassisted try. Keys are themes, `moves:N` (4 = four or more) and `collection:X`.
    pub async fn set_counts(
        user: Option<Uuid>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<PuzzleSetCount>, DbError> {
        Ok(diesel::sql_query(
            "SELECT key, count(*) AS total, count(*) FILTER (WHERE solved) AS solved_count FROM (
                 SELECT unnest(p.themes || ARRAY['moves:' || LEAST(p.solver_moves, 4)::text,
                                                 'collection:' || p.collection]) AS key,
                        COALESCE(a.solved AND NOT a.hinted, false) AS solved
                 FROM puzzles p
                 LEFT JOIN puzzle_attempts a
                   ON a.puzzle_id = p.id AND a.rated AND a.user_id = $1
                 WHERE NOT p.hidden
             ) sets
             WHERE key IS NOT NULL
             GROUP BY key",
        )
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(user)
        .load(conn)
        .await?)
    }

    /// Records one try. The first try of a signed-in player is rated (a hint counts as a miss);
    /// every later try is a replay and leaves both ratings alone.
    pub async fn record_attempt(
        puzzle_id: Uuid,
        user_id: Uuid,
        was_solved: bool,
        used_hint: bool,
        elapsed_ms: Option<i32>,
        conn: &mut DbConn<'_>,
    ) -> Result<AttemptOutcome, DbError> {
        conn.transaction::<_, DbError, _>(async move |tc| {
            let seen: bool = select(exists(
                puzzle_attempts::table
                    .filter(attempt_user_id.eq(user_id))
                    .filter(attempt_puzzle_id.eq(puzzle_id)),
            ))
            .get_result(tc)
            .await?;
            let puzzle: Puzzle = puzzles::table
                .find(puzzle_id)
                .select(Puzzle::as_select())
                .for_update()
                .first(tc)
                .await?;
            let counted_solve = was_solved && !used_hint;

            if seen {
                diesel::insert_into(puzzle_attempts::table)
                    .values((
                        attempt_user_id.eq(user_id),
                        attempt_puzzle_id.eq(puzzle_id),
                        solved.eq(was_solved),
                        rated.eq(false),
                        hinted.eq(used_hint),
                        time_ms.eq(elapsed_ms),
                    ))
                    .execute(tc)
                    .await?;
                return Ok(AttemptOutcome {
                    rated: false,
                    user_before: None,
                    user_after: None,
                    puzzle_before: puzzle.rating,
                    puzzle_after: puzzle.rating,
                });
            }

            let (rating_id, before): (i32, Glicko2Rating) = ratings::table
                .filter(rating_user.eq(user_id))
                .filter(rating_speed.eq(GameSpeed::Puzzle.to_string()))
                .select((ratings::id, (user_rating, user_deviation, user_volatility)))
                .for_update()
                .first::<(i32, (f64, f64, f64))>(tc)
                .await
                .map(|(id, (rating, deviation, volatility))| {
                    (
                        id,
                        Glicko2Rating {
                            rating,
                            deviation,
                            volatility,
                        },
                    )
                })?;
            let puzzle_glicko = Glicko2Rating {
                rating: puzzle.rating,
                deviation: puzzle.deviation,
                volatility: puzzle.volatility,
            };
            let outcome = if counted_solve {
                Outcomes::WIN
            } else {
                Outcomes::LOSS
            };
            let config = Glicko2Config {
                tau: 0.75,
                ..Default::default()
            };
            let (user_new, puzzle_new) = glicko2(&before, &puzzle_glicko, &outcome, &config);

            diesel::update(ratings::table.find(rating_id))
                .set((
                    user_rating.eq(user_new.rating),
                    user_deviation.eq(user_new.deviation),
                    user_volatility.eq(user_new.volatility),
                    user_played.eq(user_played + 1),
                    user_won.eq(user_won + i64::from(counted_solve)),
                    user_lost.eq(user_lost + i64::from(!counted_solve)),
                    rating_updated_at.eq(Utc::now()),
                ))
                .execute(tc)
                .await?;
            diesel::update(puzzles::table.find(puzzle_id))
                .set((
                    puzzle_rating.eq(puzzle_new.rating),
                    puzzle_deviation.eq(puzzle_new.deviation),
                    puzzle_volatility.eq(puzzle_new.volatility),
                    plays.eq(plays + 1),
                    solves.eq(solves + i32::from(counted_solve)),
                ))
                .execute(tc)
                .await?;
            diesel::insert_into(puzzle_attempts::table)
                .values((
                    attempt_user_id.eq(user_id),
                    attempt_puzzle_id.eq(puzzle_id),
                    solved.eq(was_solved),
                    rated.eq(true),
                    hinted.eq(used_hint),
                    time_ms.eq(elapsed_ms),
                    rating_before.eq(Some(before.rating)),
                    rating_after.eq(Some(user_new.rating)),
                ))
                .execute(tc)
                .await?;
            Ok(AttemptOutcome {
                rated: true,
                user_before: Some(before.rating),
                user_after: Some(user_new.rating),
                puzzle_before: puzzle.rating,
                puzzle_after: puzzle_new.rating,
            })
        })
        .await
    }

    /// Counts a guest's first try towards the puzzle's play statistics, without touching ratings.
    pub async fn record_guest_play(
        puzzle_id: Uuid,
        was_solved: bool,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        diesel::update(puzzles::table.find(puzzle_id))
            .set((
                plays.eq(plays + 1),
                solves.eq(solves + i32::from(was_solved)),
            ))
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Sets (or changes) a player's 0-5 stars and keeps the puzzle's sum and count in step. A
    /// puzzle enough players dislike is hidden.
    pub async fn set_stars(
        puzzle_id: Uuid,
        user_id: Uuid,
        stars: i16,
        conn: &mut DbConn<'_>,
    ) -> Result<Puzzle, DbError> {
        if !(0..=5).contains(&stars) {
            return Err(DbError::InvalidInput {
                info: "stars".to_string(),
                error: format!("{stars} is not between 0 and 5"),
            });
        }
        conn.transaction::<_, DbError, _>(async move |tc| {
            let previous: Option<i16> = puzzle_stars::table
                .filter(star_user_id.eq(user_id))
                .filter(star_puzzle_id.eq(puzzle_id))
                .select(star_value)
                .for_update()
                .first(tc)
                .await
                .optional()?;
            diesel::insert_into(puzzle_stars::table)
                .values((
                    star_user_id.eq(user_id),
                    star_puzzle_id.eq(puzzle_id),
                    star_value.eq(stars),
                ))
                .on_conflict((star_user_id, star_puzzle_id))
                .do_update()
                .set((star_value.eq(stars), star_updated_at.eq(Utc::now())))
                .execute(tc)
                .await?;
            let (added_votes, added_sum) = match previous {
                Some(old) => (0, i32::from(stars) - i32::from(old)),
                None => (1, i32::from(stars)),
            };
            let puzzle: Puzzle = diesel::update(puzzles::table.find(puzzle_id))
                .set((
                    stars_count.eq(stars_count + added_votes),
                    stars_sum.eq(stars_sum + added_sum),
                ))
                .returning(Puzzle::as_returning())
                .get_result(tc)
                .await?;
            let disliked = puzzle.stars_count >= AUTO_HIDE_MIN_VOTES
                && puzzle
                    .average_stars()
                    .is_some_and(|stars| stars < AUTO_HIDE_MAX_STARS);
            if disliked && !puzzle.hidden {
                return Ok(diesel::update(puzzles::table.find(puzzle_id))
                    .set(hidden.eq(true))
                    .returning(Puzzle::as_returning())
                    .get_result(tc)
                    .await?);
            }
            Ok(puzzle)
        })
        .await
    }

    pub async fn attempted(
        puzzle_id: Uuid,
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<bool, DbError> {
        Ok(select(exists(
            puzzle_attempts::table
                .filter(attempt_user_id.eq(user_id))
                .filter(attempt_puzzle_id.eq(puzzle_id)),
        ))
        .get_result(conn)
        .await?)
    }

    pub async fn stars_of(
        puzzle_id: Uuid,
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Option<i16>, DbError> {
        Ok(puzzle_stars::table
            .filter(star_user_id.eq(user_id))
            .filter(star_puzzle_id.eq(puzzle_id))
            .select(star_value)
            .first(conn)
            .await
            .optional()?)
    }

    /// A player's puzzles, most recent first: the latest try of each, plus their stars.
    pub async fn history(
        user_id: Uuid,
        filter: PuzzleHistoryFilter,
        page: i64,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<(PuzzleAttempt, Puzzle, Option<i16>)>, DbError> {
        let all: Vec<PuzzleAttempt> = puzzle_attempts::table
            .filter(attempt_user_id.eq(user_id))
            .order(attempt_created_at.desc())
            .select(PuzzleAttempt::as_select())
            .load(conn)
            .await?;
        // One row per puzzle, reporting the rated first try: replays never rewrite how it went.
        let mut attempts: Vec<PuzzleAttempt> = Vec::new();
        for attempt in &all {
            if attempts
                .iter()
                .any(|seen| seen.puzzle_id == attempt.puzzle_id)
            {
                continue;
            }
            let shown = all
                .iter()
                .find(|a| a.puzzle_id == attempt.puzzle_id && a.rated)
                .unwrap_or(attempt)
                .clone();
            attempts.push(shown);
        }
        // Ordered by when each puzzle was first met, so replaying an old one does not move it.
        attempts.sort_by_key(|attempt| std::cmp::Reverse(attempt.created_at));
        let ids: Vec<Uuid> = attempts.iter().map(|a| a.puzzle_id).collect();
        let found: Vec<Puzzle> = puzzles::table
            .filter(puzzles::id.eq_any(&ids))
            .select(Puzzle::as_select())
            .load(conn)
            .await?;
        let stars: Vec<(Uuid, i16)> = puzzle_stars::table
            .filter(star_user_id.eq(user_id))
            .filter(star_puzzle_id.eq_any(&ids))
            .select((star_puzzle_id, star_value))
            .load(conn)
            .await?;
        let rows = attempts
            .into_iter()
            .filter_map(|attempt| {
                let puzzle = found.iter().find(|p| p.id == attempt.puzzle_id)?.clone();
                let given = stars
                    .iter()
                    .find(|(id, _)| *id == attempt.puzzle_id)
                    .map(|(_, value)| *value);
                Some((attempt, puzzle, given))
            })
            .filter(|(attempt, _, given)| match filter {
                PuzzleHistoryFilter::All => true,
                PuzzleHistoryFilter::Solved => attempt.solved && !attempt.hinted,
                PuzzleHistoryFilter::Failed => !attempt.solved || attempt.hinted,
                PuzzleHistoryFilter::Unstarred => given.is_none(),
            })
            .skip((page.max(0) * HISTORY_PAGE) as usize)
            .take(HISTORY_PAGE as usize)
            .collect();
        Ok(rows)
    }
}
