mod common;

use common::fixtures;
use db_lib::{
    get_conn,
    models::{Rating, User},
    DbConn,
};
use shared_types::{GameSpeed, LeaderboardKind};
use uuid::Uuid;

const RANKABLE: f64 = 50.0;
const PROVISIONAL: f64 = 110.0;

async fn ranked_user(
    username: &str,
    bot: bool,
    rating: f64,
    deviation: f64,
    played: i64,
    conn: &mut DbConn<'_>,
) -> User {
    let user = fixtures::create_user(username, bot, conn).await;
    fixtures::set_rating(&user, &GameSpeed::Bullet, rating, deviation, played, conn).await;
    user
}

async fn top(
    kind: LeaderboardKind,
    viewer: Option<Uuid>,
    limit: i64,
    conn: &mut DbConn<'_>,
) -> Vec<(User, Rating, i64)> {
    User::get_top_users(kind, &GameSpeed::Bullet, viewer, limit, conn)
        .await
        .expect("load leaderboard")
}

fn ids(rows: &[(User, Rating, i64)]) -> Vec<Uuid> {
    rows.iter().map(|(user, _, _)| user.id).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn each_board_excludes_the_other_kind() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let human = ranked_user("human_sep", false, 1600.0, RANKABLE, 10, &mut conn).await;
    let bot = ranked_user("bot_sep", true, 1700.0, RANKABLE, 10, &mut conn).await;

    let humans = top(LeaderboardKind::Humans, None, 10, &mut conn).await;
    assert_eq!(ids(&humans), vec![human.id]);

    let bots = top(LeaderboardKind::Bots, None, 10, &mut conn).await;
    assert_eq!(ids(&bots), vec![bot.id]);
}

#[tokio::test(flavor = "multi_thread")]
async fn bots_without_games_are_not_listed() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    fixtures::create_user("bot_unplayed", true, &mut conn).await;

    let bots = top(LeaderboardKind::Bots, None, 10, &mut conn).await;
    assert!(
        bots.is_empty(),
        "a bot with only the rows User::create seeds must not rank at the default 1500"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn bots_are_listed_only_for_speeds_they_played() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let bot = ranked_user("bot_bullet_only", true, 1500.0, 300.0, 5, &mut conn).await;

    let bullet = top(LeaderboardKind::Bots, None, 10, &mut conn).await;
    assert_eq!(ids(&bullet), vec![bot.id]);

    let blitz = User::get_top_users(
        LeaderboardKind::Bots,
        &GameSpeed::Blitz,
        None,
        10,
        &mut conn,
    )
    .await
    .expect("load blitz leaderboard");
    assert!(blitz.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn ties_break_by_games_played_then_id() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let experienced = ranked_user("bot_tie_played", true, 1500.0, RANKABLE, 20, &mut conn).await;
    let first = ranked_user("bot_tie_one", true, 1500.0, RANKABLE, 5, &mut conn).await;
    let second = ranked_user("bot_tie_two", true, 1500.0, RANKABLE, 5, &mut conn).await;

    let mut fully_tied = vec![first.id, second.id];
    fully_tied.sort();

    let bots = top(LeaderboardKind::Bots, None, 10, &mut conn).await;
    assert_eq!(
        ids(&bots),
        vec![experienced.id, fully_tied[0], fully_tied[1]]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ordering_is_stable_across_identical_queries() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    for index in 0..6 {
        ranked_user(
            &format!("bot_stable_{index}"),
            true,
            1500.0,
            RANKABLE,
            5,
            &mut conn,
        )
        .await;
    }

    let first = top(LeaderboardKind::Bots, None, 4, &mut conn).await;
    let second = top(LeaderboardKind::Bots, None, 4, &mut conn).await;
    assert_eq!(ids(&first), ids(&second));
    assert_eq!(first.len(), 4);
}

#[tokio::test(flavor = "multi_thread")]
async fn tied_viewer_at_limit_boundary_gets_its_rank() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    for index in 0..9 {
        ranked_user(
            &format!("human_rank_{index}"),
            false,
            1600.0 - (index as f64 * 10.0),
            RANKABLE,
            10,
            &mut conn,
        )
        .await;
    }
    let tied_one = ranked_user("human_tied_one", false, 1500.0, RANKABLE, 10, &mut conn).await;
    let tied_two = ranked_user("human_tied_two", false, 1500.0, RANKABLE, 10, &mut conn).await;

    let excluded = if tied_one.id < tied_two.id {
        tied_two
    } else {
        tied_one
    };

    let rows = top(LeaderboardKind::Humans, Some(excluded.id), 10, &mut conn).await;
    assert_eq!(rows.len(), 11);

    let (viewer, _, rank) = rows.last().expect("viewer row appended");
    assert_eq!(viewer.id, excluded.id);
    assert_eq!(
        *rank, 10,
        "a viewer tied for tenth is ranked tenth, not unranked"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn bot_viewer_gets_its_rank_on_the_bots_board() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    for index in 0..10 {
        ranked_user(
            &format!("bot_ranked_{index}"),
            true,
            1600.0 - (index as f64 * 10.0),
            RANKABLE,
            10,
            &mut conn,
        )
        .await;
    }
    let outside = ranked_user("bot_outside", true, 1400.0, RANKABLE, 10, &mut conn).await;

    let rows = top(LeaderboardKind::Bots, Some(outside.id), 10, &mut conn).await;
    assert_eq!(rows.len(), 11);

    let (viewer, _, rank) = rows.last().expect("viewer row appended");
    assert_eq!(viewer.id, outside.id);
    assert_eq!(*rank, 11);
}

#[tokio::test(flavor = "multi_thread")]
async fn provisional_deviation_ranks_bots_but_not_humans() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let bot = ranked_user("bot_provisional", true, 1550.0, PROVISIONAL, 5, &mut conn).await;
    ranked_user(
        "human_provisional",
        false,
        1550.0,
        PROVISIONAL,
        5,
        &mut conn,
    )
    .await;

    let bots = top(LeaderboardKind::Bots, None, 10, &mut conn).await;
    assert_eq!(ids(&bots), vec![bot.id]);

    let humans = top(LeaderboardKind::Humans, None, 10, &mut conn).await;
    assert!(
        humans.is_empty(),
        "humans still need a rankable deviation to appear"
    );
}
