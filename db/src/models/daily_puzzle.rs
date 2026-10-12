use crate::{
    db_error::DbError,
    models::Puzzle,
    schema::{
        daily_puzzle_queue::{
            self,
            created_at as queued_at,
            puzzle_id as queued_puzzle_id,
            queued_by,
        },
        daily_puzzles::{self, day as daily_day, puzzle_id as daily_puzzle_id},
        puzzle_attempts::{
            self,
            created_at as attempt_created_at,
            hinted as attempt_hinted,
            puzzle_id as attempt_puzzle_id,
            rated as attempt_rated,
            solved as attempt_solved,
            user_id as attempt_user_id,
        },
        puzzles::{self, hidden, nanoid as puzzle_nanoid},
    },
    DbConn,
};
use chrono::{DateTime, Datelike, NaiveDate, Utc, Weekday};
use diesel::{dsl::exists, prelude::*, select};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = daily_puzzles)]
pub struct DailyPuzzle {
    pub day: NaiveDate,
    pub puzzle_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DailyEntry {
    pub day: NaiveDate,
    pub nanoid: String,
}

#[derive(Debug, Clone)]
pub struct QueuedPuzzle {
    pub puzzle: Puzzle,
    pub queued_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DailyTally {
    pub tried: i64,
    pub solved: i64,
}

struct Band {
    shortest: i32,
    longest: i32,
    lowest: f64,
    highest: f64,
}

/// Monday is gentle, Sunday is hard, like a newspaper crossword.
fn band(day: NaiveDate) -> Band {
    let (shortest, longest, lowest, highest) = match day.weekday() {
        Weekday::Mon => (2, 2, 0.0, 1300.0),
        Weekday::Tue => (2, 2, 1200.0, 1450.0),
        Weekday::Wed => (2, 3, 1350.0, 1550.0),
        Weekday::Thu => (3, 3, 1450.0, 1650.0),
        Weekday::Fri => (3, 3, 1550.0, 1750.0),
        Weekday::Sat => (3, 99, 1650.0, 1900.0),
        Weekday::Sun => (3, 99, 1750.0, 9999.0),
    };
    Band {
        shortest,
        longest,
        lowest,
        highest,
    }
}

/// Players' stars count most (shrunk towards 3 until enough votes), then a solve rate near half,
/// then the interesting ideas; the random term keeps equal puzzles rotating.
const PICK: &str = "INSERT INTO daily_puzzles (day, puzzle_id)
    SELECT $1, p.id FROM puzzles p
    WHERE NOT p.hidden
      AND p.game_type = 'Base+MLP'
      AND p.collection IS NULL
      AND NOT p.themes @> ARRAY['easy']
      AND p.solver_moves BETWEEN $2 AND $3
      AND p.rating BETWEEN $4 AND $5
      AND NOT EXISTS (SELECT 1 FROM daily_puzzles d WHERE d.puzzle_id = p.id)
      AND NOT EXISTS (SELECT 1 FROM daily_puzzle_queue q WHERE q.puzzle_id = p.id)
    ORDER BY (p.stars_sum + 15.0) / (p.stars_count + 5.0)
      - CASE WHEN p.plays >= 10 THEN 2.0 * abs(p.solves::float / p.plays - 0.55) ELSE 0 END
      + CASE WHEN p.themes && ARRAY['quietMove','throw','queenThrow','mosquito','missedWin']
             THEN 0.4 ELSE 0 END
      + LEAST(p.plays, 50) / 250.0
      + random() * 0.6 DESC
    LIMIT 1
    ON CONFLICT DO NOTHING";

impl DailyPuzzle {
    /// The puzzle of `day`, choosing it on first request: the oldest queued puzzle, else the best
    /// unused one for the weekday. Racing requests settle on whichever insert wins.
    pub async fn for_day(
        day: NaiveDate,
        conn: &mut DbConn<'_>,
    ) -> Result<Option<(DailyPuzzle, Puzzle)>, DbError> {
        if let Some(found) = Self::find(day, conn).await? {
            return Ok(Some(found));
        }
        if !Self::take_from_queue(day, conn).await? {
            Self::pick(day, conn).await?;
        }
        Self::find(day, conn).await
    }

    async fn find(
        day: NaiveDate,
        conn: &mut DbConn<'_>,
    ) -> Result<Option<(DailyPuzzle, Puzzle)>, DbError> {
        Ok(daily_puzzles::table
            .inner_join(puzzles::table)
            .filter(daily_day.eq(day))
            .select((DailyPuzzle::as_select(), Puzzle::as_select()))
            .first(conn)
            .await
            .optional()?)
    }

    async fn take_from_queue(day: NaiveDate, conn: &mut DbConn<'_>) -> Result<bool, DbError> {
        let next: Option<Uuid> = daily_puzzle_queue::table
            .inner_join(puzzles::table)
            .filter(hidden.eq(false))
            .filter(diesel::dsl::not(exists(
                daily_puzzles::table.filter(daily_puzzle_id.eq(queued_puzzle_id)),
            )))
            .order(queued_at.asc())
            .select(queued_puzzle_id)
            .first(conn)
            .await
            .optional()?;
        let Some(next) = next else {
            return Ok(false);
        };
        let inserted = diesel::insert_into(daily_puzzles::table)
            .values((daily_day.eq(day), daily_puzzle_id.eq(next)))
            .on_conflict_do_nothing()
            .execute(conn)
            .await?;
        if inserted == 1 {
            diesel::delete(daily_puzzle_queue::table.filter(queued_puzzle_id.eq(next)))
                .execute(conn)
                .await?;
        }
        Ok(true)
    }

