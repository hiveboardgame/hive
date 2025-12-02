use crate::common::{
    log_operation_complete, log_operation_start, log_progress, setup_database,
    TEST_USER_EMAIL_PATTERN, TEST_USER_USERNAME_PATTERN,
};
use anyhow::{Context, Result};
use chrono::Utc;
use db_lib::models::{Game, NewGame, NewUser, User};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use hive_lib::{GameControl, GameType, History, Piece, Position, State};
use log::info;
use nanoid::nanoid;
use rand::prelude::*;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use shared_types::{GameSpeed, TimeMode};
use std::str::FromStr;
use uuid::Uuid;

const MAX_MOVES_PER_GAME: usize = 100;
const MOVE_PROBABILITY: f64 = 0.3;
const RESIGNATION_PROBABILITY: f64 = 0.5;
const QUEEN_MUST_BE_PLAYED_BY_TURN: usize = 2;

pub async fn run_seed_database(
    num_users: usize,
    games_per_user: usize,
    database_url: Option<String>,
) -> Result<()> {
    if num_users == 0 {
        return Err(anyhow::anyhow!("num_users must be greater than 0"));
    }
    if games_per_user == 0 {
        return Err(anyhow::anyhow!("games_per_user must be greater than 0"));
    }

    log_operation_start("database seeding");

    let mut conn = setup_database(database_url)
        .await
        .context("Failed to setup database connection")?;
    info!("Connected to database");

    let (created_users, created_games) =
        execute_seeding_transaction(&mut conn, num_users, games_per_user).await?;

    log_operation_complete("Database seeding", created_users + created_games, 0);
    Ok(())
}

async fn execute_seeding_transaction(
    conn: &mut db_lib::DbConn<'_>,
    num_users: usize,
    games_per_user: usize,
) -> Result<(usize, usize)> {
    info!("Beginning seeding");

    let (created_users, created_games) = conn
        .transaction(move |conn| {
            let users_to_create = num_users;
            let games_per_user_count = games_per_user;
            Box::pin(async move {
                info!("Creating {users_to_create} test users");
                let user_ids = create_test_users(conn, users_to_create).await.map_err(|e| {
                    log::error!("Failed to create test users: {e}");
                    diesel::result::Error::RollbackTransaction
                })?;
                info!("Done, we now have {} users", user_ids.len());

                info!("Playing {games_per_user_count} games per user");
                let total_games = play_test_games(conn, &user_ids, games_per_user_count)
                    .await
                    .map_err(|e| {
                        log::error!("Failed to play test games: {e}");
                        diesel::result::Error::RollbackTransaction
                    })?;
                info!("Done, we now have {total_games} total games");

                Ok::<(usize, usize), diesel::result::Error>((user_ids.len(), total_games))
            })
        })
        .await
        .map_err(|e| {
            log::error!("Seeding failed and was rolled back: {e}");
            e
        })?;

    Ok((created_users, created_games))
}

async fn create_test_users(
    conn: &mut db_lib::DbConn<'_>,
    num_users: usize,
) -> Result<Vec<Uuid>> {
    let mut user_ids = Vec::new();

    for i in 1..=num_users {
        let username = format!("testuser{i}");
        let email = format!("test{i}@example.com");
        let password_hash = String::from("hivegame");

        let new_user = NewUser::new(&username, &password_hash, &email).map_err(|e| {
            log::error!("Failed to create NewUser for {username}: {e}");
            e
        })?;

        let user = User::create(new_user, conn).await.map_err(|e| {
            log::error!("Failed to create User {username} in database: {e}");
            e
        })?;

        user_ids.push(user.id);
        info!("Created user: {username} (ID: {})", user.id);
    }

    Ok(user_ids)
}

