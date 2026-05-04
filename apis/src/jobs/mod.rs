pub mod challenge_cleanup;
pub mod game_cleanup;
pub mod heartbeat;
pub mod ping;
pub mod tournament_start;
pub use challenge_cleanup::run as challenge_cleanup;
pub use game_cleanup::run as game_cleanup;
pub use heartbeat::run as heartbeat;
pub use ping::run as ping;
pub use tournament_start::run as tournament_start;

use db_lib::DbConn;
use diesel::QueryableByName;
use diesel_async::RunQueryDsl;

// Unique per singleton job. Picked from a private range; no collisions with
// any other advisory_lock keys this app uses.
pub(crate) const TOURNAMENT_START_LOCK: i64 = 0x6869_7665_0000_0001;
pub(crate) const GAME_CLEANUP_LOCK: i64 = 0x6869_7665_0000_0002;
pub(crate) const CHALLENGE_CLEANUP_LOCK: i64 = 0x6869_7665_0000_0003;

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
    let r: AdvisoryLockResult =
        diesel::sql_query("SELECT pg_try_advisory_xact_lock($1) AS got")
            .bind::<diesel::sql_types::BigInt, _>(key)
            .get_result(conn)
            .await?;
    Ok(r.got)
}
