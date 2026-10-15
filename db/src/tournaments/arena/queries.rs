use crate::{
    db_error::DbError,
    models::Game,
    schema::{games, tournaments},
    DbConn,
};
use chrono::{DateTime, Duration, Utc};
use diesel::{
    dsl::sql,
    pg::Pg,
    prelude::*,
    sql_types::{Text, Timestamptz},
};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

const ARENA_END_SQL: &str = "tournaments.starts_at + make_interval(secs => ((tournaments.configuration #>> '{format,configuration,duration_seconds}')::integer))";

fn live_pairing_query(as_of: DateTime<Utc>) -> tournaments::BoxedQuery<'static, Pg> {
    tournaments::table
        .into_boxed()
        .filter(sql::<Text>("configuration #>> '{format,format}'").eq("arena"))
        .filter(tournaments::starts_at.le(as_of))
        .filter(tournaments::started_at.is_not_null())
        .filter(tournaments::finished_at.is_null())
        .filter(sql::<Timestamptz>(ARENA_END_SQL).gt(as_of + Duration::seconds(60)))
}

pub async fn scheduled_start_candidates(
    as_of: DateTime<Utc>,
    limit: i64,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Uuid>, DbError> {
    if limit <= 0 {
        return Ok(Vec::new());
    }
    Ok(tournaments::table
        .filter(sql::<Text>("configuration #>> '{format,format}'").eq("arena"))
        .filter(tournaments::starts_at.le(as_of))
        .filter(tournaments::started_at.is_null())
        .filter(tournaments::finished_at.is_null())
        .order((tournaments::starts_at.asc(), tournaments::id.asc()))
        .limit(limit)
        .select(tournaments::id)
        .load(conn)
        .await?)
}

pub async fn pairing_candidates(
    after_id: Option<Uuid>,
    as_of: DateTime<Utc>,
    limit: i64,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Uuid>, DbError> {
    if limit <= 0 {
        return Ok(Vec::new());
    }
    let mut query = live_pairing_query(as_of);
    if let Some(after) = after_id {
        query = query.filter(tournaments::id.gt(after));
    }
    let mut candidates = query
        .order(tournaments::id.asc())
        .limit(limit)
        .select(tournaments::id)
        .load(conn)
        .await?;
    if let Some(after) = after_id {
        let returned =
            i64::try_from(candidates.len()).expect("database query page length fits i64");
        let remaining = limit.saturating_sub(returned);
        if remaining > 0 {
            candidates.extend(
                live_pairing_query(as_of)
                    .filter(tournaments::id.le(after))
                    .order(tournaments::id.asc())
                    .limit(remaining)
                    .select(tournaments::id)
                    .load::<Uuid>(conn)
                    .await?,
            );
        }
    }
    Ok(candidates)
}

pub async fn cutoff_candidates(
    as_of: DateTime<Utc>,
    limit: i64,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Uuid>, DbError> {
    if limit <= 0 {
        return Ok(Vec::new());
    }
    let cutoff = sql::<Timestamptz>(ARENA_END_SQL);
    Ok(tournaments::table
        .filter(sql::<Text>("configuration #>> '{format,format}'").eq("arena"))
        .filter(tournaments::starts_at.is_not_null())
        .filter(tournaments::started_at.is_not_null())
        .filter(tournaments::finished_at.is_null())
        .filter(cutoff.le(as_of))
        .order((tournaments::starts_at.asc(), tournaments::id.asc()))
        .limit(limit)
        .select(tournaments::id)
        .load(conn)
        .await?)
}

pub async fn opening_deadline_candidates(
    as_of: DateTime<Utc>,
    limit: i64,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Uuid>, DbError> {
    Game::find_due_arena_opening_ids(as_of, limit, conn).await
}
pub async fn ordinary_timeout_candidates(
    as_of: DateTime<Utc>,
    limit: i64,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Uuid>, DbError> {
    Game::find_due_arena_timeout_ids(as_of, limit, conn).await
}

/// Best-effort pre-finalization candidates strictly before the scoring cutoff.
pub async fn due_game_ids(
    tournament_id: Uuid,
    as_of: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Uuid>, DbError> {
    Ok(games::table
        .inner_join(tournaments::table.on(games::tournament_id.eq(tournaments::id.nullable())))
        .filter(games::tournament_id.eq(tournament_id))
        .filter(games::arena_ordinal.is_not_null())
        .filter(games::finished.eq(false))
        .filter(
            games::arena_move_due_at
                .lt(sql::<Timestamptz>(ARENA_END_SQL).nullable())
                .and(games::arena_move_due_at.le(as_of))
                .or(games::arena_move_due_at
                    .is_null()
                    .and(games::timeout_at.lt(sql::<Timestamptz>(ARENA_END_SQL).nullable()))
                    .and(games::timeout_at.le(as_of))),
        )
        .order(games::id.asc())
        .select(games::id)
        .load(conn)
        .await?)
}
