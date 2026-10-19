mod common;

use chrono::Duration;
use common::{
    fixtures::{create_game, create_user, new_game},
    tournament::{create_rr_tournament, insert_dense_memberships},
};
use db_lib::{
    db_error::DbError,
    get_conn,
    models::{AutoEvals, ClaimedEval, GameEval, NewGame},
    DbConn,
};
use shared_types::{EvalResult, EvalStatus, GameSpeed, MoveEval, TournamentStatus};
use uuid::Uuid;

/// Enough for the tests that only queue and claim: a running auto-eval is only preempted when
/// every worker is busy.
const ONE_WORKER: usize = 1;

async fn game(conn: &mut DbConn<'_>, white: &str, black: &str) -> Uuid {
    game_of(conn, white, black, 20).await
}

async fn game_of(conn: &mut DbConn<'_>, white: &str, black: &str, turn: i32) -> Uuid {
    let w = create_user(white, false, conn).await;
    let b = create_user(black, false, conn).await;
    let mut game = new_game(w.id, b.id, GameSpeed::Rapid, true);
    game.turn = turn;
    // Strong enough for the front page.
    game.white_rating = Some(2100.0);
    game.black_rating = Some(2100.0);
    create_game(game, conn).await.id
}

fn result_for(moves: &[&str]) -> EvalResult {
    EvalResult {
        moves: moves
            .iter()
            .map(|m| {
                Some(MoveEval {
                    played: m.to_string(),
                    best: m.to_string(),
                    line: vec![m.to_string()],
                    before: 50.0,
                    after: 50.0,
                    sims: 800,
                })
            })
            .collect(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn requesting_a_game_twice_returns_the_same_eval() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g = game(&mut conn, "w1", "b1").await;
    let asker = create_user("asker", false, &mut conn).await;
    let other = create_user("other", false, &mut conn).await;

    let first = GameEval::request(g, asker.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    let second = GameEval::request(g, other.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(second.requested_by, Some(asker.id));
    assert_eq!(second.status(), EvalStatus::Queued);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_user_has_one_active_eval_at_a_time() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g1 = game(&mut conn, "w1", "b1").await;
    let g2 = game(&mut conn, "w2", "b2").await;
    let asker = create_user("asker", false, &mut conn).await;

    let first = GameEval::request(g1, asker.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    let err = GameEval::request(g2, asker.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap_err();
    assert!(matches!(err, DbError::InvalidAction { .. }), "{err:?}");

    let claimed = GameEval::claim("worker-a", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .eval;
    assert_eq!(claimed.id, first.id);
    assert!(GameEval::finish(
        first.id,
        "worker-a",
        "test",
        &result_for(&["wQ", "bQ /wQ"]),
        &mut conn
    )
    .await
    .unwrap());

    let next = GameEval::request(g2, asker.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    assert_eq!(next.status(), EvalStatus::Queued);
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_requester_can_take_back_a_waiting_eval() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g = game(&mut conn, "w1", "b1").await;
    let asker = create_user("asker", false, &mut conn).await;
    let other = create_user("other", false, &mut conn).await;
    GameEval::request(g, asker.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();

    assert!(!GameEval::cancel(g, other.id, &mut conn).await.unwrap());
    assert!(GameEval::cancel(g, asker.id, &mut conn).await.unwrap());
    assert!(GameEval::find_by_game(g, &mut conn)
        .await
        .unwrap()
        .is_none());

    // Once a worker has it, it is no longer the requester's to take back.
    GameEval::request(g, asker.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    GameEval::claim("worker-a", AutoEvals::Off, &mut conn)
        .await
        .unwrap()
        .unwrap();
    assert!(!GameEval::cancel(g, asker.id, &mut conn).await.unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_front_page_leaves_out_games_too_short_to_be_evaluated_today() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    // Evaluated before short games were refused; it must not be advertised.
    let short = game_of(&mut conn, "w1", "b1", 2).await;
    let asker = create_user("asker", false, &mut conn).await;
    let eval = GameEval::request(short, asker.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    GameEval::claim("worker-a", AutoEvals::Off, &mut conn)
        .await
        .unwrap()
        .unwrap();
    assert!(GameEval::finish(
        eval.id,
        "worker-a",
        "test",
        &result_for(&["wQ", "bQ /wQ"]),
        &mut conn
    )
    .await
    .unwrap());

    assert!(GameEval::recent(6, &mut conn).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_front_page_leaves_out_weaker_games_except_tournament_ones() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let w = create_user("w1", false, &mut conn).await;
    let b = create_user("b1", false, &mut conn).await;
    let mut weak = new_game(w.id, b.id, GameSpeed::Rapid, true);
    weak.turn = 20;
    weak.white_rating = Some(2300.0);
    weak.black_rating = Some(1500.0);
    let weak = create_game(weak, &mut conn).await.id;
    let asker = create_user("asker", false, &mut conn).await;
    let eval = GameEval::request(weak, asker.id, 20, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    GameEval::claim("worker-a", AutoEvals::Off, &mut conn)
        .await
        .unwrap()
        .unwrap();
    assert!(GameEval::finish(
        eval.id,
        "worker-a",
        "test",
        &result_for(&["wQ", "bQ /wQ"]),
        &mut conn
    )
    .await
    .unwrap());

    assert!(GameEval::recent(6, &mut conn).await.unwrap().is_empty());

    // A tournament game shows whatever the ratings.
    let tw = create_user("tw", false, &mut conn).await;
    let tb = create_user("tb", false, &mut conn).await;
    let mut cup = new_game(tw.id, tb.id, GameSpeed::Rapid, true);
    cup.turn = 20;
    cup.white_rating = Some(1400.0);
    cup.black_rating = Some(1300.0);
    enter_tournament(&mut cup, "world", &mut conn).await;
    let cup = create_game(cup, &mut conn).await.id;
    let eval = GameEval::request(cup, asker.id, 20, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    GameEval::claim("worker-a", AutoEvals::Off, &mut conn)
        .await
        .unwrap()
        .unwrap();
    assert!(GameEval::finish(
        eval.id,
        "worker-a",
        "test",
        &result_for(&["wQ", "bQ /wQ"]),
        &mut conn
    )
    .await
    .unwrap());
    let shown: Vec<String> = GameEval::recent(6, &mut conn)
        .await
        .unwrap()
        .into_iter()
        .map(|eval| eval.white)
        .collect();
    assert_eq!(shown, vec!["tw"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn claims_go_out_oldest_first_and_only_once() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g1 = game(&mut conn, "w1", "b1").await;
    let g2 = game(&mut conn, "w2", "b2").await;
    let a = create_user("alice", false, &mut conn).await;
    let b = create_user("bobby", false, &mut conn).await;
    let first = GameEval::request(g1, a.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    let second = GameEval::request(g2, b.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();

    let c1 = GameEval::claim("worker-a", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .eval;
    let c2 = GameEval::claim("worker-b", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .eval;
    let none = GameEval::claim("worker-c", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap();

    assert_eq!((c1.id, c2.id), (first.id, second.id));
    assert!(none.is_none());
    assert_eq!(c1.status(), EvalStatus::Running);
    assert_eq!(c1.attempts, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_claiming_worker_reports_progress_or_hands_back() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g = game(&mut conn, "w1", "b1").await;
    let a = create_user("alice", false, &mut conn).await;
    let eval = GameEval::request(g, a.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    let claimed = GameEval::claim("worker-a", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claimed.game.id, g, "the claim brings its game along");

    assert!(
        !GameEval::report_progress(eval.id, "worker-b", 50, &mut conn)
            .await
            .unwrap()
    );
    assert!(
        !GameEval::give_back(eval.id, "worker-b", "engine crashed", 3, &mut conn)
            .await
            .unwrap()
    );
    assert!(
        GameEval::report_progress(eval.id, "worker-a", 50, &mut conn)
            .await
            .unwrap()
    );

    let stored = GameEval::find_by_game(g, &mut conn).await.unwrap().unwrap();
    assert_eq!(stored.progress_pct, 50);
    assert_eq!(stored.status(), EvalStatus::Running);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_result_that_arrives_after_its_eval_moved_on_still_counts() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g = game(&mut conn, "w1", "b1").await;
    let a = create_user("alice", false, &mut conn).await;
    let eval = GameEval::request(g, a.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    GameEval::claim("worker-a", AutoEvals::Off, &mut conn)
        .await
        .unwrap();
    GameEval::sweep_stale(Duration::seconds(-1), 3, &mut conn)
        .await
        .unwrap();
    GameEval::claim("worker-b", AutoEvals::Off, &mut conn)
        .await
        .unwrap();
    let result = result_for(&["wQ", "bQ /wQ"]);

    assert!(
        GameEval::finish(eval.id, "worker-a", "slow", &result, &mut conn)
            .await
            .unwrap()
    );
    assert!(
        !GameEval::report_progress(eval.id, "worker-b", 10, &mut conn)
            .await
            .unwrap(),
        "the second worker stops"
    );
    assert!(
        !GameEval::finish(eval.id, "worker-b", "fast", &result, &mut conn)
            .await
            .unwrap(),
        "a done eval keeps its first result"
    );
    let stored = GameEval::find_by_game(g, &mut conn).await.unwrap().unwrap();
    assert_eq!(
        (
            stored.status(),
            stored.worker.as_deref(),
            stored.engine.as_deref()
        ),
        (EvalStatus::Done, Some("worker-a"), Some("slow"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_handed_back_eval_waits_again_until_its_attempts_run_out() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g = game(&mut conn, "w1", "b1").await;
    let a = create_user("alice", false, &mut conn).await;
    let eval = GameEval::request(g, a.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();

    for attempt in 1..=2 {
        GameEval::claim("worker-a", AutoEvals::Off, &mut conn)
            .await
            .unwrap()
            .unwrap();
        assert!(
            GameEval::give_back(eval.id, "worker-a", "engine crashed", 2, &mut conn)
                .await
                .unwrap()
        );
        let stored = GameEval::find_by_game(g, &mut conn).await.unwrap().unwrap();
        let expected = if attempt < 2 {
            EvalStatus::Queued
        } else {
            EvalStatus::Failed
        };
        assert_eq!(stored.status(), expected, "after attempt {attempt}");
        assert_eq!(stored.error.as_deref(), Some("engine crashed"));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_finished_eval_stores_its_result() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g = game(&mut conn, "w1", "b1").await;
    let a = create_user("alice", false, &mut conn).await;
    let eval = GameEval::request(g, a.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    GameEval::claim("worker-a", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap();
    let result = result_for(&["wQ", "bQ /wQ"]);

    assert!(GameEval::finish(
        eval.id,
        "worker-a",
        "stockbee fd64d5c1 50>2>800",
        &result,
        &mut conn
    )
    .await
    .unwrap());

    let stored = GameEval::find_by_game(g, &mut conn).await.unwrap().unwrap();
    assert_eq!(stored.status(), EvalStatus::Done);
    assert_eq!(stored.progress_pct, 100);
    assert_eq!(stored.result(), Some(result));
    assert_eq!(stored.engine.as_deref(), Some("stockbee fd64d5c1 50>2>800"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_eval_is_queued_again_on_the_next_request() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g = game(&mut conn, "w1", "b1").await;
    let a = create_user("alice", false, &mut conn).await;
    let b = create_user("bobby", false, &mut conn).await;
    let eval = GameEval::request(g, a.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    GameEval::claim("worker-a", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap();
    assert!(
        GameEval::fail(eval.id, "worker-a", "engine crashed", &mut conn)
            .await
            .unwrap()
    );

    let again = GameEval::request(g, b.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();

    assert_eq!(again.id, eval.id);
    assert_eq!(again.status(), EvalStatus::Queued);
    assert_eq!(again.requested_by, Some(b.id));
    assert_eq!((again.attempts, again.error), (0, None));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_silent_worker_loses_its_eval_and_repeat_failures_give_up() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g = game(&mut conn, "w1", "b1").await;
    let a = create_user("alice", false, &mut conn).await;
    let eval = GameEval::request(g, a.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    // A negative staleness puts the cutoff in the future, so every running eval counts as quiet.
    let everything = Duration::seconds(-1);

    GameEval::claim("worker-a", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap();
    assert_eq!(
        GameEval::sweep_stale(everything, 2, &mut conn)
            .await
            .unwrap(),
        (1, 0)
    );
    let requeued = GameEval::find_by_game(g, &mut conn).await.unwrap().unwrap();
    assert_eq!(requeued.status(), EvalStatus::Queued);
    assert!(
        !GameEval::report_progress(eval.id, "worker-a", 10, &mut conn)
            .await
            .unwrap()
    );

    GameEval::claim("worker-b", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap();
    assert_eq!(
        GameEval::sweep_stale(everything, 2, &mut conn)
            .await
            .unwrap(),
        (0, 1)
    );
    let failed = GameEval::find_by_game(g, &mut conn).await.unwrap().unwrap();
    assert_eq!(failed.status(), EvalStatus::Failed);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_queue_snapshot_lists_waiting_and_running_work() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g1 = game(&mut conn, "w1", "b1").await;
    let g2 = game(&mut conn, "w2", "b2").await;
    let a = create_user("alice", false, &mut conn).await;
    let b = create_user("bobby", false, &mut conn).await;
    GameEval::request(g1, a.id, 40, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    let waiting = GameEval::request(g2, b.id, 30, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    let running = GameEval::claim("worker-a", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .eval;
    GameEval::report_progress(running.id, "worker-a", 50, &mut conn)
        .await
        .unwrap();

    let snap = GameEval::queue_snapshot(&mut conn).await.unwrap();

    assert_eq!(snap.queued, vec![(waiting.id, 30)]);
    let running: Vec<(i32, i16, bool)> = snap
        .running
        .iter()
        .map(|r| (r.moves, r.progress_pct, r.started_at.is_some()))
        .collect();
    assert_eq!(
        running,
        vec![(40, 50, true)],
        "started_at feeds the own-pace estimate"
    );
    assert_eq!(snap.workers, 1);
    assert_eq!(snap.position(waiting.id), Some(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn recent_lists_finished_evals_newest_first_with_who_asked() {
    use diesel::sql_types::Uuid as SqlUuid;
    use diesel_async::RunQueryDsl;

    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g1 = game(&mut conn, "white1", "black1").await;
    let g2 = game(&mut conn, "white2", "black2").await;
    let g3 = game(&mut conn, "white3", "black3").await;
    let alice = create_user("alice", false, &mut conn).await;
    let gone = create_user("gone", false, &mut conn).await;
    let bobby = create_user("bobby", false, &mut conn).await;
    let moves = result_for(&["wQ", "bQ /wQ"]);

    for (g, asker) in [(g1, alice.id), (g2, gone.id)] {
        let eval = GameEval::request(g, asker, 2, ONE_WORKER, &mut conn)
            .await
            .unwrap();
        GameEval::claim("worker-a", AutoEvals::QueuedOrNew, &mut conn)
            .await
            .unwrap();
        assert!(
            GameEval::finish(eval.id, "worker-a", "test", &moves, &mut conn)
                .await
                .unwrap()
        );
    }
    GameEval::request(g3, bobby.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    // What User::soft_delete leaves behind: accounts are never removed.
    diesel::sql_query(
        "UPDATE users SET deleted = true, username = 'deleted_user_' || id WHERE id = $1",
    )
    .bind::<SqlUuid, _>(gone.id)
    .execute(&mut conn)
    .await
    .unwrap();

    let recent = GameEval::recent(10, &mut conn).await.unwrap();

    assert_eq!(recent.len(), 2, "the queued eval is not listed");
    assert_eq!(
        (recent[0].white.as_str(), recent[0].black.as_str()),
        ("white2", "black2")
    );
    assert_eq!(
        recent[0].requested_by, None,
        "a deleted requester is shown as nobody"
    );
    assert_eq!(recent[1].requested_by.as_deref(), Some("alice"));
    assert_eq!(GameEval::in_line(&mut conn).await.unwrap(), 1);
}

const TWELVE_MOVES: &str = r"wL ;bL wL-;wQ /wL;bQ bL/;wA1 \wL;bA1 bQ/;wA2 -wQ;bA2 bA1/;wA3 /wQ;bA3 bA2-;wG1 -wA2;bG1 bA3\;";

struct Played {
    min_rating: f64,
    tournament: bool,
    finished: bool,
    queen_rule: bool,
    moves: &'static str,
}

impl Default for Played {
    fn default() -> Self {
        Played {
            min_rating: 1900.0,
            tournament: false,
            finished: true,
            queen_rule: true,
            moves: TWELVE_MOVES,
        }
    }
}

async fn played_game(name: &str, played: Played, conn: &mut DbConn<'_>) -> Uuid {
    let w = create_user(&format!("{name}w"), false, conn).await;
    let b = create_user(&format!("{name}b"), false, conn).await;
    let mut game = new_game(w.id, b.id, GameSpeed::Rapid, true);
    game.finished = played.finished;
    game.tournament_queen_rule = played.queen_rule;
    game.white_rating = Some(played.min_rating + 100.0);
    game.black_rating = Some(played.min_rating);
    game.history = played.moves.to_string();
    game.turn = played.moves.matches(';').count() as i32;
    if played.tournament {
        enter_tournament(&mut game, name, conn).await;
    }
    create_game(game, conn).await.id
}

// Slots can only come from a real tournament start, so the game is owned by an arena
// ordinal instead: the schema's cheapest valid tournament game.
async fn enter_tournament(game: &mut NewGame, name: &str, conn: &mut DbConn<'_>) {
    let tournament = create_rr_tournament(
        game.white_id,
        &format!("{name} cup"),
        TournamentStatus::NotStarted,
        conn,
    )
    .await;
    insert_dense_memberships(tournament.id, &[game.white_id, game.black_id], conn).await;
    game.tournament_id = Some(tournament.id);
    game.arena_ordinal = Some(0);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_idle_queue_picks_tournament_games_then_the_strongest_players() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let strong = played_game(
        "strong",
        Played {
            min_rating: 2100.0,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    let cup = played_game(
        "cupgame",
        Played {
            min_rating: 1850.0,
            tournament: true,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    let decent = played_game(
        "decent",
        Played {
            min_rating: 1900.0,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    played_game(
        "weak",
        Played {
            min_rating: 1750.0,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    played_game(
        "ongoing",
        Played {
            finished: false,
            min_rating: 2400.0,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    played_game(
        "norule",
        Played {
            queen_rule: false,
            min_rating: 2400.0,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    played_game(
        "short",
        Played {
            moves: "wL ;bL wL-;",
            min_rating: 2400.0,
            ..Default::default()
        },
        &mut conn,
    )
    .await;

    let mut picked = Vec::new();
    for worker in ["w1", "w2", "w3", "w4"] {
        if let Some(ClaimedEval { eval, .. }) =
            GameEval::claim(worker, AutoEvals::QueuedOrNew, &mut conn)
                .await
                .unwrap()
        {
            assert!(eval.requested_by_system);
            assert_eq!(eval.requested_by, None);
            assert_eq!(eval.moves, 12);
            picked.push(eval.game_id);
        }
    }
    assert_eq!(picked, vec![cup, strong, decent], "nothing else qualifies");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_user_request_sends_a_busy_workers_auto_eval_back_in_line() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    played_game("strong", Played::default(), &mut conn).await;
    let mine = game(&mut conn, "w1", "b1").await;
    let asker = create_user("asker", false, &mut conn).await;
    let auto = GameEval::claim("worker-a", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .eval;

    let request = GameEval::request(mine, asker.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();

    assert!(
        !GameEval::report_progress(auto.id, "worker-a", 40, &mut conn)
            .await
            .unwrap(),
        "the auto-eval's worker must be told to stop"
    );
    let next = GameEval::claim("worker-a", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .eval;
    assert_eq!(next.id, request.id);
    let resumed = GameEval::claim("worker-b", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .eval;
    assert_eq!(resumed.id, auto.id, "the auto-eval comes back next");
    assert_eq!(resumed.attempts, 1, "being sent back was no failed attempt");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_user_request_leaves_auto_evals_alone_while_a_worker_is_idle() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    played_game("strong", Played::default(), &mut conn).await;
    let mine = game(&mut conn, "w1", "b1").await;
    let asker = create_user("asker", false, &mut conn).await;
    let auto = GameEval::claim("worker-a", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .eval;

    let request = GameEval::request(mine, asker.id, 2, 2, &mut conn)
        .await
        .unwrap();

    assert!(
        GameEval::report_progress(auto.id, "worker-a", 40, &mut conn)
            .await
            .unwrap(),
        "the idle worker takes the request instead"
    );
    let next = GameEval::claim("worker-b", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .eval;
    assert_eq!(next.id, request.id);
}

#[tokio::test(flavor = "multi_thread")]
async fn asking_for_the_game_being_auto_evaluated_takes_it_over() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g = played_game("strong", Played::default(), &mut conn).await;
    let asker = create_user("asker", false, &mut conn).await;
    let auto = GameEval::claim("worker-a", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .eval;

    let taken = GameEval::request(g, asker.id, 12, ONE_WORKER, &mut conn)
        .await
        .unwrap();

    assert_eq!(taken.id, auto.id);
    assert_eq!(taken.status(), EvalStatus::Running);
    assert_eq!(
        (taken.requested_by, taken.requested_by_system),
        (Some(asker.id), false)
    );
    assert!(
        GameEval::report_progress(auto.id, "worker-a", 40, &mut conn)
            .await
            .unwrap(),
        "the worker keeps going on what is now the user's eval"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn recent_marks_automatic_evals() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    played_game(
        "strong",
        Played {
            min_rating: 2100.0,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    let auto = GameEval::claim("worker-a", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .eval;
    let moves: Vec<&str> = TWELVE_MOVES
        .split(';')
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .collect();
    assert!(
        GameEval::finish(auto.id, "worker-a", "test", &result_for(&moves), &mut conn)
            .await
            .unwrap()
    );

    let recent = GameEval::recent(10, &mut conn).await.unwrap();

    assert_eq!(recent.len(), 1);
    assert!(recent[0].automatic);
    assert_eq!(recent[0].requested_by, None);
}

/// Finishes whatever the worker claims next, the way a worker would.
async fn finish_next(conn: &mut DbConn<'_>) -> Uuid {
    let eval = GameEval::claim("worker-a", AutoEvals::QueuedOrNew, conn)
        .await
        .unwrap()
        .unwrap()
        .eval;
    let moves: Vec<String> = shared_types::history_moves(TWELVE_MOVES);
    let moves: Vec<&str> = moves.iter().map(String::as_str).collect();
    let result = if eval.moves == 12 {
        result_for(&moves)
    } else {
        result_for(&["wQ", "bQ /wQ"])
    };
    assert!(GameEval::finish(eval.id, "worker-a", "test", &result, conn)
        .await
        .unwrap());
    eval.game_id
}

async fn set_tournament(game: Uuid, finished_days_ago: Option<i32>, conn: &mut DbConn<'_>) {
    use diesel::sql_types::{Integer, Nullable, Uuid as SqlUuid};
    use diesel_async::RunQueryDsl;
    diesel::sql_query(
        "UPDATE tournaments
         SET started_at = now() - (COALESCE($2, 0) + 1) * interval '1 day',
             finished_at = now() - $2 * interval '1 day'
         WHERE id = (SELECT tournament_id FROM games WHERE id = $1)",
    )
    .bind::<SqlUuid, _>(game)
    .bind::<Nullable<Integer>, _>(finished_days_ago)
    .execute(conn)
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_front_page_shows_tournaments_then_requests_then_other_strong_games() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    // Auto-evals pick tournament games first, then by rating, so these finish in this order.
    let live = played_game(
        "live",
        Played {
            tournament: true,
            min_rating: 1850.0,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    let old_cup = played_game(
        "oldcup",
        Played {
            tournament: true,
            min_rating: 1820.0,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    let star = played_game(
        "star",
        Played {
            min_rating: 2200.0,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    let plain = played_game(
        "plain",
        Played {
            min_rating: 1900.0,
            ..Default::default()
        },
        &mut conn,
    )
    .await;
    for expected in [live, old_cup, star, plain] {
        assert_eq!(finish_next(&mut conn).await, expected);
    }
    set_tournament(live, None, &mut conn).await;
    set_tournament(old_cup, Some(30), &mut conn).await;
    let asked = game(&mut conn, "w1", "b1").await;
    let asker = create_user("asker", false, &mut conn).await;
    GameEval::request(asked, asker.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    assert_eq!(finish_next(&mut conn).await, asked);

    let shown: Vec<String> = GameEval::recent(10, &mut conn)
        .await
        .unwrap()
        .into_iter()
        .map(|eval| eval.white)
        .collect();

    // The live tournament game was evaluated first of all and still leads.
    assert_eq!(shown, vec!["livew", "w1", "plainw", "starw", "oldcupw"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_worker_without_auto_only_takes_user_requests() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    played_game("strong", Played::default(), &mut conn).await;
    assert!(GameEval::claim("worker-a", AutoEvals::Off, &mut conn)
        .await
        .unwrap()
        .is_none());

    let mine = game(&mut conn, "w1", "b1").await;
    let asker = create_user("asker", false, &mut conn).await;
    let request = GameEval::request(mine, asker.id, 2, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    let claimed = GameEval::claim("worker-a", AutoEvals::Off, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .eval;
    assert_eq!(claimed.id, request.id);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_worker_without_auto_leaves_requeued_auto_evals_alone() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let strong = played_game("strong", Played::default(), &mut conn).await;
    let auto = GameEval::claim("worker-a", AutoEvals::QueuedOrNew, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .eval;
    assert_eq!(auto.game_id, strong);
    GameEval::sweep_stale(Duration::seconds(-1), 3, &mut conn)
        .await
        .unwrap();

    assert!(GameEval::claim("worker-b", AutoEvals::Off, &mut conn)
        .await
        .unwrap()
        .is_none());
    let resumed = GameEval::claim("worker-b", AutoEvals::Queued, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .eval;
    assert_eq!(resumed.id, auto.id);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_worker_that_may_not_pick_new_games_only_takes_the_queue() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    played_game("strong", Played::default(), &mut conn).await;
    assert!(GameEval::claim("worker-a", AutoEvals::Queued, &mut conn)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_front_page_needs_both_ratings_to_call_a_game_strong() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let w = create_user("w1", false, &mut conn).await;
    let b = create_user("b1", false, &mut conn).await;
    let mut half_rated = new_game(w.id, b.id, GameSpeed::Rapid, true);
    half_rated.turn = 20;
    half_rated.white_rating = Some(2300.0);
    half_rated.black_rating = None;
    let half_rated = create_game(half_rated, &mut conn).await.id;
    let asker = create_user("asker", false, &mut conn).await;
    let eval = GameEval::request(half_rated, asker.id, 20, ONE_WORKER, &mut conn)
        .await
        .unwrap();
    GameEval::claim("worker-a", AutoEvals::Off, &mut conn)
        .await
        .unwrap()
        .unwrap();
    assert!(GameEval::finish(
        eval.id,
        "worker-a",
        "test",
        &result_for(&["wQ", "bQ /wQ"]),
        &mut conn
    )
    .await
    .unwrap());

    assert!(GameEval::recent(6, &mut conn).await.unwrap().is_empty());
}
