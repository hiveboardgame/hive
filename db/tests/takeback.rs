mod common;

use chrono::Utc;
use db_lib::{
    db_error::DbError,
    game_command::{execute, Command, Outcome},
    get_conn,
    models::{Game, NewGame, NewUser, User},
    DbConn,
};
use hive_lib::{Color, GameControl, GameStatus, GameType};
use shared_types::{Conclusion, GameSpeed, GameStart, TimeMode, TournamentGameResult};

/// `wL` and `wM` are not in the Base inventory, so this history only replays against the
/// game's stored `game_type`.
const MLP_HISTORY: &str = r"wL ;bL wL-;wQ /wL;bQ bL/;wM \wL;bP bQ/;";
const MLP_AFTER_TAKEBACK: &str = r"wL ;bL wL-;wQ /wL;bQ bL/;wM \wL;";

const BASE_HISTORY: &str = r"wA1 ;bA1 wA1-;wQ /wA1;bQ bA1/;wG1 \wA1;bG1 bQ/;";
const BASE_AFTER_TAKEBACK: &str = r"wA1 ;bA1 wA1-;wQ /wA1;bQ bA1/;wG1 \wA1;";

#[tokio::test(flavor = "multi_thread")]
async fn takeback_replays_an_expansion_game() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let game = setup_game(
        "mlp_white",
        "mlp_black",
        GameType::MLP,
        MLP_HISTORY,
        &mut conn,
    )
    .await;

    let taken_back = take_back(&game, &mut conn)
        .await
        .expect("an expansion game takes back");

    assert_eq!(taken_back.history, MLP_AFTER_TAKEBACK);
    assert_eq!(taken_back.turn, 5);
    assert_eq!(taken_back.current_player_id, game.black_id);
    assert!(!taken_back.finished);
    assert_eq!(taken_back.game_status, GameStatus::InProgress.to_string());
}

#[tokio::test(flavor = "multi_thread")]
async fn takeback_replays_a_base_game() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let game = setup_game(
        "base_white",
        "base_black",
        GameType::Base,
        BASE_HISTORY,
        &mut conn,
    )
    .await;

    let taken_back = take_back(&game, &mut conn)
        .await
        .expect("a base game takes back");

    assert_eq!(taken_back.history, BASE_AFTER_TAKEBACK);
    assert_eq!(taken_back.turn, 5);
}

/// A Base row holding an expansion piece is corrupt, not an expansion game: the stored type
/// stays authoritative and the takeback must refuse rather than infer `Base+MLP` from `wL`.
#[tokio::test(flavor = "multi_thread")]
async fn takeback_rejects_a_history_the_stored_game_type_forbids() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let game = setup_game(
        "forged_white",
        "forged_black",
        GameType::Base,
        MLP_HISTORY,
        &mut conn,
    )
    .await;

    let error = take_back(&game, &mut conn)
        .await
        .expect_err("a Base game containing wL cannot be reconstructed");

    let DbError::InternalError { reason } = &error else {
        panic!("expected invalid stored history, got: {error:?}");
    };
    assert!(
        reason.contains("wL is not part of this game"),
        "expected an inventory rejection, got: {reason}"
    );

    let unchanged = Game::find_by_uuid(&game.id, &mut conn).await.unwrap();
    assert_eq!(unchanged.history, MLP_HISTORY);
    assert_eq!(unchanged.turn, game.turn);
}

async fn take_back(game: &Game, conn: &mut DbConn<'_>) -> Result<Game, DbError> {
    execute(
        game.id,
        Command::Control {
            user_id: game.white_id,
            control: GameControl::TakebackRequest(Color::White),
        },
        conn,
    )
    .await?;
    let outcome = execute(
        game.id,
        Command::Control {
            user_id: game.black_id,
            control: GameControl::TakebackAccept(Color::Black),
        },
        conn,
    )
    .await?;
    let Outcome::Applied { game, .. } = outcome else {
        panic!("takeback acceptance must update the fixture game: {outcome:?}");
    };
    Ok(game)
}

async fn setup_game(
    white_name: &str,
    black_name: &str,
    game_type: GameType,
    history: &str,
    conn: &mut DbConn<'_>,
) -> Game {
    let white = User::create(
        NewUser::new(white_name, "password", &format!("{white_name}@test.com")).unwrap(),
        conn,
    )
    .await
    .unwrap();
    let black = User::create(
        NewUser::new(black_name, "password", &format!("{black_name}@test.com")).unwrap(),
        conn,
    )
    .await
    .unwrap();

    let now = Utc::now();
    let time_left = Some(60_000_000_000_i64);
    let turn = history.split_terminator(';').count() as i32;

    Game::create(
        NewGame {
            nanoid: nanoid::nanoid!(12),
            current_player_id: if turn % 2 == 0 { white.id } else { black.id },
            black_id: black.id,
            finished: false,
            game_status: GameStatus::InProgress.to_string(),
            game_type: game_type.to_string(),
            history: history.to_string(),
            game_control_history: String::new(),
            rated: true,
            tournament_queen_rule: false,
            turn,
            white_id: white.id,
            white_rating: None,
            black_rating: None,
            white_rating_change: None,
            black_rating_change: None,
            created_at: now,
            updated_at: now,
            time_mode: TimeMode::RealTime.to_string(),
            time_base: Some(60),
            time_increment: Some(0),
            last_interaction: Some(now),
            black_time_left: time_left,
            white_time_left: time_left,
            speed: GameSpeed::Bullet.to_string(),
            hashes: Vec::new(),
            conclusion: Conclusion::Unknown.to_string(),
            tournament_id: None,
            tournament_game_result: TournamentGameResult::Unknown.to_string(),
            game_start: GameStart::Moves.to_string(),
            move_times: Vec::new(),
            timeout_at: time_left.map(|nanos| now + chrono::Duration::nanoseconds(nanos)),
            tournament_slot_id: None,
            arena_ordinal: None,
            white_berserked: false,
            black_berserked: false,
            arena_move_due_at: None,
        },
        conn,
    )
    .await
    .unwrap()
}
