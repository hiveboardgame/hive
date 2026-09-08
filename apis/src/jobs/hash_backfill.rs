//! Fills in `games.hashes` and the `game_hashes` rows for finished games that have none. Runs on
//! every boot and stops at once when there is nothing to do.
//!
//! # Recomputing every hash
//!
//! Needed whenever the hash changes meaning. Stale hashes are worse than missing ones: the archive
//! filter and the opening explorer silently return nothing, and analysis refuses to open the game
//! ("inconsistent position hash").
//!
//! An empty `games.hashes` is what marks a game as outstanding, so clearing the column queues the
//! recompute.
//!
//! 1. Stop **every** instance - an old-version writer would store old-algorithm data after the
//!    clear, and a new-version one would have its fresh rows truncated by it.
//! 2. Run, once:
//!
//!    ```sql
//!    BEGIN;
//!    TRUNCATE game_hashes;
//!    UPDATE games SET hashes = '{}' WHERE history <> '';
//!    COMMIT;
//!    ```
//!
//!    All games with history, not only finished ones: a stale array breaks analysis on an
//!    in-flight game, and a timeout or resignation ends it without ever rewriting the column -
//!    leaving it non-empty, invisible to this job, and permanent.
//!
//!    `TRUNCATE` rather than the per-game delete: a game this job cannot replay would keep its
//!    wrong rows.
//! 3. Start the instances and watch `hash_backfill:` in the logs. A session-scoped advisory lock
//!    means only one runs the pass; the others wait briefly for it and then stand down.
//! 4. Verify, then scale back up:
//!
//!    ```sql
//!    SELECT count(*) FROM games WHERE finished AND history <> '' AND hashes = '{}';
//!    ```
//!
//!    A count above zero is not proof the pass failed, and it can climb after the pass ends:
//!    only a move rewrites the column, so a game cleared while live and then ended by timeout,
//!    resignation or an accepted draw joins this query when it finishes, and waits for the next
//!    boot. Outside a rehash nothing reaches that state - every move writes `hashes`, and a
//!    finish that plays no move has no new ply to record.
//!
//! Because "empty" means "not done", a crash or deploy mid-pass just resumes. Do not re-run the
//! SQL while a pass is active: the cursor only moves forward, so games re-cleared behind it wait
//! for the next boot.

use db_lib::{
    db_error::DbError,
    get_conn,
    models::{Game, GameFinishContext, GameHash},
    DbPool,
};
use diesel_async::AsyncConnection;
use hive_lib::State;
use std::time::Duration;

const BATCH_SIZE: i64 = 200;
const LOCK_RETRY_EVERY: Duration = Duration::from_secs(10);
const LOCK_WAIT_MAX: Duration = Duration::from_secs(300);

pub fn run(pool: DbPool) {
    actix_rt::spawn(async move {
        let Ok(conn) = get_conn(&pool).await else {
            log::error!("hash_backfill: failed to get connection");
            return;
        };

        // Held for the whole pass, which is why it is session- and not
        // transaction-scoped. Two instances overlap on every blue/green deploy
        // and each would replay the same games from the same cursor; during a
        // rehash they would also be running different hash algorithms, which is
        // what the module docs mean by "start one instance".
        //
        // Retried rather than skipped once: the holder is normally the outgoing
        // slot, which is stopped a few seconds later, possibly mid-pass. Giving
        // up on the first refusal would strand the remaining games until some
        // unrelated restart happened to pick them up.
        let mut lock_conn = conn;
        let mut waited = Duration::from_secs(0);
        loop {
            match crate::jobs::try_advisory_session_lock(
                &mut lock_conn,
                crate::jobs::HASH_BACKFILL_LOCK,
            )
            .await
            {
                Ok(true) => break,
                Ok(false) => {
                    if waited >= LOCK_WAIT_MAX {
                        log::warn!(
                            "hash_backfill: another instance has held the lock for \
                             {}s; giving up until the next boot",
                            waited.as_secs()
                        );
                        return;
                    }
                    if waited.is_zero() {
                        log::info!("hash_backfill: another instance holds the lock; waiting");
                    }
                    actix_rt::time::sleep(LOCK_RETRY_EVERY).await;
                    waited += LOCK_RETRY_EVERY;
                }
                Err(e) => {
                    log::error!("hash_backfill: could not take the lock: {e}");
                    return;
                }
            }
        }

        let remaining = match Game::count_needing_hash_backfill(&mut lock_conn).await {
            Ok(n) => n,
            Err(e) => {
                log::error!("hash_backfill: count failed: {e}");
                unlock(&mut lock_conn).await;
                return;
            }
        };

        if remaining == 0 {
            log::info!("hash_backfill: nothing to do");
            unlock(&mut lock_conn).await;
            return;
        }
        log::info!("hash_backfill: {remaining} games to process");

        let mut last_id = None;
        let mut total = 0u64;

        loop {
            let Ok(mut conn) = get_conn(&pool).await else {
                log::error!("hash_backfill: failed to get connection");
                break;
            };
            let batch = match Game::find_needing_hash_backfill(last_id, BATCH_SIZE, &mut conn).await
            {
                Ok(b) => b,
                Err(e) => {
                    log::error!("hash_backfill: query failed: {e}");
                    break;
                }
            };
            if batch.is_empty() {
                break;
            }

            for game in &batch {
                last_id = Some(game.id);

                let state = match State::new_from_str(&game.history, &game.game_type) {
                    Ok(s) => s,
                    Err(e) => {
                        log::warn!("hash_backfill: skip {} ({}): {e}", game.nanoid, game.id);
                        continue;
                    }
                };

                let game_id = game.id;
                let nanoid = game.nanoid.clone();
                let new_hashes: Vec<Option<i64>> =
                    state.hashes.iter().map(|h| Some(*h as i64)).collect();
                let raw_hashes = state.hashes.clone();
                let moves = state.history.moves.clone();
                let ctx = GameFinishContext::from_finished_game(game);

                let result = conn
                    .transaction(async |conn| {
                        Game::set_hashes(game_id, new_hashes, conn).await?;
                        GameHash::insert_for_game(game_id, &raw_hashes, &moves, &ctx, conn).await?;
                        Ok::<_, DbError>(())
                    })
                    .await;

                match result {
                    Ok(()) => total += 1,
                    Err(e) => log::warn!("hash_backfill: skip {} ({}): {e}", nanoid, game_id),
                }
            }
            log::info!("hash_backfill: {total}/{remaining}");
        }

        // The cursor only moves forward, so games skipped over a transient error are still
        // outstanding - "done" must not say otherwise to the operator watching the migration.
        match Game::count_needing_hash_backfill(&mut lock_conn).await {
            Ok(0) => log::info!("hash_backfill: done ({total} games processed)"),
            Ok(outstanding) => log::warn!(
                "hash_backfill: pass ended with {outstanding} games outstanding \
                 ({total} processed); they will be retried on the next boot"
            ),
            Err(e) => log::warn!("hash_backfill: final recount failed: {e}"),
        }
        unlock(&mut lock_conn).await;
    });
}

async fn unlock(conn: &mut db_lib::DbConn<'_>) {
    crate::jobs::advisory_session_unlock(conn, crate::jobs::HASH_BACKFILL_LOCK).await;
}
