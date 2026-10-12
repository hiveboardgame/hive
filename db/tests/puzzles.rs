mod common;

use chrono::NaiveDate;
use common::fixtures::{create_user, rating_for};
use db_lib::{
    get_conn,
    models::{DailyPuzzle, Puzzle},
    DbConn,
};
use shared_types::{
    GameSpeed,
    PuzzleDifficulty,
    PuzzleGameTypeFilter,
    PuzzleHistoryFilter,
    PuzzleImport,
    PuzzleLength,
    PuzzleSelection,
};

fn fixture() -> PuzzleImport {
    serde_json::from_str(include_str!("../../shared_types/test_data/puzzle.json"))
        .expect("fixture puzzle")
}

async fn import(hash: i64, rating: f64, conn: &mut DbConn<'_>) -> Puzzle {
    let mut puzzle = fixture();
    puzzle.rating = Some(rating);
    Puzzle::import(&puzzle, hash, conn)
        .await
        .expect("import")
        .expect("new position")
}

#[tokio::test(flavor = "multi_thread")]
async fn a_position_is_only_stored_once() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let first = import(1, 1500.0, &mut conn).await;
    assert_eq!(first.authors(), vec!["philanthus".to_string()]);
    let again = Puzzle::import(&fixture(), 1, &mut conn)
        .await
        .expect("import");
    assert!(again.is_none());
    assert_eq!(Puzzle::visible_count(&mut conn).await.expect("count"), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_first_try_is_rated() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("solver", false, &mut conn).await;
    let puzzle = import(2, 1500.0, &mut conn).await;

    let first = Puzzle::record_attempt(puzzle.id, user.id, true, false, Some(4000), &mut conn)
        .await
        .expect("first try");
    assert!(first.rated);
    assert!(first.user_after > first.user_before);
    assert!(first.puzzle_after < first.puzzle_before);
    let rating = rating_for(&user, &GameSpeed::Puzzle, &mut conn).await;
    assert_eq!(Some(rating.rating), first.user_after);
    assert_eq!((rating.played, rating.won), (1, 1));

    let replay = Puzzle::record_attempt(puzzle.id, user.id, false, false, None, &mut conn)
        .await
        .expect("replay");
    assert!(!replay.rated);
    let unchanged = rating_for(&user, &GameSpeed::Puzzle, &mut conn).await;
    assert_eq!(unchanged.rating, rating.rating);
    let stored = Puzzle::find_by_nanoid(&puzzle.nanoid, &mut conn)
        .await
        .expect("puzzle");
    assert_eq!((stored.plays, stored.solves), (1, 1));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_hint_turns_a_solve_into_a_miss() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("hinted", false, &mut conn).await;
    let puzzle = import(3, 1500.0, &mut conn).await;
    let outcome = Puzzle::record_attempt(puzzle.id, user.id, true, true, None, &mut conn)
        .await
        .expect("try");
    assert!(outcome.user_after < outcome.user_before);
    let rows = Puzzle::history(user.id, PuzzleHistoryFilter::All, 0, &mut conn)
        .await
        .expect("history");
    assert_eq!(rows.len(), 1);
    assert!(rows[0].0.hinted);
}

