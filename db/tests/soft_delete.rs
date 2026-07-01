mod common;

use chrono::Utc;
use db_lib::{
    db_error::DbError,
    get_conn,
    models::{Game, NewGame, NewSchedule, NewTournament, NewUser, Schedule, Tournament, User},
    schema::{games, tournaments, tournaments_organizers, tournaments_users, users},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use hudsoni::{Color, GameResult, GameStatus, GameType};
use shared_types::{
    Conclusion,
    GameId,
    GameSpeed,
    GameStart,
    ScoringMode,
    StartMode,
    Tiebreaker,
    TimeMode,
    TournamentGameResult,
    TournamentMode,
    TournamentStatus,
};

const DELETED_USERNAME_PREFIX: &str = "deleted_user_";

#[tokio::test(flavor = "multi_thread")]
async fn soft_delete_anonymizes_user_and_rejects_tombstone_login() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let user = create_user("delete_me", &mut conn).await;
    diesel::update(users::table.find(user.id))
        .set(users::admin.eq(true))
        .execute(&mut conn)
        .await
        .expect("mark user as admin");

    let report = user
        .soft_delete("replacement-password-hash", &mut conn)
        .await
        .expect("soft delete user");

    assert!(report.deleted_games.is_empty());
    assert!(report.resigned_games.is_empty());

    let deleted = User::find_by_uuid(&user.id, &mut conn)
        .await
        .expect("load historical deleted user");
    assert!(deleted.deleted);
    assert_eq!(
        deleted.username,
        format!("{DELETED_USERNAME_PREFIX}{}", user.id)
    );
    assert_eq!(deleted.normalized_username, deleted.username);
    assert_eq!(
        deleted.email,
        format!("{}@deleted.invalid", deleted.username)
    );
    assert_eq!(deleted.password, "replacement-password-hash");
    assert!(!deleted.admin);

    assert!(!User::username_exists("delete_me", &mut conn)
        .await
        .expect("old username is available"));
    assert!(!User::username_exists("deleted_user_999", &mut conn)
        .await
        .expect("unknown tombstone username is absent"));
    assert!(matches!(
        NewUser::new(
            "deleted_user_999",
            "new-password",
            "deleted-user-999@example.com"
        )
        .expect_err("deleted username pattern is reserved"),
        DbError::InvalidInput { .. }
    ));
    assert!(matches!(
        User::find_for_login(&deleted.username, &mut conn)
            .await
            .expect_err("deleted user cannot log in"),
        DbError::NotFound { .. }
    ));
    assert!(matches!(
        User::find_for_login(&deleted.email, &mut conn)
            .await
            .expect_err("deleted user cannot log in by tombstone email"),
        DbError::NotFound { .. }
    ));

    let replacement = NewUser::new("delete_me", "new-password", "delete_me@example.com")
        .expect("old identity can be reused");
    User::create(replacement, &mut conn)
        .await
        .expect("create replacement account");
}

