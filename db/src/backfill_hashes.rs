use db_lib::{
    config::DbConfig,
    get_conn, get_pool,
    models::{Game, GameFinishContext, GameHash},
};

use hive_lib::State;
use uuid::Uuid;

const BATCH_SIZE: i64 = 200;

#[tokio::main]
async fn main() {
    let config = DbConfig::from_env().expect("Failed to load config from env");
    let pool = get_pool(&config.database_url)
        .await
        .expect("Failed to get pool");

    let mut conn = get_conn(&pool).await.expect("Failed to get connection");
    let remaining = Game::count_needing_hash_backfill(&mut conn)
        .await
        .expect("Failed to count games");
    drop(conn);

    if remaining == 0 {
        println!("No games need backfilling.");
        return;
    }
    println!("{remaining} games to backfill.");

    let mut last_id: Option<Uuid> = None;
    let mut total = 0_u64;

    loop {
        let mut conn = get_conn(&pool).await.expect("Failed to get connection");
        let batch = Game::find_needing_hash_backfill(last_id, BATCH_SIZE, &mut conn)
            .await
            .expect("Failed to query games");

        if batch.is_empty() {
            break;
        }

        for game in &batch {
            last_id = Some(game.id);

            let state = match State::new_from_str(&game.history, &game.game_type) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!(
                        "Skipping game {} (id {}): failed to replay history: {e}",
                        game.nanoid, game.id
                    );
                    continue;
                }
            };

            let new_hashes: Vec<Option<i64>> =
                state.hashes.iter().map(|h| Some(*h as i64)).collect();

            if let Err(e) = Game::set_hashes(game.id, new_hashes, &mut conn).await {
                eprintln!(
                    "Skipping game {} (id {}): failed to update hashes: {e}",
                    game.nanoid, game.id
                );
                continue;
            }

            let ctx = GameFinishContext::from_finished_game(game);
            if let Err(e) = GameHash::insert_for_game(game.id, &state.hashes, &ctx, &mut conn).await {
                eprintln!(
                    "Warning: game {} (id {}): failed to insert game_hashes: {e}",
                    game.nanoid, game.id
                );
            }

            total += 1;
        }

        println!("Backfilled {total}/{remaining} games...");
    }

    println!("Done. Backfilled {total}/{remaining} games.");
}
