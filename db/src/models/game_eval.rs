use crate::{db_error::DbError, schema::game_evals, DbConn};
use chrono::{DateTime, Duration, Utc};
use diesel::{
    result::{DatabaseErrorKind, Error as DieselError},
    sql_types::{
        Array,
        BigInt,
        Bool,
        Double,
        Integer,
        Jsonb,
        Nullable,
        Text,
        Timestamptz,
        Uuid as SqlUuid,
    },
    ExpressionMethods,
    Insertable,
    OptionalExtension,
    QueryDsl,
    Queryable,
    QueryableByName,
    Selectable,
    SelectableHelper,
};
use diesel_async::{AsyncConnection, RunQueryDsl};
use shared_types::{history_moves, EvalResult, EvalStatus, GameEvalView, GameId, RecentEval};
use uuid::Uuid;

/// Seconds per move before any eval has finished, from the prod benchmark of the
/// 50 > 800 scheme on three eval servers (~69 s for a median 41-move game).
const DEFAULT_SECS_PER_MOVE: f64 = 1.7;
const RECENT_FOR_ESTIMATE: i64 = 20;

/// (moves, started_at, finished_at) of a finished eval.
type FinishedRun = (i32, Option<DateTime<Utc>>, Option<DateTime<Utc>>);
/// (moves, progress_pct, started_at, worker) of an eval being worked on.
type RunningRow = (i32, i16, Option<DateTime<Utc>>, Option<String>);

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = game_evals)]
pub struct GameEval {
    pub id: Uuid,
    pub game_id: Uuid,
    pub requested_by: Option<Uuid>,
    pub status: String,
    pub moves: i32,
    pub progress_pct: i16,
    pub attempts: i16,
    pub worker: Option<String>,
    pub engine: Option<String>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub heartbeat_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    /// Started by the site on an idle queue, not by a user; user requests always go first.
    pub requested_by_system: bool,
}

#[derive(QueryableByName)]
struct RecentEvalRow {
    #[diesel(sql_type = Text)]
    nanoid: String,
    #[diesel(sql_type = Text)]
    white: String,
    #[diesel(sql_type = Text)]
    black: String,
    #[diesel(sql_type = Nullable<Double>)]
    white_rating: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    black_rating: Option<f64>,
    #[diesel(sql_type = Text)]
    game_status: String,
    #[diesel(sql_type = Text)]
    conclusion: String,
    #[diesel(sql_type = Integer)]
    turn: i32,
    #[diesel(sql_type = Nullable<Text>)]
    tournament: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    requester: Option<String>,
    #[diesel(sql_type = Bool)]
    automatic: bool,
    #[diesel(sql_type = Timestamptz)]
    finished_at: DateTime<Utc>,
    #[diesel(sql_type = Nullable<Jsonb>)]
    result: Option<serde_json::Value>,
}

#[derive(QueryableByName)]
struct AutoCandidate {
    #[diesel(sql_type = SqlUuid)]
    id: Uuid,
    #[diesel(sql_type = Text)]
    history: String,
}

/// Auto-evals only pick games where both players were at least this strong.
const AUTO_MIN_RATING: f64 = 1800.0;
/// Outside tournaments the front page only advertises games where both players were at least
/// this strong. Tournament games always show: a world championship field is not all
/// top-rated.
const FRONT_PAGE_MIN_RATING: f64 = 1800.0;
/// How long a finished tournament's games keep their place at the top of the front page.
const RECENT_TOURNAMENT_DAYS: i32 = 7;
const MIN_MOVES: i32 = shared_types::EVAL_MIN_MOVES as i32;

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(Insertable)]
#[diesel(table_name = game_evals)]
struct NewGameEval {
    game_id: Uuid,
    requested_by: Option<Uuid>,
    moves: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct RunningEval {
    pub moves: i32,
    pub progress_pct: i16,
    pub started_at: Option<DateTime<Utc>>,
}

/// Before this much of a game is done, its own pace says too little and the average of
/// recent evals stands in.
const OWN_PACE_FROM_PCT: i16 = 5;
const OWN_PACE_FROM_SECS: i64 = 10;

#[derive(Debug, Clone, Default)]
pub struct QueueSnapshot {
    /// (id, moves) in claim order.
    pub queued: Vec<(Uuid, i32)>,
    pub running: Vec<RunningEval>,
    pub workers: usize,
    pub secs_per_move: f64,
    /// Whether any worker is around to take queued evals; the database cannot tell, the
    /// caller fills it in.
    pub worker_online: bool,
}

impl QueueSnapshot {
    /// 1-based place in the queue, or None if the eval is not waiting.
    pub fn position(&self, eval_id: Uuid) -> Option<usize> {
        self.queued
            .iter()
            .position(|(id, _)| *id == eval_id)
            .map(|i| i + 1)
    }