#[tokio::test(flavor = "multi_thread")]
async fn soft_delete_aborts_early_games_resigns_ready_tournament_games_and_deletes_schedules() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let deleting_user = create_user("soon_gone", &mut conn).await;
    let opponent = create_user("opponent", &mut conn).await;
    let organizer = create_user("organizer", &mut conn).await;

    let early_game = create_abortable_game(deleting_user.id, opponent.id, &mut conn).await;
    let tournament = create_realtime_tournament(organizer.id, &mut conn).await;
    let ready_game = Game::create(
        NewGame::new_from_tournament(deleting_user.id, opponent.id, &tournament),
        &mut conn,
    )
    .await
    .expect("insert ready tournament game");
    let schedule = NewSchedule::new(
        deleting_user.id,
        &GameId(ready_game.nanoid.clone()),
        Utc::now(),
        &mut conn,
    )
    .await
    .expect("create schedule details");
    Schedule::create(schedule, deleting_user.id, &mut conn)
        .await
        .expect("insert schedule");

    let report = deleting_user
        .soft_delete("replacement-password-hash", &mut conn)
        .await
        .expect("soft delete user");

    assert_eq!(report.deleted_games.len(), 1);
    assert_eq!(report.deleted_games[0].nanoid, early_game.nanoid);
    assert_eq!(report.resigned_games.len(), 1);
    assert_eq!(report.resigned_games[0].nanoid, ready_game.nanoid.clone());

    let early_exists = games::table
        .find(early_game.id)
        .select(games::id)
        .first::<uuid::Uuid>(&mut conn)
        .await;
    assert!(matches!(early_exists, Err(diesel::result::Error::NotFound)));

    let resigned = Game::find_by_uuid(&ready_game.id, &mut conn)
        .await
        .expect("load resigned tournament game");
    assert!(resigned.finished);
    assert_eq!(resigned.conclusion, Conclusion::Resigned.to_string());
    assert_eq!(
        resigned.game_status,
        GameStatus::Finished(GameResult::Winner(Color::Black)).to_string()
    );
    assert!(resigned.white_rating.is_some());
    assert!(resigned.black_rating.is_some());
    assert!(resigned.white_rating_change.is_some());
    assert!(resigned.black_rating_change.is_some());

    let schedules = Schedule::all_from_nanoid(ready_game.nanoid, &mut conn)
        .await
        .expect("load schedules");
    assert!(schedules.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn soft_delete_deletes_unstarted_tournaments_organized_by_user_but_keeps_started() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let deleting_organizer = create_user("deleted_organizer", &mut conn).await;
    let player = create_user("organized_player", &mut conn).await;

    let unstarted = create_double_swiss_tournament(deleting_organizer.id, &mut conn).await;
    unstarted
        .join(&player.id, &mut conn)
        .await
        .expect("join unstarted tournament");
    let started = create_realtime_tournament(deleting_organizer.id, &mut conn).await;

    deleting_organizer
        .soft_delete("replacement-password-hash", &mut conn)
        .await
        .expect("soft delete organizer");

    let deleted_tournament = tournaments::table
        .find(unstarted.id)
        .select(tournaments::id)
        .first::<uuid::Uuid>(&mut conn)
        .await;
    assert!(matches!(
        deleted_tournament,
        Err(diesel::result::Error::NotFound)
    ));

    let deleted_player_row = tournaments_users::table
        .find((unstarted.id, player.id))
        .select(tournaments_users::user_id)
        .first::<uuid::Uuid>(&mut conn)
        .await;
    assert!(matches!(
        deleted_player_row,
        Err(diesel::result::Error::NotFound)
    ));

    let deleted_organizer_row = tournaments_organizers::table
        .find((unstarted.id, deleting_organizer.id))
        .select(tournaments_organizers::organizer_id)
        .first::<uuid::Uuid>(&mut conn)
        .await;
    assert!(matches!(
        deleted_organizer_row,
        Err(diesel::result::Error::NotFound)
    ));

    let kept_started = Tournament::find_by_uuid(started.id, &mut conn)
        .await
        .expect("started tournament remains");
    assert_eq!(
        kept_started.status,
        TournamentStatus::InProgress.to_string()
    );
    tournaments_organizers::table
        .find((started.id, deleting_organizer.id))
        .select(tournaments_organizers::organizer_id)
        .first::<uuid::Uuid>(&mut conn)
        .await
        .expect("started tournament organizer remains for admin handling");
}

#[tokio::test(flavor = "multi_thread")]
async fn soft_deleted_double_swiss_player_is_not_paired_in_next_round() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.expect("get connection");
    let organizer = create_user("swiss_organizer", &mut conn).await;
    let player_one = create_user("swiss_one", &mut conn).await;
    let player_two = create_user("swiss_two", &mut conn).await;
    let player_three = create_user("swiss_three", &mut conn).await;
    let deleting_user = create_user("swiss_deleted", &mut conn).await;
    let bye_player = create_swiss_bye_player(&mut conn).await;

    let tournament = create_double_swiss_tournament(organizer.id, &mut conn).await;
    for player in [&player_one, &player_two, &player_three, &deleting_user] {
        tournament
            .join(&player.id, &mut conn)
            .await
            .expect("join tournament");
    }

    let (tournament, first_round_games, _) = tournament
        .start_by_organizer(&organizer.id, &mut conn)
        .await
        .expect("start tournament");
    assert_eq!(first_round_games.len(), 4);
    for game in first_round_games {
        game.adjudicate_tournament_result(
            &organizer.id,
            &TournamentGameResult::Winner(Color::White),
            &mut conn,
        )
        .await
        .expect("finish first round game");
    }

    deleting_user
        .soft_delete("replacement-password-hash", &mut conn)
        .await
        .expect("soft delete user");

    let next_round_games = tournament
        .swiss_create_next_round(&organizer.id, &mut conn)
        .await
        .expect("create next round");

    assert_eq!(next_round_games.len(), 4);
    assert!(next_round_games
        .iter()
        .all(|game| game.white_id != deleting_user.id && game.black_id != deleting_user.id));
    assert!(next_round_games
        .iter()
        .any(|game| game.white_id == bye_player.id || game.black_id == bye_player.id));

    tournaments_users::table
        .find((tournament.id, bye_player.id))
        .select(tournaments_users::user_id)
        .first::<uuid::Uuid>(&mut conn)
        .await
        .expect("bye player is available for standings display");
}

