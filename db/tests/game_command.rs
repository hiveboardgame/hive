mod common;

use chrono::Utc;
use db_lib::{
    db_error::DbError,
    game_command::{execute, Command, Outcome},
    get_conn,
    models::{Game, NewGame, NewUser, User},
    DbConn,
};
use hive_lib::{Color, Direction, GameControl, GameResult, GameStatus, GameType, Position, Turn};
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