    async fn pick(day: NaiveDate, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        use diesel::sql_types::{Date, Double, Integer};
        let band = band(day);
        let attempts = [
            (band.shortest, band.longest, band.lowest, band.highest),
            (band.shortest, band.longest, 0.0, f64::MAX),
            (1, 99, 0.0, f64::MAX),
        ];
        for (shortest, longest, lowest, highest) in attempts {
            let inserted = diesel::sql_query(PICK)
                .bind::<Date, _>(day)
                .bind::<Integer, _>(shortest)
                .bind::<Integer, _>(longest)
                .bind::<Double, _>(lowest)
                .bind::<Double, _>(highest)
                .execute(conn)
                .await?;
            if inserted > 0 || Self::find(day, conn).await?.is_some() {
                return Ok(());
            }
        }
        Ok(())
    }

    pub async fn day_of(
        puzzle_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Option<NaiveDate>, DbError> {
        Ok(daily_puzzles::table
            .filter(daily_puzzle_id.eq(puzzle_id))
            .select(daily_day)
            .first(conn)
            .await
            .optional()?)
    }

    pub async fn days_of(
        puzzle_ids: &[Uuid],
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<(Uuid, NaiveDate)>, DbError> {
        Ok(daily_puzzles::table
            .filter(daily_puzzle_id.eq_any(puzzle_ids))
            .select((daily_puzzle_id, daily_day))
            .load(conn)
            .await?)
    }

    /// Rated first tries and unassisted solves on `day` itself; replays never count.
    pub async fn tally(
        puzzle_id: Uuid,
        day: NaiveDate,
        conn: &mut DbConn<'_>,
    ) -> Result<DailyTally, DbError> {
        let Some(start) = day.and_hms_opt(0, 0, 0).map(|start| start.and_utc()) else {
            return Ok(DailyTally::default());
        };
        let end = start + chrono::Duration::days(1);
        let today = puzzle_attempts::table
            .filter(attempt_puzzle_id.eq(puzzle_id))
            .filter(attempt_created_at.ge(start))
            .filter(attempt_rated.eq(true))
            .filter(attempt_created_at.lt(end));
        let tried = today
            .select(diesel::dsl::count(attempt_user_id).aggregate_distinct())
            .first(conn)
            .await?;
        let solved = today
            .filter(attempt_solved.eq(true))
            .filter(attempt_hinted.eq(false))
            .select(diesel::dsl::count(attempt_user_id).aggregate_distinct())
            .first(conn)
            .await?;
        Ok(DailyTally { tried, solved })
    }

    /// How `user`'s first try went: `Some(true)` solved without help, `Some(false)` missed.
    pub async fn first_result(
        puzzle_id: Uuid,
        user: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Option<bool>, DbError> {
        Ok(puzzle_attempts::table
            .filter(attempt_puzzle_id.eq(puzzle_id))
            .filter(attempt_user_id.eq(user))
            .order(attempt_created_at.asc())
            .select((attempt_solved, attempt_hinted))
            .first::<(bool, bool)>(conn)
            .await
            .optional()?
            .map(|(was_solved, was_hinted)| was_solved && !was_hinted))
    }

    pub async fn recent(
        from: NaiveDate,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<DailyEntry>, DbError> {
        Ok(daily_puzzles::table
            .inner_join(puzzles::table)
            .filter(daily_day.ge(from))
            .order(daily_day.desc())
            .select((daily_day, puzzle_nanoid))
            .load::<(NaiveDate, String)>(conn)
            .await?
            .into_iter()
            .map(|(day, nanoid)| DailyEntry { day, nanoid })
            .collect())
    }

    pub async fn queued(conn: &mut DbConn<'_>) -> Result<Vec<QueuedPuzzle>, DbError> {
        Ok(daily_puzzle_queue::table
            .inner_join(puzzles::table)
            .order(queued_at.asc())
            .select((Puzzle::as_select(), queued_at))
            .load::<(Puzzle, DateTime<Utc>)>(conn)
            .await?
            .into_iter()
            .map(|(puzzle, at)| QueuedPuzzle {
                puzzle,
                queued_at: at,
            })
            .collect())
    }

    pub async fn enqueue(nanoid: &str, admin: Uuid, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        let puzzle = Puzzle::find_by_nanoid(nanoid, conn).await?;
        if puzzle.hidden {
            return Err(DbError::InvalidAction {
                info: "That puzzle is hidden.".to_string(),
            });
        }
        let used: bool = select(exists(
            daily_puzzles::table.filter(daily_puzzle_id.eq(puzzle.id)),
        ))
        .get_result(conn)
        .await?;
        if used {
            return Err(DbError::InvalidAction {
                info: "That puzzle was already a puzzle of the day.".to_string(),
            });
        }
        diesel::insert_into(daily_puzzle_queue::table)
            .values((queued_puzzle_id.eq(puzzle.id), queued_by.eq(admin)))
            .on_conflict_do_nothing()
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn dequeue(nanoid: &str, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        let puzzle = Puzzle::find_by_nanoid(nanoid, conn).await?;
        diesel::delete(daily_puzzle_queue::table.filter(queued_puzzle_id.eq(puzzle.id)))
            .execute(conn)
            .await?;
        Ok(())
    }
}
