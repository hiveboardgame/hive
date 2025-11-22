use crate::common::{log_operation_complete, log_operation_start, log_progress, setup_database};
use anyhow::{Context, Result};
use chrono::Utc;
use db_lib::models::{Game, NewGame, NewUser, User};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use hive_lib::{GameControl, GameType, History, Piece, Position, State};
use log::info;
use nanoid::nanoid;
use rand::prelude::*;
use rand::{rng, Rng};
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
                info!("Creating {} test users", users_to_create);
                let user_ids = create_test_users(conn, users_to_create).await.map_err(|e| {
                    log::error!("Failed to create test users: {}", e);
                    diesel::result::Error::RollbackTransaction
                })?;
                info!("Done, we now have {} users", user_ids.len());

                info!("Playing {} games per user", games_per_user_count);
                let total_games = play_test_games(conn, &user_ids, games_per_user_count)
                    .await
                    .map_err(|e| {
                        log::error!("Failed to play test games: {}", e);
                        diesel::result::Error::RollbackTransaction
                    })?;
                info!("Done, we now have {} total games", total_games);

                Ok::<(usize, usize), diesel::result::Error>((user_ids.len(), total_games))
            })
        })
        .await
        .map_err(|e| {
            log::error!("Seeding failed and was rolled back: {}", e);
            e
        })?;

    Ok((created_users, created_games))
}

async fn create_test_users(
    conn: &mut db_lib::DbConn<'_>,
    num_users: usize,
) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
    let mut user_ids = Vec::new();

    for i in 1..=num_users {
        let username = format!("testuser{}", i);
        let email = format!("test{}@example.com", i);
        let password_hash = String::from("hivegame");

        let new_user = NewUser::new(&username, &password_hash, &email).map_err(|e| {
            log::error!("Failed to create NewUser for {}: {}", username, e);
            e
        })?;

        let user = User::create(new_user, conn).await.map_err(|e| {
            log::error!("Failed to create User {} in database: {}", username, e);
            e
        })?;

        user_ids.push(user.id);
        info!("Created user: {} (ID: {})", username, user.id);
    }

    Ok(user_ids)
}

async fn play_test_games(
    conn: &mut db_lib::DbConn<'_>,
    user_ids: &[Uuid],
    games_per_user: usize,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut total_games = 0;
    let total_expected_games = user_ids.len() * games_per_user;

    for (user_idx, user_id) in user_ids.iter().enumerate() {
        info!(
            "Creating games for user {} ({}/{})",
            user_id,
            user_idx + 1,
            user_ids.len()
        );

        for game_idx in 0..games_per_user {
            let opponent_id = get_random_opponent(user_id, user_ids);
            let game_speed = get_random_game_speed();

            let (white_id, black_id) = {
                let mut rng = rng();
                if rng.random_bool(0.5) {
                    (*user_id, opponent_id)
                } else {
                    (opponent_id, *user_id)
                }
            };

            info!(
                "Creating game {} for user {} (game {}/{})",
                total_games + 1,
                user_id,
                game_idx + 1,
                games_per_user
            );

            let game = create_game(conn, white_id, black_id, game_speed)
                .await
                .map_err(|e| {
                    log::error!("Failed to create game {}: {}", total_games + 1, e);
                    e
                })?;

            play_game(&game, conn).await.map_err(|e| {
                log::error!("Failed to play game {}: {}", game.nanoid, e);
                e
            })?;

            resign_game(&game, conn).await.map_err(|e| {
                log::error!("Failed to resign game {}: {}", game.nanoid, e);
                e
            })?;

            total_games += 1;
            log_progress(total_games, total_expected_games, "Creating games");
        }
    }

    Ok(total_games)
}

fn get_random_opponent(current_user: &Uuid, all_users: &[Uuid]) -> Uuid {
    let available_opponents: Vec<Uuid> = all_users
        .iter()
        .filter(|&&id| id != *current_user)
        .copied()
        .collect();

    let mut rng = rng();
    *available_opponents.choose(&mut rng).unwrap()
}