    /// Seconds until `eval_id` is finished: the running work plus everything queued up to and
    /// including it, spread over the workers.
    pub fn wait_secs(&self, eval_id: Uuid) -> Option<u64> {
        let position = self.position(eval_id)?;
        let now = Utc::now();
        let running: f64 = self
            .running
            .iter()
            .map(|eval| self.left_secs(*eval, now))
            .sum();
        let queued: f64 = self.queued[..position]
            .iter()
            .map(|(_, moves)| *moves as f64 * self.secs_per_move)
            .sum();
        Some(((running + queued) / self.workers.max(1) as f64).ceil() as u64)
    }

    pub fn remaining_secs(&self, eval: RunningEval) -> u64 {
        self.left_secs(eval, Utc::now()).ceil() as u64
    }

    /// Seconds a running eval still needs. Once it is under way, its own pace so far: workers
    /// differ, a home machine can be several times faster than the server, and the average
    /// of recent evals mixes them all.
    fn left_secs(&self, eval: RunningEval, now: DateTime<Utc>) -> f64 {
        let pct = eval.progress_pct.clamp(0, 100);
        let elapsed = eval
            .started_at
            .map(|started| (now - started).num_milliseconds());
        match elapsed {
            Some(ms) if pct >= OWN_PACE_FROM_PCT && ms >= OWN_PACE_FROM_SECS * 1000 => {
                ms as f64 / 1000.0 * (100 - pct) as f64 / pct as f64
            }
            _ => eval.moves as f64 * (100 - pct) as f64 / 100.0 * self.secs_per_move,
        }
    }
}

impl GameEval {
    pub fn status(&self) -> EvalStatus {
        self.status.parse().unwrap_or(EvalStatus::Failed)
    }

    pub fn view(&self, queue: &QueueSnapshot, viewer: Option<Uuid>) -> GameEvalView {
        match self.status() {
            EvalStatus::Queued => GameEvalView::Queued {
                position: queue.position(self.id).unwrap_or(1),
                mine: viewer.is_some() && viewer == self.requested_by,
                queue_len: queue.queued.len().max(1),
                wait_secs: queue
                    .worker_online
                    .then(|| queue.wait_secs(self.id).unwrap_or(0)),
            },
            EvalStatus::Running => GameEvalView::Running {
                progress_pct: self.progress_pct.clamp(0, 100) as u8,
                wait_secs: queue.remaining_secs(RunningEval {
                    moves: self.moves,
                    progress_pct: self.progress_pct,
                    started_at: self.started_at,
                }),
            },
            EvalStatus::Done => self
                .result()
                .map(GameEvalView::Done)
                .unwrap_or(GameEvalView::Failed),
            EvalStatus::Failed => GameEvalView::Failed,
        }
    }

    pub fn result(&self) -> Option<EvalResult> {
        self.result
            .as_ref()
            .and_then(|value| serde_json::from_value::<EvalResult>(value.clone()).ok())
            .map(EvalResult::for_display)
    }

    pub async fn find(eval: Uuid, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        use crate::schema::game_evals::dsl::*;
        Ok(game_evals
            .find(eval)
            .select(GameEval::as_select())
            .first(conn)
            .await?)
    }

    pub async fn find_by_game(game: Uuid, conn: &mut DbConn<'_>) -> Result<Option<Self>, DbError> {
        use crate::schema::game_evals::dsl::*;
        Ok(game_evals
            .filter(game_id.eq(game))
            .select(GameEval::as_select())
            .first(conn)
            .await
            .optional()?)
    }

