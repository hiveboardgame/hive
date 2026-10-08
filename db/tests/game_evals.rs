mod common;

use chrono::Duration;
use common::fixtures::{create_game, create_user, new_game};
use db_lib::{db_error::DbError, get_conn, models::GameEval, DbConn};
use shared_types::{EvalResult, EvalStatus, GameSpeed, MoveEval};
use uuid::Uuid;

async fn game(conn: &mut DbConn<'_>, white: &str, black: &str) -> Uuid {
    let w = create_user(white, false, conn).await;
    let b = create_user(black, false, conn).await;
    create_game(new_game(w.id, b.id, GameSpeed::Rapid, true), conn)
        .await
        .id
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

    let first = GameEval::request(g, asker.id, 2, &mut conn).await.unwrap();
    let second = GameEval::request(g, other.id, 2, &mut conn).await.unwrap();

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

    let first = GameEval::request(g1, asker.id, 2, &mut conn).await.unwrap();
    let err = GameEval::request(g2, asker.id, 2, &mut conn)
        .await
        .unwrap_err();
    assert!(matches!(err, DbError::InvalidAction { .. }), "{err:?}");

    let claimed = GameEval::claim("worker-a", true, &mut conn)
        .await
        .unwrap()
        .unwrap();
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

    let next = GameEval::request(g2, asker.id, 2, &mut conn).await.unwrap();
    assert_eq!(next.status(), EvalStatus::Queued);
}

#[tokio::test(flavor = "multi_thread")]
async fn claims_go_out_oldest_first_and_only_once() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g1 = game(&mut conn, "w1", "b1").await;
    let g2 = game(&mut conn, "w2", "b2").await;
    let a = create_user("alice", false, &mut conn).await;
    let b = create_user("bobby", false, &mut conn).await;
    let first = GameEval::request(g1, a.id, 2, &mut conn).await.unwrap();
    let second = GameEval::request(g2, b.id, 2, &mut conn).await.unwrap();

    let c1 = GameEval::claim("worker-a", true, &mut conn)
        .await
        .unwrap()
        .unwrap();
    let c2 = GameEval::claim("worker-b", true, &mut conn)
        .await
        .unwrap()
        .unwrap();
    let none = GameEval::claim("worker-c", true, &mut conn).await.unwrap();

    assert_eq!((c1.id, c2.id), (first.id, second.id));
    assert!(none.is_none());
    assert_eq!(c1.status(), EvalStatus::Running);
    assert_eq!(c1.attempts, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_claiming_worker_can_report() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g = game(&mut conn, "w1", "b1").await;
    let a = create_user("alice", false, &mut conn).await;
    let eval = GameEval::request(g, a.id, 2, &mut conn).await.unwrap();
    GameEval::claim("worker-a", true, &mut conn).await.unwrap();

    assert!(
        !GameEval::report_progress(eval.id, "worker-b", 50, &mut conn)
            .await
            .unwrap()
    );
    assert!(!GameEval::finish(
        eval.id,
        "worker-b",
        "x",
        &result_for(&["wQ", "bQ /wQ"]),
        &mut conn
    )
    .await
    .unwrap());
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
async fn a_finished_eval_stores_its_result() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g = game(&mut conn, "w1", "b1").await;
    let a = create_user("alice", false, &mut conn).await;
    let eval = GameEval::request(g, a.id, 2, &mut conn).await.unwrap();
    GameEval::claim("worker-a", true, &mut conn).await.unwrap();
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
    let eval = GameEval::request(g, a.id, 2, &mut conn).await.unwrap();
    GameEval::claim("worker-a", true, &mut conn).await.unwrap();
    assert!(
        GameEval::fail(eval.id, "worker-a", "engine crashed", &mut conn)
            .await
            .unwrap()
    );

    let again = GameEval::request(g, b.id, 2, &mut conn).await.unwrap();

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
    let eval = GameEval::request(g, a.id, 2, &mut conn).await.unwrap();
    // A negative staleness puts the cutoff in the future, so every running eval counts as quiet.
    let everything = Duration::seconds(-1);

    GameEval::claim("worker-a", true, &mut conn).await.unwrap();
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

    GameEval::claim("worker-b", true, &mut conn).await.unwrap();
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
    GameEval::request(g1, a.id, 40, &mut conn).await.unwrap();
    let waiting = GameEval::request(g2, b.id, 30, &mut conn).await.unwrap();
    let running = GameEval::claim("worker-a", true, &mut conn)
        .await
        .unwrap()
        .unwrap();
    GameEval::report_progress(running.id, "worker-a", 50, &mut conn)
        .await
        .unwrap();

    let snap = GameEval::queue_snapshot(&mut conn).await.unwrap();

    assert_eq!(snap.queued, vec![(waiting.id, 30)]);
    assert_eq!(snap.running, vec![(40, 50)]);
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
        let eval = GameEval::request(g, asker, 2, &mut conn).await.unwrap();
        GameEval::claim("worker-a", true, &mut conn).await.unwrap();
        assert!(
            GameEval::finish(eval.id, "worker-a", "test", &moves, &mut conn)
                .await
                .unwrap()
        );
    }
    GameEval::request(g3, bobby.id, 2, &mut conn).await.unwrap();
    diesel::sql_query("DELETE FROM users WHERE id = $1")
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
        game.tournament_id = Some(tournament(w.id, name, conn).await);
    }
    create_game(game, conn).await.id
}

