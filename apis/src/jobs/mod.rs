pub mod challenge_cleanup;
pub mod email_cleanup;
pub mod email_drain;
pub mod game_cleanup;
pub mod hash_backfill;
pub mod heartbeat;
pub mod ping;
pub mod push_device_sweep;
pub mod timeout_sweeper;
pub mod tournament_cleanup;
pub mod tournament_start;
pub mod ws_telemetry;
pub use challenge_cleanup::run as challenge_cleanup;
pub use email_cleanup::run as email_cleanup;
pub use email_drain::run as email_drain;
pub use game_cleanup::run as game_cleanup;
pub use hash_backfill::run as hash_backfill;
pub use heartbeat::run as heartbeat;
pub use ping::run as ping;
pub use push_device_sweep::run as push_device_sweep;
pub use timeout_sweeper::run as timeout_sweeper;
pub use tournament_cleanup::run as tournament_cleanup;
pub use tournament_start::run as tournament_start;
pub use ws_telemetry::run as ws_telemetry;

use db_lib::DbConn;
use diesel::QueryableByName;
use diesel_async::RunQueryDsl;

// Unique per singleton job. Picked from a private range; no collisions with
// any other advisory_lock keys this app uses.
pub(crate) const TOURNAMENT_START_LOCK: i64 = 0x6869_7665_0000_0001;
pub(crate) const GAME_CLEANUP_LOCK: i64 = 0x6869_7665_0000_0002;
pub(crate) const CHALLENGE_CLEANUP_LOCK: i64 = 0x6869_7665_0000_0003;
pub(crate) const HASH_BACKFILL_LOCK: i64 = 0x6869_7665_0000_0004;

#[derive(QueryableByName)]
struct AdvisoryLockResult {
    #[diesel(sql_type = diesel::sql_types::Bool)]
    got: bool,
}

// Tries a transaction-scoped advisory lock so only one app instance runs
// the singleton job per tick during blue/green overlap. Auto-releases at
// commit/rollback.
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

// Session-scoped sibling of `try_advisory_xact_lock`, for a job that spans many
// transactions. The connection is pooled, so the session outlives the job and
// the lock is NOT released by dropping the guard — every exit path has to call
// `advisory_session_unlock`, or the next borrower of that connection hands a
// held lock to whatever runs next.
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