async fn play_test_games(
    conn: &mut db_lib::DbConn<'_>,
    user_ids: &[Uuid],
    games_per_user: usize,
) -> Result<usize> {
    let mut total_games = 0;
    let total_expected_games = user_ids.len() * games_per_user;

    for (user_idx, user_id) in user_ids.iter().enumerate() {
        let user_num = user_idx + 1;
        let total_users = user_ids.len();
        info!(
            "Creating games for user {user_id} ({user_num}/{total_users})"
        );

        for game_idx in 0..games_per_user {
            let (game_speed, white_id, black_id) = {
                let seed = (total_games as u64).wrapping_mul(17).wrapping_add(game_idx as u64);
                let mut rng = SmallRng::seed_from_u64(seed);
                let opp_id = get_random_opponent(user_id, user_ids, &mut rng)?;
                let speed = get_random_game_speed(&mut rng);
                let (w_id, b_id) = if rng.random_bool(0.5) {
                    (*user_id, opp_id)
                } else {
                    (opp_id, *user_id)
                };
                (speed, w_id, b_id)
            };

            let game_num = total_games + 1;
            let current_game = game_idx + 1;
            info!(
                "Creating game {game_num} for user {user_id} (game {current_game}/{games_per_user})"
            );

            let game = create_game(conn, white_id, black_id, game_speed)
                .await
                .with_context(|| format!("Failed to create game {}", total_games + 1))?;

            play_game(&game, conn).await
                .with_context(|| format!("Failed to play game {}", game.nanoid))?;

            resign_game(&game, conn).await
                .with_context(|| format!("Failed to resign game {}", game.nanoid))?;

            total_games += 1;
            log_progress(total_games, total_expected_games, "Creating games");
        }
    }

    Ok(total_games)
}

fn get_random_opponent(current_user: &Uuid, all_users: &[Uuid], rng: &mut impl Rng) -> Result<Uuid> {
    let available_opponents: Vec<Uuid> = all_users
        .iter()
        .filter(|&&id| id != *current_user)
        .copied()
        .collect();

    available_opponents
        .choose(rng)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("No available opponents for user {}", current_user))
}

fn get_random_game_speed(rng: &mut impl Rng) -> GameSpeed {
    let game_speeds = [
        GameSpeed::Bullet,
        GameSpeed::Blitz,
        GameSpeed::Rapid,
        GameSpeed::Classic,
        GameSpeed::Correspondence,
    ];
    *game_speeds.choose(rng).expect("Game speeds array should never be empty")
}

async fn create_game(
    conn: &mut db_lib::DbConn<'_>,
    white_id: Uuid,
    black_id: Uuid,
    game_speed: GameSpeed,
) -> Result<Game> {
    let nanoid = nanoid!(12);

    let (time_base, time_increment) = match game_speed {
        GameSpeed::Bullet => (Some(60), Some(0)),
        GameSpeed::Blitz => (Some(180), Some(0)),
        GameSpeed::Rapid => (Some(600), Some(0)),
        GameSpeed::Classic => (Some(1800), Some(0)),
        GameSpeed::Correspondence => (Some(86400), Some(0)),
        _ => (None, None),
    };

    let time_mode = if time_base.is_some() {
        TimeMode::RealTime
    } else {
        TimeMode::Untimed
    };

    let new_game = NewGame {
        nanoid: nanoid.clone(),
        current_player_id: white_id,
        black_id,
        finished: false,
        game_status: "NotStarted".to_string(),
        game_type: GameType::MLP.to_string(),
        history: String::new(),
        game_control_history: String::new(),
        rated: true,
        tournament_queen_rule: true,
        turn: 0,
        white_id,
        white_rating: None,
        black_rating: None,
        white_rating_change: None,
        black_rating_change: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        time_mode: time_mode.to_string(),
        time_base,
        time_increment,
        last_interaction: None,
        black_time_left: time_base.map(|base| (base as u64 * 1_000_000_000) as i64),
        white_time_left: time_base.map(|base| (base as u64 * 1_000_000_000) as i64),
        speed: game_speed.to_string(),
        hashes: vec![],
        conclusion: "Unknown".to_string(),
        tournament_id: None,
        tournament_game_result: "Unknown".to_string(),
        game_start: "Moves".to_string(),
        move_times: vec![],
    };

    let game = Game::create(new_game, conn).await?;

    Ok(game)
}