async fn create_user(username: &str, conn: &mut db_lib::DbConn<'_>) -> User {
    let new_user = NewUser::new(username, "password", &format!("{username}@example.com"))
        .expect("create new user fixture");
    User::create(new_user, conn).await.expect("insert user")
}

async fn create_swiss_bye_player(conn: &mut db_lib::DbConn<'_>) -> User {
    let new_user = NewUser::new("SwissByePlayer", "password", "swiss-bye-player@example.com")
        .expect("create Swiss bye user fixture");
    User::create(new_user, conn)
        .await
        .expect("insert Swiss bye user")
}

async fn create_abortable_game(
    white_id: uuid::Uuid,
    black_id: uuid::Uuid,
    conn: &mut db_lib::DbConn<'_>,
) -> Game {
    let now = Utc::now();
    let time_left = Some(60 * 1_000_000_000_i64);
    Game::create(
        NewGame {
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
            turn: 1,
            white_id,
            white_rating: None,
            black_rating: None,
            white_rating_change: None,
            black_rating_change: None,
            created_at: now,
            updated_at: now,
            time_mode: TimeMode::RealTime.to_string(),
            time_base: Some(60),
            time_increment: Some(0),
            last_interaction: None,
            black_time_left: time_left,
            white_time_left: time_left,
            speed: GameSpeed::Bullet.to_string(),
            hashes: Vec::new(),
            conclusion: Conclusion::Unknown.to_string(),
            tournament_id: None,
            tournament_game_result: TournamentGameResult::Unknown.to_string(),
            game_start: GameStart::Moves.to_string(),
            move_times: Vec::new(),
            timeout_at: None,
        },
        conn,
    )
    .await
    .expect("insert game")
}

async fn create_double_swiss_tournament(
    organizer_id: uuid::Uuid,
    conn: &mut db_lib::DbConn<'_>,
) -> Tournament {
    Tournament::create(
        organizer_id,
        &NewTournament {
            nanoid: nanoid::nanoid!(11),
            name: "Soft delete Double Swiss tournament".to_string(),
            description: String::new(),
            scoring: ScoringMode::Game.to_string(),
            tiebreaker: vec![Some(Tiebreaker::RawPoints.to_string())],
            seats: 4,
            min_seats: 2,
            rounds: 2,
            invite_only: false,
            mode: TournamentMode::DoubleSwiss.to_string(),
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
    .expect("insert Double Swiss tournament")
}

async fn create_realtime_tournament(
    organizer_id: uuid::Uuid,
    conn: &mut db_lib::DbConn<'_>,
) -> Tournament {
    Tournament::create(
        organizer_id,
        &NewTournament {
            nanoid: nanoid::nanoid!(11),
            name: "Soft delete tournament".to_string(),
            description: String::new(),
            scoring: ScoringMode::Game.to_string(),
            tiebreaker: vec![Some(Tiebreaker::RawPoints.to_string())],
            seats: 2,
            min_seats: 2,
            rounds: 1,
            invite_only: false,
            mode: "RR".to_string(),
            time_mode: TimeMode::RealTime.to_string(),
            time_base: Some(60),
            time_increment: Some(0),
            band_upper: None,
            band_lower: None,
            start_mode: StartMode::Manual.to_string(),
            starts_at: None,
            ends_at: None,
            started_at: Some(Utc::now()),
            round_duration: None,
            status: TournamentStatus::InProgress.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            series: None,
        },
        conn,
    )
    .await
    .expect("insert tournament")
}