async fn tournament(organizer: Uuid, name: &str, conn: &mut DbConn<'_>) -> Uuid {
    use chrono::Utc;
    use db_lib::models::{NewTournament, Tournament};
    use shared_types::{
        ScoringMode,
        StartMode,
        Tiebreaker,
        TimeMode,
        TournamentMode,
        TournamentStatus,
    };
    Tournament::create(
        organizer,
        &NewTournament {
            nanoid: nanoid::nanoid!(11),
            name: name.to_string(),
            description: String::new(),
            scoring: ScoringMode::Game.to_string(),
            tiebreaker: vec![Some(Tiebreaker::RawPoints.to_string())],
            seats: 4,
            min_seats: 2,
            rounds: 1,
            invite_only: false,
            mode: TournamentMode::DoubleRoundRobin.to_string(),
            time_mode: TimeMode::RealTime.to_string(),
            time_base: Some(60),
            time_increment: Some(0),
            band_upper: None,
            band_lower: None,
            start_mode: StartMode::Manual.to_string(),
            starts_at: None,
            ends_at: None,
            started_at: None,
            round_duration: None,
            status: TournamentStatus::NotStarted.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            series: None,
        },
        conn,
    )
    .await
    .expect("insert tournament")
    .id
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
        if let Some(eval) = GameEval::claim(worker, true, &mut conn).await.unwrap() {
            assert!(eval.requested_by_system);
            assert_eq!(eval.requested_by, None);
            assert_eq!(eval.moves, 12);
            picked.push(eval.game_id);
        }
    }
    assert_eq!(picked, vec![cup, strong, decent], "nothing else qualifies");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_user_request_drops_a_running_auto_eval_and_goes_next() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    played_game("strong", Played::default(), &mut conn).await;
    let mine = game(&mut conn, "w1", "b1").await;
    let asker = create_user("asker", false, &mut conn).await;
    let auto = GameEval::claim("worker-a", true, &mut conn)
        .await
        .unwrap()
        .unwrap();

    let request = GameEval::request(mine, asker.id, 2, &mut conn)
        .await
        .unwrap();

    assert!(
        !GameEval::report_progress(auto.id, "worker-a", 40, &mut conn)
            .await
            .unwrap(),
        "the auto-eval's worker must be told to stop"
    );
    let next = GameEval::claim("worker-a", true, &mut conn)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(next.id, request.id);
}

#[tokio::test(flavor = "multi_thread")]
async fn asking_for_the_game_being_auto_evaluated_takes_it_over() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let g = played_game("strong", Played::default(), &mut conn).await;
    let asker = create_user("asker", false, &mut conn).await;
    let auto = GameEval::claim("worker-a", true, &mut conn)
        .await
        .unwrap()
        .unwrap();

    let taken = GameEval::request(g, asker.id, 12, &mut conn).await.unwrap();

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
    let auto = GameEval::claim("worker-a", true, &mut conn)
        .await
        .unwrap()
        .unwrap();
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
    let eval = GameEval::claim("worker-a", true, conn)
        .await
        .unwrap()
        .unwrap();
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

async fn set_tournament(game: Uuid, status: &str, ended_days_ago: i32, conn: &mut DbConn<'_>) {
    use diesel::sql_types::{Integer, Text, Uuid as SqlUuid};
    use diesel_async::RunQueryDsl;
    diesel::sql_query(
        "UPDATE tournaments SET status = $2, ends_at = now() - make_interval(days => $3)
         WHERE id = (SELECT tournament_id FROM games WHERE id = $1)",
    )
    .bind::<SqlUuid, _>(game)
    .bind::<Text, _>(status)
    .bind::<Integer, _>(ended_days_ago)
    .execute(conn)
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_front_page_puts_tournaments_then_requests_then_top_automatic_evals() {
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
    set_tournament(live, "InProgress", 0, &mut conn).await;
    set_tournament(old_cup, "Finished", 30, &mut conn).await;
    let asked = game(&mut conn, "w1", "b1").await;
    let asker = create_user("asker", false, &mut conn).await;
    GameEval::request(asked, asker.id, 2, &mut conn)
        .await
        .unwrap();
    assert_eq!(finish_next(&mut conn).await, asked);

    let shown: Vec<String> = GameEval::recent(10, &mut conn)
        .await
        .unwrap()
        .into_iter()
        .map(|eval| eval.white)
        .collect();

    // The live tournament game was evaluated first of all and still leads; an automatic eval
    // of an ordinary game never shows.
    assert_eq!(shown, vec!["livew", "w1", "starw", "oldcupw"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_worker_without_auto_only_takes_user_requests() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    played_game("strong", Played::default(), &mut conn).await;
    assert!(GameEval::claim("worker-a", false, &mut conn)
        .await
        .unwrap()
        .is_none());

    let mine = game(&mut conn, "w1", "b1").await;
    let asker = create_user("asker", false, &mut conn).await;
    let request = GameEval::request(mine, asker.id, 2, &mut conn)
        .await
        .unwrap();
    let claimed = GameEval::claim("worker-a", false, &mut conn)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claimed.id, request.id);
}