fn get_random_game_speed() -> GameSpeed {
    let game_speeds = [
        GameSpeed::Bullet,
        GameSpeed::Blitz,
        GameSpeed::Rapid,
        GameSpeed::Classic,
        GameSpeed::Correspondence,
    ];
    let mut rng = rng();
    *game_speeds.choose(&mut rng).unwrap()
}

async fn create_game(
    conn: &mut db_lib::DbConn<'_>,
    white_id: Uuid,
    black_id: Uuid,
    game_speed: GameSpeed,
) -> Result<Game, Box<dyn std::error::Error>> {
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
) -> Result<(), Box<dyn std::error::Error>> {
    let started_game = game.start(conn).await.map_err(|e| {
        log::error!("Failed to start game {}: {}", game.nanoid, e);
        e
    })?;

    let history = History::new_from_str(&started_game.history).map_err(|e| {
        log::error!(
            "Failed to parse history for game {}: {}",
            started_game.nanoid,
            e
        );
        e
    })?;

    let mut state = State::new_from_history(&history).map_err(|e| {
        log::error!(
            "Failed to create state from history for game {}: {}",
            started_game.nanoid,
            e
        );
        e
    })?;

    state.game_type = GameType::from_str(&started_game.game_type).map_err(|e| {
        log::error!(
            "Failed to parse game type '{}' for game {}: {}",
            started_game.game_type,
            started_game.nanoid,
            e
        );
        e
    })?;

    for move_number in 0..MAX_MOVES_PER_GAME {
        if matches!(state.game_status, hive_lib::GameStatus::Finished(_)) {
            info!(
                "Game {} finished after {} moves",
                started_game.nanoid, move_number
            );
            break;
        }

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

        info!("Game {} - Turn {} (game turn {}): {:?} to play - {} moves, {} spawn positions, {} pieces in reserve",
              started_game.nanoid, move_number + 1, state.turn, current_color,
              available_moves.len(), available_spawns.len(),
              reserve.values().map(|v| v.len()).sum::<usize>());

        if available_moves.is_empty() && (available_spawns.is_empty() || reserve.is_empty()) {
            info!(
                "Game {} - No valid moves or spawns available, ending game",
                started_game.nanoid
            );
            break;
        }

        let mut move_made = false;

        let should_attempt_move = queen_has_been_played && !available_moves.is_empty() && {
            let mut rng = rng();
            rng.random_bool(MOVE_PROBABILITY)
        };

        if should_attempt_move {
            let move_entries: Vec<_> = available_moves.iter().collect();
            if let Some(((piece, from_pos), target_positions)) = {
                let mut rng = rng();
                move_entries.choose(&mut rng)
            } {
                if let Some(target_pos) = {
                    let mut rng = rng();
                    target_positions.choose(&mut rng)
                } {
                    if let Ok(()) = state.play_turn_from_position(*piece, *target_pos) {
                        info!(
                            "Game {} - Turn {}: Move {} from {} to {}",
                            started_game.nanoid,
                            move_number + 1,
                            piece,
                            from_pos,
                            target_pos
                        );
                        move_made = true;
                    }
                }
            }
        }

        if !move_made && !available_spawns.is_empty() && !reserve.is_empty() {
            let reserve_entries: Vec<_> = reserve
                .iter()
                .flat_map(|(bug, pieces)| pieces.iter().map(move |p| (*bug, p.clone())))
                .collect();

            if reserve_entries.is_empty() {
                info!(
                    "Game {} - Turn {}: Reserve is empty after filtering",
                    started_game.nanoid,
                    move_number + 1
                );
            } else if let Some((_bug, piece_str)) = {
                let mut rng = rng();
                reserve_entries.choose(&mut rng)
            } {
                match piece_str.parse::<Piece>() {
                    Ok(piece) => {
                        if let Some(spawn_pos) = {
                            let mut rng = rng();
                            available_spawns.choose(&mut rng)
                        } {
                            match state.play_turn_from_position(piece, *spawn_pos) {
                                Ok(()) => {
                                    info!(
                                        "Game {} - Turn {}: Spawn {} at {}",
                                        started_game.nanoid,
                                        move_number + 1,
                                        piece,
                                        spawn_pos
                                    );
                                    move_made = true;
                                }
                                Err(e) => {
                                    log::warn!(
                                        "Game {} - Turn {}: Failed to spawn {} at {}: {:?}",
                                        started_game.nanoid,
                                        move_number + 1,
                                        piece,
                                        spawn_pos,
                                        e
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "Game {} - Turn {}: Failed to parse piece '{}': {:?}",
                            started_game.nanoid,
                            move_number + 1,
                            piece_str,
                            e
                        );
                    }
                }
            }
        }

        if !move_made {
            info!(
                "Game {} - Turn {}: No valid move could be made",
                started_game.nanoid,
                move_number + 1
            );
            break;
        }
    }

    if state.turn > 0 {
        info!("Updating game state in database...");
        started_game
            .update_gamestate(&state, 0.0, conn)
            .await
            .map_err(|e| {
                log::error!(
                    "Failed to update game state for game {}: {}",
                    started_game.nanoid,
                    e
                );
                e
            })?;
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
) -> Result<(), Box<dyn std::error::Error>> {
    let resigning_user = {
        let mut rng = rng();
        if rng.random_bool(RESIGNATION_PROBABILITY) {
            game.white_id
        } else {
            game.black_id
        }
    };

    let user_color = game.user_color(resigning_user).ok_or_else(|| {
        let error = format!(
            "User {} is not a player in game {}",
            resigning_user, game.nanoid
        );
        log::error!("{}", error);
        error
    })?;

    let game_control = GameControl::Resign(user_color);

    info!(
        "Resigning game {} for user {} ({})",
        game.nanoid, resigning_user, user_color
    );
    game.resign(&game_control, conn).await.map_err(|e| {
        log::error!("Failed to resign game {}: {}", game.nanoid, e);
        e
    })?;

    info!("Game {} resigned by user {}", game.nanoid, resigning_user);

    Ok(())
}

pub async fn cleanup_test_data(database_url: Option<String>) -> Result<()> {
    log_operation_start("test data cleanup");

    info!("Setting up database connection...");
    let mut conn = setup_database(database_url)
        .await
        .context("Failed to setup database connection")?;
    info!("Connected to database");

    let test_users = find_test_users(&mut conn).await?;
    info!("Found {} test users to clean up", test_users.len());

    if test_users.is_empty() {
        info!("No test users found, nothing to clean up");
        log_operation_complete("Test data cleanup", 0, 0);
        return Ok(());
    }

    let deleted_games = delete_games_involving_test_users(&mut conn, &test_users).await?;
    info!("Deleted {} games", deleted_games);

    let deleted_users = delete_test_users(&mut conn).await?;

    info!("Deleted {} test users", deleted_users);

    let total_deleted = deleted_games + deleted_users;
    log_operation_complete("Test data cleanup", total_deleted, 0);

    info!("Cleanup completed successfully!");
    info!("Summary:");
    info!("  - Deleted {} games", deleted_games);
    info!(
        "  - Deleted {} test users (ratings and game_users entries cascaded)",
        deleted_users
    );
    info!("  - Total records deleted: {}", total_deleted);

    Ok(())
}

async fn find_test_users(conn: &mut db_lib::DbConn<'_>) -> Result<Vec<Uuid>> {
    info!("Finding test users...");
    db_lib::schema::users::table
        .filter(
            db_lib::schema::users::username
                .like("testuser%")
                .and(db_lib::schema::users::email.like("test%@example.com")),
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
                .like("testuser%")
                .and(db_lib::schema::users::email.like("test%@example.com")),
        ),
    )
    .execute(conn)
    .await
    .context("Failed to delete test users")
}