    pub async fn evaluated_among(
        nanoids: &[String],
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<String>, DbError> {
        #[derive(QueryableByName)]
        struct Row {
            #[diesel(sql_type = Text)]
            nanoid: String,
        }
        let rows: Vec<Row> = diesel::sql_query(
            "SELECT g.nanoid FROM game_evals e JOIN games g ON g.id = e.game_id \
             WHERE e.status = 'done' AND g.nanoid = ANY($1)",
        )
        .bind::<Array<Text>, _>(nanoids)
        .load(conn)
        .await?;
        Ok(rows.into_iter().map(|row| row.nanoid).collect())
    }

    /// Takes back `user`'s request for `game` while it still waits; false if there was none.
    pub async fn cancel(game: Uuid, user: Uuid, conn: &mut DbConn<'_>) -> Result<bool, DbError> {
        use crate::schema::game_evals::dsl::*;
        let deleted = diesel::delete(
            game_evals
                .filter(game_id.eq(game))
                .filter(requested_by.eq(user))
                .filter(status.eq(EvalStatus::Queued.to_string())),
        )
        .execute(conn)
        .await?;
        Ok(deleted > 0)
    }

    /// Queues an eval of `game` for `user`. An existing eval is returned as is unless it failed,
    /// in which case it is queued again for this user.
    pub async fn request(
        game: Uuid,
        user: Uuid,
        game_moves: i32,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        use crate::schema::game_evals::dsl::*;
        let attempt = conn
            .transaction::<_, DieselError, _>(async move |tc| {
                let existing: Option<GameEval> = game_evals
                    .filter(game_id.eq(game))
                    .for_update()
                    .select(GameEval::as_select())
                    .first(tc)
                    .await
                    .optional()?;
                let active = |eval: &GameEval| {
                    matches!(eval.status(), EvalStatus::Queued | EvalStatus::Running)
                };
                match existing {
                    // Asking for the game the site is already on: it becomes this user's eval.
                    Some(eval) if eval.requested_by_system && active(&eval) => {
                        diesel::update(game_evals.find(eval.id))
                            .set((requested_by.eq(Some(user)), requested_by_system.eq(false)))
                            .returning(GameEval::as_returning())
                            .get_result(tc)
                            .await
                    }
                    Some(eval) if eval.status() != EvalStatus::Failed => Ok(eval),
                    Some(eval) => {
                        Self::drop_active_auto_evals(tc).await?;
                        diesel::update(game_evals.find(eval.id))
                            .set((
                                status.eq(EvalStatus::Queued.to_string()),
                                requested_by.eq(Some(user)),
                                requested_by_system.eq(false),
                                progress_pct.eq(0),
                                attempts.eq(0),
                                worker.eq(None::<String>),
                                error.eq(None::<String>),
                                created_at.eq(Utc::now()),
                                started_at.eq(None::<DateTime<Utc>>),
                                heartbeat_at.eq(None::<DateTime<Utc>>),
                                finished_at.eq(None::<DateTime<Utc>>),
                            ))
                            .returning(GameEval::as_returning())
                            .get_result(tc)
                            .await
                    }
                    None => {
                        Self::drop_active_auto_evals(tc).await?;
                        diesel::insert_into(game_evals)
                            .values(NewGameEval {
                                game_id: game,
                                requested_by: Some(user),
                                moves: game_moves,
                            })
                            .returning(GameEval::as_returning())
                            .get_result(tc)
                            .await
                    }
                }
            })
            .await;
        match attempt {
            Ok(eval) => Ok(eval),
            Err(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, info))
                if info.constraint_name() == Some("game_evals_one_active_per_user") =>
            {
                Err(DbError::InvalidAction {
                    info: "You already have an eval queued or running.".to_string(),
                })
            }
            // Someone else requested the same game between our read and insert.
            Err(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, info))
                if info.constraint_name() == Some("game_evals_game_id") =>
            {
                Self::find_by_game(game, conn)
                    .await?
                    .ok_or(DbError::InternalError)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Hands the next eval to `worker_name`: user requests oldest first, then auto-evals; with
    /// nothing queued, the site picks a game itself.
    pub async fn claim(
        worker_name: &str,
        auto: bool,
        conn: &mut DbConn<'_>,
    ) -> Result<Option<Self>, DbError> {
        use crate::schema::game_evals::dsl::*;
        let worker_name = worker_name.to_string();
        conn.transaction::<_, DbError, _>(async move |tc| {
            let next: Option<Uuid> = game_evals
                .filter(status.eq(EvalStatus::Queued.to_string()))
                .order((requested_by_system.asc(), created_at.asc()))
                .select(id)
                .for_update()
                .skip_locked()
                .first(tc)
                .await
                .optional()?;
            let now = Utc::now();
            let Some(next) = next else {
                if !auto {
                    return Ok(None);
                }
                let Some(candidate) = Self::auto_candidate(tc).await? else {
                    return Ok(None);
                };
                // Another idle worker may have picked the same game; it then simply waits.
                return Ok(diesel::insert_into(game_evals)
                    .values((
                        game_id.eq(candidate.id),
                        moves.eq(history_moves(&candidate.history).len() as i32),
                        requested_by_system.eq(true),
                        status.eq(EvalStatus::Running.to_string()),
                        worker.eq(Some(worker_name)),
                        attempts.eq(1),
                        started_at.eq(Some(now)),
                        heartbeat_at.eq(Some(now)),
                    ))
                    .on_conflict_do_nothing()
                    .returning(GameEval::as_returning())
                    .get_result(tc)
                    .await
                    .optional()?);
            };
            Ok(Some(
                diesel::update(game_evals.find(next))
                    .set((
                        status.eq(EvalStatus::Running.to_string()),
                        worker.eq(Some(worker_name)),
                        progress_pct.eq(0),
                        attempts.eq(attempts + 1),
                        started_at.eq(Some(now)),
                        heartbeat_at.eq(Some(now)),
                    ))
                    .returning(GameEval::as_returning())
                    .get_result(tc)
                    .await?,
            ))
        })
        .await
    }

    /// Makes room for a user: their request never waits behind an auto-eval. A running one's
    /// worker learns at its next progress report and stops.
    async fn drop_active_auto_evals(conn: &mut DbConn<'_>) -> Result<usize, DieselError> {
        use crate::schema::game_evals::dsl::*;
        diesel::delete(
            game_evals
                .filter(requested_by_system.eq(true))
                .filter(status.eq_any([
                    EvalStatus::Queued.to_string(),
                    EvalStatus::Running.to_string(),
                ])),
        )
        .execute(conn)
        .await
    }

    /// The most prominent game without an eval: tournament games first, then the stronger
    /// weaker player, then the most recent.
    async fn auto_candidate(conn: &mut DbConn<'_>) -> Result<Option<AutoCandidate>, DbError> {
        Ok(diesel::sql_query(
            r#"
            SELECT g.id, g.history
            FROM games g
            WHERE g.finished
                AND g.game_type = $1
                AND g.tournament_queen_rule
                AND g.turn >= $2
                AND g.white_rating >= $3
                AND g.black_rating >= $3
                AND NOT EXISTS (SELECT 1 FROM game_evals e WHERE e.game_id = g.id)
            ORDER BY (g.tournament_id IS NOT NULL) DESC,
                LEAST(g.white_rating, g.black_rating) DESC,
                g.updated_at DESC
            LIMIT 1
            "#,
        )
        .bind::<Text, _>(hive_lib::GameType::MLP.to_string())
        .bind::<Integer, _>(MIN_MOVES)
        .bind::<Double, _>(AUTO_MIN_RATING)
        .get_result(conn)
        .await
        .optional()?)
    }

    /// Records progress. False when the eval is no longer this worker's to report on, for
    /// example after the sweeper gave it to another worker.
    pub async fn report_progress(
        eval: Uuid,
        worker_name: &str,
        pct: i16,
        conn: &mut DbConn<'_>,
    ) -> Result<bool, DbError> {
        use crate::schema::game_evals::dsl::*;
        let updated = diesel::update(Self::owned(eval, worker_name))
            .set((
                progress_pct.eq(pct.clamp(0, 99)),
                heartbeat_at.eq(Some(Utc::now())),
            ))
            .execute(conn)
            .await?;
        Ok(updated == 1)
    }

    pub async fn finish(
        eval: Uuid,
        worker_name: &str,
        engine_id: &str,
        eval_result: &EvalResult,
        conn: &mut DbConn<'_>,
    ) -> Result<bool, DbError> {
        use crate::schema::game_evals::dsl::*;
        let value = serde_json::to_value(eval_result).map_err(|_| DbError::InternalError)?;
        let now = Utc::now();
        let updated = diesel::update(Self::owned(eval, worker_name))
            .set((
                status.eq(EvalStatus::Done.to_string()),
                progress_pct.eq(100),
                engine.eq(Some(engine_id)),
                result.eq(Some(value)),
                heartbeat_at.eq(Some(now)),
                finished_at.eq(Some(now)),
            ))
            .execute(conn)
            .await?;
        Ok(updated == 1)
    }

    pub async fn fail(
        eval: Uuid,
        worker_name: &str,
        reason: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<bool, DbError> {
        use crate::schema::game_evals::dsl::*;
        let now = Utc::now();
        let updated = diesel::update(Self::owned(eval, worker_name))
            .set((
                status.eq(EvalStatus::Failed.to_string()),
                error.eq(Some(reason)),
                finished_at.eq(Some(now)),
            ))
            .execute(conn)
            .await?;
        Ok(updated == 1)
    }

    /// Running evals whose worker has gone quiet for `stale` go back to the queue, or fail
    /// once they have been tried `max_attempts` times. Returns (requeued, failed).
    pub async fn sweep_stale(
        stale: Duration,
        max_attempts: i16,
        conn: &mut DbConn<'_>,
    ) -> Result<(usize, usize), DbError> {
        use crate::schema::game_evals::dsl::*;
        let cutoff = Utc::now() - stale;
        let quiet = || {
            game_evals
                .filter(status.eq(EvalStatus::Running.to_string()))
                .filter(heartbeat_at.lt(cutoff))
        };
        let failed = diesel::update(quiet().filter(attempts.ge(max_attempts)))
            .set((
                status.eq(EvalStatus::Failed.to_string()),
                error.eq(Some("worker stopped responding")),
                finished_at.eq(Some(Utc::now())),
            ))
            .execute(conn)
            .await?;
        let requeued = diesel::update(quiet().filter(attempts.lt(max_attempts)))
            .set((
                status.eq(EvalStatus::Queued.to_string()),
                worker.eq(None::<String>),
                progress_pct.eq(0),
                started_at.eq(None::<DateTime<Utc>>),
                heartbeat_at.eq(None::<DateTime<Utc>>),
            ))
            .execute(conn)
            .await?;
        Ok((requeued, failed))
    }

    /// Finished evals of tournament or strong games for the front page, each group newest
    /// first: games of running or just-finished tournaments, then user requests, then
    /// automatic evals. A finished tournament only stays on top for a week, so the auto-evals,
    /// which start with tournament games, never crowd out what people asked for.
    pub async fn recent(limit: i64, conn: &mut DbConn<'_>) -> Result<Vec<RecentEval>, DbError> {
        let rows: Vec<RecentEvalRow> = diesel::sql_query(
            r#"
            SELECT * FROM (
                SELECT g.nanoid, w.username AS white, b.username AS black,
                       g.white_rating, g.black_rating, g.game_status, g.conclusion, g.turn,
                       t.name AS tournament, r.username AS requester,
                       e.requested_by_system AS automatic, e.finished_at, e.result,
                       CASE
                           WHEN t.status = 'InProgress'
                               OR (t.status = 'Finished'
                                   AND COALESCE(t.ends_at, t.updated_at)
                                       > now() - make_interval(days => $3)) THEN 0
                           WHEN NOT e.requested_by_system THEN 1
                           ELSE 2
                       END AS place
                FROM game_evals e
                JOIN games g ON g.id = e.game_id
                JOIN users w ON w.id = g.white_id
                JOIN users b ON b.id = g.black_id
                LEFT JOIN tournaments t ON t.id = g.tournament_id
                LEFT JOIN users r ON r.id = e.requested_by
                WHERE e.status = 'done' AND e.finished_at IS NOT NULL AND g.turn >= $4
                    AND (t.id IS NOT NULL OR LEAST(g.white_rating, g.black_rating) >= $2)
            ) ranked
            ORDER BY place, finished_at DESC
            LIMIT $1
            "#,
        )
        .bind::<BigInt, _>(limit)
        .bind::<Double, _>(FRONT_PAGE_MIN_RATING)
        .bind::<Integer, _>(RECENT_TOURNAMENT_DAYS)
        .bind::<Integer, _>(MIN_MOVES)
        .load(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| RecentEval {
                game_id: GameId(row.nanoid),
                white: row.white,
                black: row.black,
                white_rating: row.white_rating,
                black_rating: row.black_rating,
                game_status: row.game_status,
                conclusion: row.conclusion,
                moves: row.turn,
                tournament: row.tournament,
                requested_by: row.requester,
                automatic: row.automatic,
                finished_at: row.finished_at,
                marks: row
                    .result
                    .and_then(|value| serde_json::from_value::<EvalResult>(value).ok())
                    .map_or([0; 3], |result| result.for_display().marks(None)),
            })
            .collect())
    }

    pub async fn in_line(conn: &mut DbConn<'_>) -> Result<usize, DbError> {
        let row: CountRow = diesel::sql_query(
            "SELECT count(*) AS count FROM game_evals WHERE status IN ('queued', 'running')",
        )
        .get_result(conn)
        .await?;
        Ok(row.count as usize)
    }

    pub async fn queue_snapshot(conn: &mut DbConn<'_>) -> Result<QueueSnapshot, DbError> {
        use crate::schema::game_evals::dsl::*;
        let queued: Vec<(Uuid, i32)> = game_evals
            .filter(status.eq(EvalStatus::Queued.to_string()))
            .order((requested_by_system.asc(), created_at.asc()))
            .select((id, moves))
            .load(conn)
            .await?;
        let running_rows: Vec<RunningRow> = game_evals
            .filter(status.eq(EvalStatus::Running.to_string()))
            .select((moves, progress_pct, started_at, worker))
            .load(conn)
            .await?;
        let recent: Vec<FinishedRun> = game_evals
            .filter(status.eq(EvalStatus::Done.to_string()))
            .order(finished_at.desc())
            .limit(RECENT_FOR_ESTIMATE)
            .select((moves, started_at, finished_at))
            .load(conn)
            .await?;

        let mut workers: Vec<&str> = running_rows
            .iter()
            .filter_map(|(_, _, _, w)| w.as_deref())
            .collect();
        workers.sort_unstable();
        workers.dedup();

        Ok(QueueSnapshot {
            queued,
            running: running_rows
                .iter()
                .map(|(m, p, s, _)| RunningEval {
                    moves: *m,
                    progress_pct: *p,
                    started_at: *s,
                })
                .collect(),
            workers: workers.len(),
            secs_per_move: secs_per_move(&recent),
            worker_online: false,
        })
    }

    #[diesel::dsl::auto_type(no_type_alias)]
    fn owned(eval: Uuid, worker_name: &str) -> _ {
        let running: String = EvalStatus::Running.to_string();
        let worker_name: String = worker_name.to_string();
        game_evals::table
            .filter(game_evals::id.eq(eval))
            .filter(game_evals::status.eq(running))
            .filter(game_evals::worker.eq(worker_name))
    }
}

fn secs_per_move(recent: &[FinishedRun]) -> f64 {
    let (secs, moves) = recent
        .iter()
        .filter_map(|(m, start, end)| Some((((*end)? - (*start)?).num_milliseconds(), *m)))
        .fold((0i64, 0i64), |(s, n), (ms, m)| (s + ms, n + m as i64));
    if moves == 0 {
        DEFAULT_SECS_PER_MOVE
    } else {
        secs as f64 / 1000.0 / moves as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(
        queued: &[i32],
        running: &[(i32, i16)],
        workers: usize,
    ) -> (QueueSnapshot, Vec<Uuid>) {
        let ids: Vec<Uuid> = queued.iter().map(|_| Uuid::new_v4()).collect();
        (
            QueueSnapshot {
                queued: ids.iter().copied().zip(queued.iter().copied()).collect(),
                running: running
                    .iter()
                    .map(|(moves, progress_pct)| RunningEval {
                        moves: *moves,
                        progress_pct: *progress_pct,
                        started_at: None,
                    })
                    .collect(),
                workers,
                secs_per_move: 2.0,
                worker_online: true,
            },
            ids,
        )
    }

    #[test]
    fn wait_counts_the_running_remainder_and_everything_up_to_this_eval() {
        let (snap, ids) = snapshot(&[40, 50, 60], &[(100, 25)], 1);
        // 75 moves left on the running eval, then 40 + 50 queued ahead and including ours.
        assert_eq!(snap.wait_secs(ids[1]), Some((75 + 40 + 50) * 2));
        assert_eq!(snap.position(ids[1]), Some(2));
    }

    #[test]
    fn wait_is_shared_between_workers() {
        let (snap, ids) = snapshot(&[40], &[(40, 0)], 2);
        assert_eq!(snap.wait_secs(ids[0]), Some(80));
    }

    #[test]
    fn an_eval_not_in_the_queue_has_no_wait() {
        let (snap, _) = snapshot(&[40], &[], 0);
        assert_eq!(snap.wait_secs(Uuid::new_v4()), None);
    }

    #[test]
    fn a_running_eval_is_timed_by_its_own_pace_once_under_way() {
        let (snap, _) = snapshot(&[], &[], 1);
        let started = |secs: i64| Some(Utc::now() - Duration::seconds(secs));
        // 40 seconds for a quarter: three quarters to go take 120, whatever the average says.
        let fast = RunningEval {
            moves: 400,
            progress_pct: 25,
            started_at: started(40),
        };
        assert!((119..=121).contains(&snap.remaining_secs(fast)));
        // Just started: the average (2 s a move) is all there is.
        let fresh = RunningEval {
            moves: 50,
            progress_pct: 2,
            started_at: started(3),
        };
        assert_eq!(snap.remaining_secs(fresh), 98);
    }

    #[test]
    fn secs_per_move_averages_by_move_not_by_eval() {
        let t = Utc::now();
        let recent = vec![
            (10, Some(t), Some(t + Duration::seconds(10))),
            (90, Some(t), Some(t + Duration::seconds(270))),
        ];
        assert_eq!(secs_per_move(&recent), 2.8);
    }

    fn eval_with(status: EvalStatus, moves: i32, progress_pct: i16) -> GameEval {
        GameEval {
            id: Uuid::new_v4(),
            game_id: Uuid::new_v4(),
            requested_by: None,
            status: status.to_string(),
            moves,
            progress_pct,
            attempts: 0,
            worker: None,
            engine: None,
            result: None,
            error: None,
            created_at: Utc::now(),
            started_at: None,
            heartbeat_at: None,
            finished_at: None,
            requested_by_system: false,
        }
    }

    #[test]
    fn a_running_eval_shows_its_progress_and_what_is_left() {
        let (snap, _) = snapshot(&[], &[(40, 50)], 1);
        assert_eq!(
            eval_with(EvalStatus::Running, 40, 50).view(&snap, None),
            GameEvalView::Running {
                progress_pct: 50,
                wait_secs: 40
            }
        );
    }

    #[test]
    fn a_queued_eval_shows_its_place_and_wait() {
        let mut eval = eval_with(EvalStatus::Queued, 30, 0);
        let (mut snap, _) = snapshot(&[20], &[], 1);
        snap.queued.push((eval.id, 30));
        eval.moves = 30;
        assert_eq!(
            eval.view(&snap, None),
            GameEvalView::Queued {
                position: 2,
                mine: false,
                queue_len: 2,
                wait_secs: Some(100)
            }
        );
    }

    #[test]
    fn only_the_requester_sees_a_queued_eval_as_theirs() {
        let mut eval = eval_with(EvalStatus::Queued, 30, 0);
        let asker = Uuid::new_v4();
        eval.requested_by = Some(asker);
        let (mut snap, _) = snapshot(&[], &[], 1);
        snap.queued.push((eval.id, 30));
        let mine = |viewer| {
            matches!(
                eval.view(&snap, viewer),
                GameEvalView::Queued { mine: true, .. }
            )
        };
        assert!(mine(Some(asker)));
        assert!(!mine(Some(Uuid::new_v4())));
        assert!(!mine(None));
    }

    #[test]
    fn without_a_worker_a_queued_eval_promises_no_time() {
        let eval = eval_with(EvalStatus::Queued, 30, 0);
        let (mut snap, _) = snapshot(&[], &[], 0);
        snap.queued.push((eval.id, 30));
        snap.worker_online = false;
        assert_eq!(
            eval.view(&snap, None),
            GameEvalView::Queued {
                position: 1,
                mine: false,
                queue_len: 1,
                wait_secs: None
            }
        );
    }

    #[test]
    fn a_done_eval_with_an_unreadable_result_shows_as_failed() {
        let mut eval = eval_with(EvalStatus::Done, 2, 100);
        eval.result = Some(serde_json::json!({"not": "a result"}));
        assert_eq!(
            eval.view(&QueueSnapshot::default(), None),
            GameEvalView::Failed
        );
    }

    #[test]
    fn secs_per_move_falls_back_without_history() {
        assert_eq!(secs_per_move(&[]), DEFAULT_SECS_PER_MOVE);
    }
}
