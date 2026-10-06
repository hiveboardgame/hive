use db_lib::DbConn;
use diesel::QueryableByName;
use diesel_async::RunQueryDsl;

pub(crate) const TOURNAMENT_START_LOCK: i64 = 0x6869_7665_0000_0001;
pub(crate) const GAME_CLEANUP_LOCK: i64 = 0x6869_7665_0000_0002;
pub(crate) const CHALLENGE_CLEANUP_LOCK: i64 = 0x6869_7665_0000_0003;
pub(crate) const HASH_BACKFILL_LOCK: i64 = 0x6869_7665_0000_0004;

#[derive(QueryableByName)]
struct AdvisoryLockResult {
    #[diesel(sql_type = diesel::sql_types::Bool)]
    got: bool,
}

pub(crate) async fn try_advisory_xact_lock(
    conn: &mut DbConn<'_>,
    key: i64,
) -> Result<bool, diesel::result::Error> {
    let r: AdvisoryLockResult = diesel::sql_query("SELECT pg_try_advisory_xact_lock($1) AS got")
        .bind::<diesel::sql_types::BigInt, _>(key)
        .get_result(conn)
        .await?;
    Ok(r.got)
}

// Session-scoped on a pooled connection: dropping the connection does not release it.
pub(crate) async fn try_advisory_session_lock(
    conn: &mut DbConn<'_>,
    key: i64,
) -> Result<bool, diesel::result::Error> {
    let r: AdvisoryLockResult = diesel::sql_query("SELECT pg_try_advisory_lock($1) AS got")
        .bind::<diesel::sql_types::BigInt, _>(key)
        .get_result(conn)
        .await?;
    Ok(r.got)
}

pub(crate) async fn advisory_session_unlock(conn: &mut DbConn<'_>, key: i64) {
    if let Err(e) = diesel::sql_query("SELECT pg_advisory_unlock($1) AS got")
        .bind::<diesel::sql_types::BigInt, _>(key)
        .execute(conn)
        .await
    {
        log::error!("failed to release advisory lock {key:#x}: {e}");
    }
}