async fn play_game(
    game: &Game,
    conn: &mut db_lib::DbConn<'_>,
) -> Result<()> {
    let started_game = game.start(conn).await
        .with_context(|| format!("Failed to start game {}", game.nanoid))?;

    let history = History::new_from_str(&started_game.history)
        .with_context(|| format!("Failed to parse history for game {}", started_game.nanoid))?;

    let mut state = State::new_from_history(&history)
        .with_context(|| format!("Failed to create state from history for game {}", started_game.nanoid))?;

    state.game_type = GameType::from_str(&started_game.game_type)
        .with_context(|| format!("Failed to parse game type '{}' for game {}", started_game.game_type, started_game.nanoid))?;

    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let mut rng = SmallRng::seed_from_u64(seed);
    for move_number in 0..MAX_MOVES_PER_GAME {
        if matches!(state.game_status, hive_lib::GameStatus::Finished(_)) {
            info!(
                "Game {} finished after {move_number} moves",
                started_game.nanoid
            );
            break;
        }

        let move_made = attempt_game_move(&mut state, &started_game.nanoid, move_number + 1, &mut rng)?;

        if !move_made {
            let turn_num = move_number + 1;
            info!(
                "Game {} - Turn {turn_num}: No valid move could be made",
                started_game.nanoid
            );
            break;
        }
    }

    if state.turn > 0 {
        info!("Updating game state in database...");
        started_game
            .update_gamestate(&state, 0.0, conn)
            .await
            .with_context(|| format!("Failed to update game state for game {}", started_game.nanoid))?;
    }

    info!(
        "Game {} completed with {} moves",
        started_game.nanoid, state.turn
    );
    Ok(())
}

async fn resign_game(
    game: &Game,
    conn: &mut db_lib::DbConn<'_>,
) -> Result<()> {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let mut rng = SmallRng::seed_from_u64(seed);
    let resigning_user = if rng.random_bool(RESIGNATION_PROBABILITY) {
            game.white_id
        } else {
            game.black_id
    };

    let user_color = game.user_color(resigning_user)
        .ok_or_else(|| anyhow::anyhow!(
            "User {} is not a player in game {}",
            resigning_user, game.nanoid
        ))?;

    let game_control = GameControl::Resign(user_color);

    info!(
        "Resigning game {} for user {} ({user_color:?})",
        game.nanoid, resigning_user
    );
    game.resign(&game_control, conn).await
        .with_context(|| format!("Failed to resign game {}", game.nanoid))?;

    info!("Game {} resigned by user {resigning_user}", game.nanoid);

    Ok(())
}

