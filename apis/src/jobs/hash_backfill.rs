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
//! 3. Start **one** instance and watch `hash_backfill:` in the logs; every booting instance runs
//!    its own unlocked copy, so more only multiply the replay load.
//! 4. Verify, then scale back up:
//!
//!    ```sql
//!    SELECT count(*) FROM games WHERE finished AND history <> '' AND hashes = '{}';
//!    ```
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

const BATCH_SIZE: i64 = 200;

pub fn run(pool: DbPool) {
    actix_rt::spawn(async move {
        let Ok(mut conn) = get_conn(&pool).await else {
            log::error!("hash_backfill: failed to get connection");
            return;
        };
        let remaining = match Game::count_needing_hash_backfill(&mut conn).await {
            Ok(n) => n,
            Err(e) => {
                log::error!("hash_backfill: count failed: {e}");
                return;
            }
        };
        drop(conn);

        if remaining == 0 {
            log::info!("hash_backfill: nothing to do");
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
        match get_conn(&pool).await {
            Ok(mut conn) => match Game::count_needing_hash_backfill(&mut conn).await {
                Ok(0) => log::info!("hash_backfill: done ({total} games processed)"),
                Ok(outstanding) => log::warn!(
                    "hash_backfill: pass ended with {outstanding} games outstanding \
                     ({total} processed); they will be retried on the next boot"
                ),
                Err(e) => log::warn!("hash_backfill: final recount failed: {e}"),
            },
            Err(_) => log::warn!("hash_backfill: no connection for the final recount"),
        }
    });
}