#[tokio::test(flavor = "multi_thread")]
async fn stars_aggregate_and_can_change() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let first = create_user("rater_one", false, &mut conn).await;
    let second = create_user("rater_two", false, &mut conn).await;
    let puzzle = import(4, 1500.0, &mut conn).await;

    Puzzle::set_stars(puzzle.id, first.id, 5, &mut conn)
        .await
        .expect("stars");
    let updated = Puzzle::set_stars(puzzle.id, second.id, 2, &mut conn)
        .await
        .expect("stars");
    assert_eq!((updated.stars_sum, updated.stars_count), (7, 2));
    let changed = Puzzle::set_stars(puzzle.id, first.id, 3, &mut conn)
        .await
        .expect("change stars");
    assert_eq!((changed.stars_sum, changed.stars_count), (5, 2));
    assert_eq!(
        Puzzle::stars_of(puzzle.id, first.id, &mut conn)
            .await
            .expect("stars of"),
        Some(3)
    );
    assert!(Puzzle::set_stars(puzzle.id, first.id, 6, &mut conn)
        .await
        .is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_widely_disliked_puzzle_is_hidden() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let puzzle = import(5, 1500.0, &mut conn).await;
    let mut last = None;
    for index in 0..shared_types::AUTO_HIDE_MIN_VOTES {
        let user = create_user(&format!("critic{index}"), false, &mut conn).await;
        last = Some(
            Puzzle::set_stars(puzzle.id, user.id, 1, &mut conn)
                .await
                .expect("stars"),
        );
    }
    assert!(last.expect("votes").hidden);
    assert_eq!(Puzzle::visible_count(&mut conn).await.expect("count"), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn next_skips_what_the_player_has_seen() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("browser", false, &mut conn).await;
    let near = import(6, 1500.0, &mut conn).await;
    let far = import(7, 2400.0, &mut conn).await;

    let first = Puzzle::next(
        1500.0,
        Some(user.id),
        &PuzzleSelection::default(),
        &[],
        &mut conn,
    )
    .await
    .expect("next")
    .expect("a puzzle");
    assert_eq!(first.id, near.id, "the closest rating comes first");

    Puzzle::record_attempt(near.id, user.id, true, false, None, &mut conn)
        .await
        .expect("try");
    let second = Puzzle::next(
        1500.0,
        Some(user.id),
        &PuzzleSelection::default(),
        &[],
        &mut conn,
    )
    .await
    .expect("next")
    .expect("the window widens");
    assert_eq!(second.id, far.id);

    let base_only = Puzzle::next(
        1500.0,
        None,
        &PuzzleSelection {
            game_type: PuzzleGameTypeFilter::Base,
            ..PuzzleSelection::default()
        },
        &[],
        &mut conn,
    )
    .await
    .expect("next");
    assert!(base_only.is_none(), "the fixture is an MLP puzzle");
}

async fn import_with(
    hash: i64,
    rating: f64,
    themes: &[&str],
    collection: Option<&str>,
    conn: &mut DbConn<'_>,
) -> Puzzle {
    let mut puzzle = fixture();
    puzzle.rating = Some(rating);
    puzzle.themes = themes.iter().map(|theme| theme.to_string()).collect();
    puzzle.collection = collection.map(str::to_string);
    Puzzle::import(&puzzle, hash, conn)
        .await
        .expect("import")
        .expect("new position")
}

#[tokio::test(flavor = "multi_thread")]
async fn selections_shift_the_rating_and_narrow_the_pool() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let near = import_with(20, 1500.0, &["winIn2"], None, &mut conn).await;
    let far = import_with(21, 1800.0, &["throw"], Some("vol1"), &mut conn).await;
    let next = async |selection: PuzzleSelection, conn: &mut DbConn<'_>| {
        Puzzle::next(1500.0, None, &selection, &[], conn)
            .await
            .expect("next")
            .map(|puzzle| puzzle.id)
    };

    let normal = next(PuzzleSelection::default(), &mut conn).await;
    assert_eq!(normal, Some(near.id));
    let harder = PuzzleSelection {
        difficulty: PuzzleDifficulty::Harder,
        ..PuzzleSelection::default()
    };
    assert_eq!(next(harder, &mut conn).await, Some(far.id));
    let win_in_one = PuzzleSelection {
        length: PuzzleLength::One,
        ..PuzzleSelection::default()
    };
    assert_eq!(
        next(win_in_one, &mut conn).await,
        None,
        "the fixture is a win in 2"
    );
    let throws = PuzzleSelection {
        theme: Some("throw".to_string()),
        ..PuzzleSelection::default()
    };
    assert_eq!(next(throws, &mut conn).await, Some(far.id));
    let book = PuzzleSelection {
        collection: Some("vol1".to_string()),
        ..PuzzleSelection::default()
    };
    assert_eq!(next(book, &mut conn).await, Some(far.id));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_counts_cover_themes_lengths_and_collections() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("counter", false, &mut conn).await;
    let near = import_with(30, 1500.0, &["winIn2"], None, &mut conn).await;
    import_with(31, 1800.0, &["throw"], Some("vol1"), &mut conn).await;
    Puzzle::record_attempt(near.id, user.id, true, false, None, &mut conn)
        .await
        .expect("try");

    let counts = Puzzle::set_counts(Some(user.id), &mut conn)
        .await
        .expect("counts");
    let find = |key: &str| {
        counts
            .iter()
            .find(|count| count.key == key)
            .map(|count| (count.total, count.solved_count))
    };
    assert_eq!(find("moves:2"), Some((2, 1)));
    assert_eq!(find("throw"), Some((1, 0)));
    assert_eq!(find("collection:vol1"), Some((1, 0)));
    assert_eq!(find("winIn2"), Some((1, 1)));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_daily_puzzle_takes_the_queue_first_and_never_repeats() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let admin = create_user("curator", false, &mut conn).await;
    import(30, 1200.0, &mut conn).await;
    import(31, 1250.0, &mut conn).await;
    let queued = import(32, 1900.0, &mut conn).await;
    DailyPuzzle::enqueue(&queued.nanoid, admin.id, &mut conn)
        .await
        .expect("queue");

    let day = |d| NaiveDate::from_ymd_opt(2026, 10, d).expect("date");
    let first = DailyPuzzle::for_day(day(12), &mut conn)
        .await
        .expect("pick")
        .expect("a puzzle");
    assert_eq!(first.1.id, queued.id);
    let same = DailyPuzzle::for_day(day(12), &mut conn)
        .await
        .expect("pick")
        .expect("a puzzle");
    assert_eq!(same.1.id, queued.id);
    assert!(DailyPuzzle::queued(&mut conn)
        .await
        .expect("queue")
        .is_empty());

    let second = DailyPuzzle::for_day(day(13), &mut conn)
        .await
        .expect("pick")
        .expect("a puzzle");
    let third = DailyPuzzle::for_day(day(14), &mut conn)
        .await
        .expect("pick")
        .expect("a puzzle");
    assert_ne!(second.1.id, queued.id);
    assert_ne!(third.1.id, second.1.id);
    assert_ne!(third.1.id, queued.id);
    assert!(DailyPuzzle::for_day(day(15), &mut conn)
        .await
        .expect("pick")
        .is_none());

    assert!(DailyPuzzle::enqueue(&queued.nanoid, admin.id, &mut conn)
        .await
        .is_err());
    assert_eq!(
        DailyPuzzle::day_of(queued.id, &mut conn)
            .await
            .expect("day"),
        Some(day(12))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn one_try_never_moves_a_rating_by_more_than_the_step() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("newcomer", false, &mut conn).await;
    let puzzle = import(40, 900.0, &mut conn).await;
    let outcome = Puzzle::record_attempt(puzzle.id, user.id, false, false, None, &mut conn)
        .await
        .expect("try");
    let (before, after) = (
        outcome.user_before.expect("rated"),
        outcome.user_after.expect("rated"),
    );
    assert!(after < before);
    assert!(before - after <= 120.0 + 1e-9);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_daily_tally_counts_first_tries_only() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let solver = create_user("early_bird", false, &mut conn).await;
    let misser = create_user("late_riser", false, &mut conn).await;
    let puzzle = import(41, 1500.0, &mut conn).await;
    let today = chrono::Utc::now().date_naive();
    for (user, first, replay) in [(&solver, true, false), (&misser, false, true)] {
        Puzzle::record_attempt(puzzle.id, user.id, first, false, None, &mut conn)
            .await
            .expect("first try");
        Puzzle::record_attempt(puzzle.id, user.id, replay, false, None, &mut conn)
            .await
            .expect("replay");
    }
    let tally = DailyPuzzle::tally(puzzle.id, today, &mut conn)
        .await
        .expect("tally");
    assert_eq!((tally.tried, tally.solved), (2, 1));
    assert_eq!(
        DailyPuzzle::first_result(puzzle.id, misser.id, &mut conn)
            .await
            .expect("result"),
        Some(false)
    );
}
