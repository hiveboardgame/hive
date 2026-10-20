mod common;

use chrono::{Duration, Utc};
use common::fixtures::{create_game, create_user, new_bullet_game, new_game};
use db_lib::{get_conn, models::Game};
use hive_lib::{GameResult, GameStatus};
use shared_types::{GameSpeed, TimeMode};

#[tokio::test(flavor = "multi_thread")]
async fn only_realtime_games_with_a_running_clock_count_as_live() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("test database connection");
    let white = create_user("live_white", false, &mut conn).await;
    let black = create_user("live_black", false, &mut conn).await;

    let mut older = new_bullet_game(white.id, black.id);
    older.created_at = Utc::now() - Duration::minutes(5);
    let older = create_game(older, &mut conn).await;
    let newer = create_game(new_bullet_game(black.id, white.id), &mut conn).await;

    let mut finished = new_bullet_game(white.id, black.id);
    finished.finished = true;
    finished.game_status = GameStatus::Finished(GameResult::Draw).to_string();
    create_game(finished, &mut conn).await;

    let mut unstarted = new_bullet_game(white.id, black.id);
    unstarted.game_status = GameStatus::NotStarted.to_string();
    unstarted.turn = 0;
    unstarted.history = String::new();
    create_game(unstarted, &mut conn).await;

    let mut correspondence = new_game(white.id, black.id, GameSpeed::Correspondence, false);
    correspondence.time_mode = TimeMode::Correspondence.to_string();
    create_game(correspondence, &mut conn).await;

    let live = Game::live_realtime(&mut conn)
        .await
        .expect("load live realtime games");
    let ids: Vec<_> = live.iter().map(|game| game.nanoid.clone()).collect();
    assert_eq!(ids, vec![newer.nanoid, older.nanoid], "newest first");
    assert_eq!(live[1].white_id, white.id);
    assert_eq!(live[1].black_id, black.id);
    assert_eq!(live[1].created_at, older.created_at);
}