fn attempt_game_move(
    state: &mut State,
    game_nanoid: &str,
    move_number: usize,
    rng: &mut impl Rng,
) -> Result<bool> {
    let current_color = state.turn_color;

    let available_spawns: Vec<Position> =
        state.board.spawnable_positions(current_color).collect();
    let mut reserve = state.reserve(current_color);

    if state.turn < QUEEN_MUST_BE_PLAYED_BY_TURN {
        reserve.remove(&hive_lib::Bug::Queen);
    }

    let queen_has_been_played = state.board.queen_played(current_color);
    let available_moves = if queen_has_been_played {
        state.board.moves(current_color)
    } else {
        std::collections::HashMap::new()
    };

    let moves_count = available_moves.len();
    let spawns_count = available_spawns.len();
    let reserve_count = reserve.values().map(|v| v.len()).sum::<usize>();
    info!("Game {game_nanoid} - Turn {move_number} (game turn {}): {current_color:?} to play - {moves_count} moves, {spawns_count} spawn positions, {reserve_count} pieces in reserve",
          state.turn);

    if available_moves.is_empty() && (available_spawns.is_empty() || reserve.is_empty()) {
        info!(
            "Game {game_nanoid} - No valid moves or spawns available, ending game"
        );
        return Ok(false);
    }

    let should_attempt_move = queen_has_been_played && !available_moves.is_empty() && rng.random_bool(MOVE_PROBABILITY);

    if should_attempt_move {
        let move_entries: Vec<_> = available_moves.iter().collect();
        if let Some(((piece, from_pos), target_positions)) = move_entries.choose(rng) {
            if let Some(target_pos) = target_positions.choose(rng) {
                if state.play_turn_from_position(*piece, *target_pos).is_ok() {
                    info!(
                        "Game {game_nanoid} - Turn {move_number}: Move {piece} from {from_pos} to {target_pos}"
                    );
                    return Ok(true);
                }
            }
        }
    }

    if !available_spawns.is_empty() && !reserve.is_empty() {
        let reserve_entries: Vec<_> = reserve
            .iter()
            .flat_map(|(bug, pieces)| pieces.iter().map(move |p| (*bug, p.clone())))
            .collect();

        if reserve_entries.is_empty() {
            info!(
                "Game {game_nanoid} - Turn {move_number}: Reserve is empty after filtering"
            );
        } else if let Some((_bug, piece_str)) = reserve_entries.choose(rng) {
            match piece_str.parse::<Piece>() {
                Ok(piece) => {
                    if let Some(spawn_pos) = available_spawns.choose(rng) {
                        match state.play_turn_from_position(piece, *spawn_pos) {
                            Ok(()) => {
                                info!(
                                    "Game {game_nanoid} - Turn {move_number}: Spawn {piece} at {spawn_pos}"
                                );
                                return Ok(true);
                            }
                            Err(e) => {
                                log::warn!(
                                    "Game {game_nanoid} - Turn {move_number}: Failed to spawn {piece} at {spawn_pos}: {e:?}"
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Game {game_nanoid} - Turn {move_number}: Failed to parse piece '{piece_str}': {e:?}"
                    );
                }
            }
        }
    }

    Ok(false)
}

pub async fn cleanup_test_data(database_url: Option<String>) -> Result<()> {
    log_operation_start("test data cleanup");

    info!("Setting up database connection...");
    let mut conn = setup_database(database_url)
        .await
        .context("Failed to setup database connection")?;
    info!("Connected to database");

    let test_users = find_test_users(&mut conn).await?;
    let test_user_count = test_users.len();
    info!("Found {test_user_count} test users to clean up");

    if test_users.is_empty() {
        info!("No test users found, nothing to clean up");
        log_operation_complete("Test data cleanup", 0, 0);
        return Ok(());
    }

    let deleted_games = delete_games_involving_test_users(&mut conn, &test_users).await?;
    info!("Deleted {deleted_games} games");

    let deleted_users = delete_test_users(&mut conn).await?;

    info!("Deleted {deleted_users} test users");

    let total_deleted = deleted_games + deleted_users;
    log_operation_complete("Test data cleanup", total_deleted, 0);

    info!("Cleanup completed successfully!");
    info!("Summary:");
    info!("  - Deleted {deleted_games} games");
    info!(
        "  - Deleted {deleted_users} test users (ratings and game_users entries cascaded)"
    );
    info!("  - Total records deleted: {total_deleted}");

    Ok(())
}

async fn find_test_users(conn: &mut db_lib::DbConn<'_>) -> Result<Vec<Uuid>> {
    info!("Finding test users...");
    db_lib::schema::users::table
        .filter(
            db_lib::schema::users::username
                .like(TEST_USER_USERNAME_PATTERN)
                .and(db_lib::schema::users::email.like(TEST_USER_EMAIL_PATTERN)),
        )
        .select(db_lib::schema::users::id)
        .load::<Uuid>(conn)
        .await
        .context("Failed to find test users")
}

async fn delete_games_involving_test_users(
    conn: &mut db_lib::DbConn<'_>,
    test_users: &[Uuid],
) -> Result<usize> {
    info!("Deleting games involving test users...");
    diesel::delete(
        db_lib::schema::games::table.filter(
            db_lib::schema::games::white_id
                .eq_any(test_users)
                .or(db_lib::schema::games::black_id.eq_any(test_users)),
        ),
    )
    .execute(conn)
    .await
    .context("Failed to delete games involving test users")
}

async fn delete_test_users(conn: &mut db_lib::DbConn<'_>) -> Result<usize> {
    info!("Deleting test users...");
    diesel::delete(
        db_lib::schema::users::table.filter(
            db_lib::schema::users::username
                .like(TEST_USER_USERNAME_PATTERN)
                .and(db_lib::schema::users::email.like(TEST_USER_EMAIL_PATTERN)),
        ),
    )
    .execute(conn)
    .await
    .context("Failed to delete test users")
}
