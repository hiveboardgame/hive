use db_lib::{
    db_error::DbError,
    get_conn,
    models::{Game, GameFinishContext, GameHash},
    DbPool,
};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection};
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
                    .transaction(|conn| {
                        async move {
                            Game::set_hashes(game_id, new_hashes, conn).await?;
                            GameHash::insert_for_game(game_id, &raw_hashes, &moves, &ctx, conn)
                                .await?;
                            Ok::<_, DbError>(())
                        }
                        .scope_boxed()
                    })
                    .await;

                match result {
                    Ok(()) => total += 1,
                    Err(e) => log::warn!("hash_backfill: skip {} ({}): {e}", nanoid, game_id),
                }
            }
            log::info!("hash_backfill: {total}/{remaining}");
        }

        log::info!("hash_backfill: done ({total} games processed)");
    });
}
