use crate::{db_error::DbError, models::Game, schema::tournaments, DbConn};
use chrono::{DateTime, Duration, Utc};
use diesel::{
    dsl::sql,
    prelude::*,
    sql_types::{Text, Timestamptz},
};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

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
    let mut query = tournaments::table
        .into_boxed()
        .filter(sql::<Text>("configuration #>> '{format,format}'").eq("arena"))
        .filter(tournaments::starts_at.le(as_of))
        .filter(tournaments::started_at.is_not_null())
        .filter(tournaments::finished_at.is_null())
        .filter(
            sql::<Timestamptz>(
                "tournaments.starts_at + make_interval(secs => ((tournaments.configuration #>> '{format,configuration,duration_seconds}')::integer))",
            )
            .gt(as_of + Duration::seconds(60)),
        );
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
                tournaments::table
                    .filter(sql::<Text>("configuration #>> '{format,format}'").eq("arena"))
                    .filter(tournaments::starts_at.le(as_of))
                    .filter(tournaments::started_at.is_not_null())
                    .filter(tournaments::finished_at.is_null())
                    .filter(
                        sql::<Timestamptz>(
                            "tournaments.starts_at + make_interval(secs => ((tournaments.configuration #>> '{format,configuration,duration_seconds}')::integer))",
                        )
                        .gt(as_of + Duration::seconds(60)),
                    )
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
    let cutoff = sql::<Timestamptz>(
        "tournaments.starts_at + make_interval(secs => ((tournaments.configuration #>> '{format,configuration,duration_seconds}')::integer))",
    );
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
