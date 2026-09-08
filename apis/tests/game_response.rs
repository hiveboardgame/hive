#![cfg(feature = "ssr")]

mod common;

use apis::responses::GameResponse;
use chrono::Utc;
use db_lib::{
    get_conn,
    models::{Game, NewGame, NewUser, User},
};
use hive_lib::{Bug, GameStatus, GameType};
use shared_types::{Conclusion, GameSpeed, GameStart, TimeMode, TournamentGameResult};

/// `wL` and `wM` are not in the Base inventory, so this history only replays against the
/// game's stored `game_type`.
const MLP_HISTORY: &str = r"wL ;bL wL-;wQ /wL;bQ bL/;wM \wL;bP bQ/;";
const BASE_HISTORY: &str = r"wA1 ;bA1 wA1-;wQ /wA1;bQ bA1/;wG1 \wA1;bG1 bQ/;";

#[tokio::test(flavor = "multi_thread")]
async fn from_model_replays_an_expansion_game() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let game = setup_game("mlp_w", "mlp_b", GameType::MLP, MLP_HISTORY, &mut conn).await;

    let response = GameResponse::from_model(&game, &mut conn)
        .await
        .expect("an expansion game becomes a response");

    assert_eq!(response.game_type, GameType::MLP);
    assert_eq!(response.turn, 6);
    assert_eq!(response.history.len(), 6);
    assert!(response.reserve_white.contains_key(&Bug::Pillbug));
}

#[tokio::test(flavor = "multi_thread")]
async fn from_model_replays_a_base_game() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let game = setup_game("base_w", "base_b", GameType::Base, BASE_HISTORY, &mut conn).await;

    let response = GameResponse::from_model(&game, &mut conn)
        .await
        .expect("a base game becomes a response");

    assert_eq!(response.game_type, GameType::Base);
    assert_eq!(response.turn, 6);
    assert!(!response.reserve_white.contains_key(&Bug::Pillbug));
}

/// The batch propagates the first replay error, so one expansion game takes the whole games
/// list down rather than just its own row.
#[tokio::test(flavor = "multi_thread")]
async fn from_games_batch_replays_a_mixed_batch() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let base = setup_game("mix_bw", "mix_bb", GameType::Base, BASE_HISTORY, &mut conn).await;
    let mlp = setup_game("mix_mw", "mix_mb", GameType::MLP, MLP_HISTORY, &mut conn).await;

    let responses = GameResponse::from_games_batch(vec![base.clone(), mlp.clone()], &mut conn)
        .await
        .expect("a batch of mixed game types becomes responses");

    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0].uuid, base.id);
    assert_eq!(responses[0].game_type, GameType::Base);
    assert_eq!(responses[1].uuid, mlp.id);
    assert_eq!(responses[1].game_type, GameType::MLP);
    assert!(responses.iter().all(|response| response.turn == 6));
}

async fn setup_game(
    white_name: &str,
    black_name: &str,
    game_type: GameType,
    history: &str,
    conn: &mut db_lib::DbConn<'_>,
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
        },
        conn,
    )
    .await
    .unwrap()
}
