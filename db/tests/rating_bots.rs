mod common;

use common::fixtures;
use db_lib::{get_conn, models::Rating};
use shared_types::{Conclusion, GameSpeed};

fn assert_counters(before: &Rating, after: &Rating, won: i64, lost: i64, drawn: i64) {
    assert_eq!(after.played, before.played + 1);
    assert_eq!(after.won, before.won + won);
    assert_eq!(after.lost, before.lost + lost);
    assert_eq!(after.draw, before.draw + drawn);
}

#[tokio::test(flavor = "multi_thread")]
async fn games_against_bots_rate_both_players() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let human = fixtures::create_user("human_one", false, &mut conn).await;
    let bot = fixtures::create_user("bot_one", true, &mut conn).await;

    let human_before = fixtures::bullet_rating(&human, &mut conn).await;
    let bot_before = fixtures::bullet_rating(&bot, &mut conn).await;

    let game = fixtures::create_bullet_game(human.id, bot.id, &mut conn).await;
    let finished = fixtures::resign_as_white(game, &db.pool)
        .await
        .expect("finish game");

    let human_after = fixtures::bullet_rating(&human, &mut conn).await;
    let bot_after = fixtures::bullet_rating(&bot, &mut conn).await;

    assert_eq!(finished.conclusion, Conclusion::Resigned.to_string());
    assert!(human_after.rating < human_before.rating);
    assert!(bot_after.rating > bot_before.rating);

    assert_counters(&human_before, &human_after, 0, 1, 0);
    assert_counters(&bot_before, &bot_after, 1, 0, 0);

    assert!(finished.white_rating_change.is_some());
    assert!(finished.black_rating_change.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn bot_versus_bot_moves_both_ratings() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let white = fixtures::create_user("bot_white", true, &mut conn).await;
    let black = fixtures::create_user("bot_black", true, &mut conn).await;

    let white_before = fixtures::bullet_rating(&white, &mut conn).await;
    let black_before = fixtures::bullet_rating(&black, &mut conn).await;

    let game = fixtures::create_bullet_game(white.id, black.id, &mut conn).await;
    let finished = fixtures::resign_as_white(game, &db.pool)
        .await
        .expect("finish game");

    let white_after = fixtures::bullet_rating(&white, &mut conn).await;
    let black_after = fixtures::bullet_rating(&black, &mut conn).await;

    assert_eq!(finished.conclusion, Conclusion::Resigned.to_string());
    assert!(white_after.rating < white_before.rating);
    assert!(black_after.rating > black_before.rating);

    assert_counters(&white_before, &white_after, 0, 1, 0);
    assert_counters(&black_before, &black_after, 1, 0, 0);

    assert!(finished.white_rating_change.is_some());
    assert!(finished.black_rating_change.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn rated_draw_credits_a_draw_to_both_sides() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let white = fixtures::create_user("human_draw_white", false, &mut conn).await;
    let black = fixtures::create_user("bot_draw_black", true, &mut conn).await;

    let white_before = fixtures::bullet_rating(&white, &mut conn).await;
    let black_before = fixtures::bullet_rating(&black, &mut conn).await;

    let game = fixtures::create_bullet_game(white.id, black.id, &mut conn).await;
    let finished = fixtures::draw_game(game, &db.pool)
        .await
        .expect("draw game");

    let white_after = fixtures::bullet_rating(&white, &mut conn).await;
    let black_after = fixtures::bullet_rating(&black, &mut conn).await;

    assert_eq!(finished.conclusion, Conclusion::Draw.to_string());
    assert_counters(&white_before, &white_after, 0, 0, 1);
    assert_counters(&black_before, &black_after, 0, 0, 1);

    assert!(finished.white_rating_change.is_some());
    assert!(finished.black_rating_change.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn unrated_decisive_game_moves_counters_but_not_ratings() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let human = fixtures::create_user("human_unrated", false, &mut conn).await;
    let bot = fixtures::create_user("bot_unrated", true, &mut conn).await;

    let human_before = fixtures::bullet_rating(&human, &mut conn).await;
    let bot_before = fixtures::bullet_rating(&bot, &mut conn).await;

    let new_game = fixtures::new_game(human.id, bot.id, GameSpeed::Bullet, false);
    let game = fixtures::create_game(new_game, &mut conn).await;
    let finished = fixtures::resign_as_white(game, &db.pool)
        .await
        .expect("finish game");

    let human_after = fixtures::bullet_rating(&human, &mut conn).await;
    let bot_after = fixtures::bullet_rating(&bot, &mut conn).await;

    assert_eq!(finished.conclusion, Conclusion::Resigned.to_string());
    assert_eq!(human_after.rating, human_before.rating);
    assert_eq!(human_after.deviation, human_before.deviation);
    assert_eq!(human_after.volatility, human_before.volatility);
    assert_eq!(bot_after.rating, bot_before.rating);

    assert_counters(&human_before, &human_after, 0, 1, 0);
    assert_counters(&bot_before, &bot_after, 1, 0, 0);

    assert_eq!(finished.white_rating_change, None);
    assert_eq!(finished.black_rating_change, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn unrated_draw_moves_only_the_draw_counter() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let white = fixtures::create_user("human_unrated_draw", false, &mut conn).await;
    let black = fixtures::create_user("bot_unrated_draw", true, &mut conn).await;

    let white_before = fixtures::bullet_rating(&white, &mut conn).await;
    let black_before = fixtures::bullet_rating(&black, &mut conn).await;

    let new_game = fixtures::new_game(white.id, black.id, GameSpeed::Bullet, false);
    let game = fixtures::create_game(new_game, &mut conn).await;
    let finished = fixtures::draw_game(game, &db.pool)
        .await
        .expect("draw game");

    let white_after = fixtures::bullet_rating(&white, &mut conn).await;
    let black_after = fixtures::bullet_rating(&black, &mut conn).await;

    assert_eq!(finished.conclusion, Conclusion::Draw.to_string());
    assert_eq!(white_after.rating, white_before.rating);
    assert_eq!(black_after.rating, black_before.rating);

    assert_counters(&white_before, &white_after, 0, 0, 1);
    assert_counters(&black_before, &black_after, 0, 0, 1);

    assert_eq!(finished.white_rating_change, None);
    assert_eq!(finished.black_rating_change, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn updated_at_tracks_rating_movement_not_row_activity() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let human = fixtures::create_user("human_touched", false, &mut conn).await;
    let bot = fixtures::create_user("bot_touched", true, &mut conn).await;

    let human_before = fixtures::bullet_rating(&human, &mut conn).await;
    let bot_before = fixtures::bullet_rating(&bot, &mut conn).await;

    let unrated = fixtures::new_game(human.id, bot.id, GameSpeed::Bullet, false);
    let game = fixtures::create_game(unrated, &mut conn).await;
    fixtures::resign_as_white(game, &db.pool)
        .await
        .expect("finish unrated game");

    let human_unrated = fixtures::bullet_rating(&human, &mut conn).await;
    let bot_unrated = fixtures::bullet_rating(&bot, &mut conn).await;
    assert_eq!(human_unrated.updated_at, human_before.updated_at);
    assert_eq!(bot_unrated.updated_at, bot_before.updated_at);

    let game = fixtures::create_bullet_game(human.id, bot.id, &mut conn).await;
    fixtures::resign_as_white(game, &db.pool)
        .await
        .expect("finish rated game");

    let human_rated = fixtures::bullet_rating(&human, &mut conn).await;
    let bot_rated = fixtures::bullet_rating(&bot, &mut conn).await;
    assert!(human_rated.updated_at > human_unrated.updated_at);
    assert!(bot_rated.updated_at > bot_unrated.updated_at);
}
