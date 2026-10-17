mod common;

use chrono::{DateTime, Duration as ChronoDuration, SubsecRound, Utc};
use db_lib::{
    db_error::DbError,
    game_command::{execute, Command, Outcome, MAX_MOVE_COMPENSATION_SECS},
    get_conn,
    models::{Game, NewGame, NewUser, User},
    DbConn,
};
use hive_lib::{
    Color,
    Direction,
    GameControl,
    GameResult,
    GameStatus,
    GameType,
    Piece,
    Position,
    State,
    Turn,
};
use shared_types::{Conclusion, GameSpeed, GameStart, TimeMode, TournamentGameResult};
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread")]
async fn takebacks_to_opening_turns_keep_the_clock_running() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    for (label, time_mode, time_base, time_increment, speed) in [
        (
            "rt",
            TimeMode::RealTime,
            Some(60),
            Some(0),
            GameSpeed::Bullet,
        ),
        (
            "corr",
            TimeMode::Correspondence,
            None,
            Some(60),
            GameSpeed::Correspondence,
        ),
    ] {
        let white = create_user(&format!("tb_{label}_white"), &mut conn).await;
        let black = create_user(&format!("tb_{label}_black"), &mut conn).await;
        let game = create_moves_game(
            white.id,
            black.id,
            time_mode,
            time_base,
            time_increment,
            speed,
            &mut conn,
        )
        .await;

        for (user_id, piece, position) in [
            (white.id, "wA1", Position::initial_spawn_position()),
            (
                black.id,
                "bA1",
                Position::initial_spawn_position().to(Direction::E),
            ),
        ] {
            execute(
                game.id,
                Command::Move {
                    user_id,
                    turn: Turn::Move(piece.parse().expect("parse opening piece"), position),
                    compensation: 0.0,
                    arrived_at: None,
                },
                &mut conn,
            )
            .await
            .expect("play opening move");
        }

        execute(
            game.id,
            Command::Control {
                user_id: white.id,
                control: GameControl::TakebackRequest(Color::White),
            },
            &mut conn,
        )
        .await
        .expect("request takeback");
        let turn_one = execute(
            game.id,
            Command::Control {
                user_id: black.id,
                control: GameControl::TakebackAccept(Color::Black),
            },
            &mut conn,
        )
        .await
        .expect("accept takeback");
        let Outcome::Applied { game: turn_one, .. } = turn_one else {
            panic!("takeback acceptance updates the game")
        };

        assert_running_opening_clock(&turn_one, 1);

        execute(
            game.id,
            Command::Control {
                user_id: white.id,
                control: GameControl::TakebackRequest(Color::White),
            },
            &mut conn,
        )
        .await
        .expect("request the remaining opening takeback");
        let turn_zero = execute(
            game.id,
            Command::Control {
                user_id: black.id,
                control: GameControl::TakebackAccept(Color::Black),
            },
            &mut conn,
        )
        .await
        .expect("accept the remaining opening takeback");
        let Outcome::Applied {
            game: turn_zero, ..
        } = turn_zero
        else {
            panic!("takeback acceptance updates the game")
        };

        assert_running_opening_clock(&turn_zero, 0);

        let abort_error = execute(
            game.id,
            Command::Control {
                user_id: white.id,
                control: GameControl::Abort(Color::White),
            },
            &mut conn,
        )
        .await
        .expect_err("a started game returned to turn zero cannot be aborted");
        assert!(matches!(abort_error, DbError::InvalidAction { .. }));

        let still_running = Game::find_by_uuid(&game.id, &mut conn)
            .await
            .expect("reload game after rejected abort");
        assert_running_opening_clock(&still_running, 0);

        let takeback_error = execute(
            game.id,
            Command::Control {
                user_id: white.id,
                control: GameControl::TakebackRequest(Color::White),
            },
            &mut conn,
        )
        .await
        .expect_err("turn zero has no remaining move to take back");
        assert!(matches!(takeback_error, DbError::InvalidAction { .. }));

        execute(
            game.id,
            Command::Control {
                user_id: white.id,
                control: GameControl::DrawOffer(Color::White),
            },
            &mut conn,
        )
        .await
        .expect("offer a draw from the resumed opening");
        execute(
            game.id,
            Command::Control {
                user_id: black.id,
                control: GameControl::DrawReject(Color::Black),
            },
            &mut conn,
        )
        .await
        .expect("reject a draw from the resumed opening");
        let resigned = execute(
            game.id,
            Command::Control {
                user_id: white.id,
                control: GameControl::Resign(Color::White),
            },
            &mut conn,
        )
        .await
        .expect("resign from the resumed opening");
        let Outcome::Applied { game: resigned, .. } = resigned else {
            panic!("resignation updates the game")
        };
        assert!(resigned.finished);
        assert_eq!(
            resigned.game_status,
            GameStatus::Finished(GameResult::Winner(Color::Black)).to_string()
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn move_compensation_outside_zero_to_the_ceiling_is_rejected() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let white = create_user("comp_ceiling_white", &mut conn).await;
    let black = create_user("comp_ceiling_black", &mut conn).await;
    let game = create_moves_game(
        white.id,
        black.id,
        TimeMode::RealTime,
        Some(180),
        Some(2),
        GameSpeed::Blitz,
        &mut conn,
    )
    .await;
    let opening = || {
        Turn::Move(
            "wA1".parse().expect("parse opening piece"),
            Position::initial_spawn_position(),
        )
    };

    for compensation in [
        MAX_MOVE_COMPENSATION_SECS + 0.001,
        -0.001,
        f64::NAN,
        f64::INFINITY,
    ] {
        let error = execute(
            game.id,
            Command::Move {
                user_id: white.id,
                turn: opening(),
                compensation,
                arrived_at: None,
            },
            &mut conn,
        )
        .await
        .expect_err("compensation outside the accepted range is refused");
        assert!(
            matches!(error, DbError::InvalidInput { .. }),
            "{compensation}: {error:?}"
        );
    }
    let untouched = Game::find_by_uuid(&game.id, &mut conn)
        .await
        .expect("reload game after rejected moves");
    assert_eq!(untouched.turn, 0);

    let accepted = execute(
        game.id,
        Command::Move {
            user_id: white.id,
            turn: opening(),
            compensation: MAX_MOVE_COMPENSATION_SECS,
            arrived_at: None,
        },
        &mut conn,
    )
    .await
    .expect("compensation at the ceiling is accepted");
    let Outcome::Applied { game: moved, .. } = accepted else {
        panic!("a move updates the game")
    };
    assert_eq!(moved.turn, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn compensation_never_refunds_more_than_the_move_took() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let white = create_user("comp_refund_white", &mut conn).await;
    let black = create_user("comp_refund_black", &mut conn).await;
    let game = create_moves_game(
        white.id,
        black.id,
        TimeMode::RealTime,
        Some(180),
        Some(2),
        GameSpeed::Blitz,
        &mut conn,
    )
    .await;

    let mut started = None;
    for (user_id, piece, position) in [
        (white.id, "wA1", Position::initial_spawn_position()),
        (
            black.id,
            "bA1",
            Position::initial_spawn_position().to(Direction::E),
        ),
    ] {
        let outcome = execute(
            game.id,
            Command::Move {
                user_id,
                turn: Turn::Move(piece.parse().expect("parse opening piece"), position),
                compensation: 0.0,
                arrived_at: None,
            },
            &mut conn,
        )
        .await
        .expect("play opening move");
        let Outcome::Applied { game, .. } = outcome else {
            panic!("an opening move updates the game")
        };
        started = Some(game);
    }
    let started = started.expect("the clock is running");
    let white_before = started.white_time_left.expect("white has a clock");

    let outcome = execute(
        game.id,
        Command::Move {
            user_id: white.id,
            turn: Turn::Move(
                "wG1".parse().expect("parse piece"),
                Position::initial_spawn_position().to(Direction::W),
            ),
            compensation: MAX_MOVE_COMPENSATION_SECS,
            arrived_at: None,
        },
        &mut conn,
    )
    .await
    .expect("play an instant move");
    let Outcome::Applied { game: moved, .. } = outcome else {
        panic!("a move updates the game")
    };

    let increment = 2 * 1_000_000_000;
    assert_eq!(
        moved.white_time_left,
        Some(white_before + increment),
        "an instant move keeps exactly its increment, not the compensation on top"
    );
}

fn assert_running_opening_clock(game: &Game, expected_turn: i32) {
    assert_eq!(game.turn, expected_turn);
    assert_eq!(game.game_status, GameStatus::InProgress.to_string());
    let last_interaction = game
        .last_interaction
        .expect("a live opening has a clock anchor");
    assert!(
        game.timeout_at
            .expect("a live opening has a timeout deadline")
            > last_interaction
    );
}

async fn create_user(username: &str, conn: &mut DbConn<'_>) -> User {
    User::create(
        NewUser::new(username, "password", &format!("{username}@example.com")).expect("build user"),
        conn,
    )
    .await
    .expect("insert user")
}

async fn create_moves_game(
    white_id: Uuid,
    black_id: Uuid,
    time_mode: TimeMode,
    time_base: Option<i32>,
    time_increment: Option<i32>,
    speed: GameSpeed,
    conn: &mut DbConn<'_>,
) -> Game {
    let now = Utc::now();
    let time_left = time_base
        .or(time_increment)
        .map(|seconds| i64::from(seconds) * 1_000_000_000_i64);
    Game::create(
        NewGame {
            tournament_id: None,
            nanoid: nanoid::nanoid!(12),
            current_player_id: white_id,
            black_id,
            finished: false,
            game_status: GameStatus::NotStarted.to_string(),
            game_type: GameType::MLP.to_string(),
            history: String::new(),
            game_control_history: String::new(),
            rated: true,
            tournament_queen_rule: false,
            turn: 0,
            white_id,
            white_rating: None,
            black_rating: None,
            white_rating_change: None,
            black_rating_change: None,
            created_at: now,
            updated_at: now,
            time_mode: time_mode.to_string(),
            time_base,
            time_increment,
            last_interaction: None,
            black_time_left: time_left,
            white_time_left: time_left,
            speed: speed.to_string(),
            hashes: Vec::new(),
            conclusion: Conclusion::Unknown.to_string(),
            tournament_game_result: TournamentGameResult::Unknown.to_string(),
            game_start: GameStart::Moves.to_string(),
            move_times: Vec::new(),
            timeout_at: None,
            tournament_slot_id: None,
            arena_ordinal: None,
            white_berserked: false,
            black_berserked: false,
            arena_move_due_at: None,
        },
        conn,
    )
    .await
    .expect("insert game")
}

const SECOND: i64 = 1_000_000_000;

// A one-minute game in progress, white to move, whose clock last switched at `last_move`.
async fn minute_game_switched_at(
    label: &str,
    time_mode: TimeMode,
    last_move: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> (Game, User) {
    let white = create_user(&format!("{label}_white"), conn).await;
    let black = create_user(&format!("{label}_black"), conn).await;
    let mut new_game = common::fixtures::new_game(white.id, black.id, GameSpeed::Bullet, false);
    new_game.time_mode = time_mode.to_string();
    (new_game.time_base, new_game.time_increment) = match time_mode {
        TimeMode::Correspondence => (None, Some(60)),
        _ => (Some(60), Some(0)),
    };
    new_game.white_time_left = Some(60 * SECOND);
    new_game.black_time_left = Some(60 * SECOND);
    new_game.last_interaction = Some(last_move);
    new_game.timeout_at = Some(last_move + ChronoDuration::seconds(60));
    (
        Game::create(new_game, conn).await.expect("insert game"),
        white,
    )
}

fn whites_move(game: &Game) -> Turn {
    let state = State::new_from_str(&game.history, &game.game_type).expect("replay history");
    Turn::Move(
        "wA1".parse::<Piece>().expect("parse piece"),
        Position::from_string("wQ/", &state.board).expect("a hex beside white only"),
    )
}

async fn move_arriving(
    game: &Game,
    white: &User,
    arrived_at: DateTime<Utc>,
    compensation: f64,
    conn: &mut DbConn<'_>,
) -> Outcome {
    execute(
        game.id,
        Command::Move {
            user_id: white.id,
            turn: whites_move(game),
            compensation,
            arrived_at: Some(arrived_at),
        },
        conn,
    )
    .await
    .expect("the move is processed")
}

#[tokio::test(flavor = "multi_thread")]
async fn a_move_arriving_after_the_deadline_is_saved_by_its_compensation() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let last_move = Utc::now().trunc_subsecs(3) - ChronoDuration::milliseconds(60_300);
    let (game, white) =
        minute_game_switched_at("late_saved", TimeMode::RealTime, last_move, &mut conn).await;

    let outcome = move_arriving(
        &game,
        &white,
        last_move + ChronoDuration::milliseconds(60_300),
        0.5,
        &mut conn,
    )
    .await;

    let Outcome::Applied { game: moved, .. } = outcome else {
        panic!("the compensated move beats the flag: {outcome:?}")
    };
    assert!(!moved.finished);
    assert_eq!(moved.white_time_left, Some(SECOND / 5));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_move_later_than_its_compensation_covers_flags() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let last_move = Utc::now().trunc_subsecs(3) - ChronoDuration::milliseconds(60_300);
    let (game, white) =
        minute_game_switched_at("late_flagged", TimeMode::RealTime, last_move, &mut conn).await;

    let outcome = move_arriving(
        &game,
        &white,
        last_move + ChronoDuration::milliseconds(60_300),
        0.2,
        &mut conn,
    )
    .await;

    let Outcome::TimedOut { game: flagged, .. } = outcome else {
        panic!("0.2 s cannot cover a move 0.3 s late: {outcome:?}")
    };
    assert_eq!(flagged.conclusion, Conclusion::Timeout.to_string());
    assert_eq!(flagged.white_time_left, Some(0));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_real_time_flag_waits_until_no_move_in_flight_could_be_saved() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let now = Utc::now();

    let (within_grace, _) = minute_game_switched_at(
        "grace_within",
        TimeMode::RealTime,
        now - ChronoDuration::milliseconds(60_500),
        &mut conn,
    )
    .await;
    let outcome = execute(within_grace.id, Command::SettleDeadline, &mut conn)
        .await
        .expect("settle");
    let Outcome::Applied { game, .. } = outcome else {
        panic!("settling never removes a game")
    };
    assert!(
        !game.finished,
        "half a second past the deadline is inside the grace"
    );

    let (past_grace, _) = minute_game_switched_at(
        "grace_past",
        TimeMode::RealTime,
        now - ChronoDuration::milliseconds(61_500),
        &mut conn,
    )
    .await;
    let outcome = execute(past_grace.id, Command::SettleDeadline, &mut conn)
        .await
        .expect("settle");
    let Outcome::Applied { game, .. } = outcome else {
        panic!("settling never removes a game")
    };
    assert!(game.finished);
    assert_eq!(game.conclusion, Conclusion::Timeout.to_string());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_correspondence_deadline_has_no_grace() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let (game, _) = minute_game_switched_at(
        "corr_deadline",
        TimeMode::Correspondence,
        Utc::now() - ChronoDuration::milliseconds(60_500),
        &mut conn,
    )
    .await;

    let outcome = execute(game.id, Command::SettleDeadline, &mut conn)
        .await
        .expect("settle");

    let Outcome::Applied { game, .. } = outcome else {
        panic!("settling never removes a game")
    };
    assert!(game.finished);
}

#[tokio::test(flavor = "multi_thread")]
async fn time_between_a_moves_arrival_and_its_commit_is_charged_to_nobody() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let last_move = Utc::now().trunc_subsecs(3) - ChronoDuration::seconds(10);
    let (game, white) =
        minute_game_switched_at("server_delay", TimeMode::RealTime, last_move, &mut conn).await;
    let arrived_at = last_move + ChronoDuration::seconds(5);

    let outcome = move_arriving(&game, &white, arrived_at, 0.0, &mut conn).await;

    let Outcome::Applied { game: moved, .. } = outcome else {
        panic!("a timely move is applied: {outcome:?}")
    };
    assert_eq!(
        moved.white_time_left,
        Some(55 * SECOND),
        "white pays only until the move arrived"
    );
    let black_clock_started = moved.last_interaction.expect("the clock runs");
    assert!(
        black_clock_started >= arrived_at + ChronoDuration::seconds(4),
        "black's clock starts at the commit, not at white's arrival"
    );
    assert_eq!(
        moved.timeout_at,
        Some(black_clock_started + ChronoDuration::seconds(60))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_player_on_move_can_end_their_own_grace() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let last_move = Utc::now().trunc_subsecs(3) - ChronoDuration::milliseconds(60_300);
    let (game, white) =
        minute_game_switched_at("grace_owner", TimeMode::RealTime, last_move, &mut conn).await;

    let error = execute(
        game.id,
        Command::Move {
            user_id: game.black_id,
            turn: whites_move(&game),
            compensation: 0.0,
            arrived_at: None,
        },
        &mut conn,
    )
    .await
    .expect_err("black is not on move");
    assert!(matches!(error, DbError::InvalidAction { .. }), "{error:?}");

    let outcome = move_arriving(
        &game,
        &white,
        last_move + ChronoDuration::milliseconds(60_300),
        0.5,
        &mut conn,
    )
    .await;
    assert!(
        matches!(outcome, Outcome::Applied { .. }),
        "black's move must not flag white while white's move can still be saved: {outcome:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_flagged_player_gets_no_grace_for_anything_but_a_move() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let last_move = Utc::now() - ChronoDuration::milliseconds(60_300);
    let (game, white) =
        minute_game_switched_at("grace_control", TimeMode::RealTime, last_move, &mut conn).await;

    let outcome = execute(
        game.id,
        Command::Control {
            user_id: white.id,
            control: GameControl::DrawOffer(Color::White),
        },
        &mut conn,
    )
    .await
    .expect("the control is processed");

    let Outcome::TimedOut { game: flagged, .. } = outcome else {
        panic!("white is past the deadline and offers no move to be saved by: {outcome:?}")
    };
    assert_eq!(flagged.conclusion, Conclusion::Timeout.to_string());
}
