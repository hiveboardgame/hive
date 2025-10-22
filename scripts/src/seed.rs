use db_lib::{models::{NewGame, NewUser, User, Game}};
use diesel::prelude::*;
use diesel_async::{RunQueryDsl, AsyncConnection};

use nanoid::nanoid;
use shared_types::{GameSpeed, TimeMode};
use uuid::Uuid;
use chrono::Utc;
use hive_lib::{GameType, GameControl, State, History, Piece, Position};
use std::str::FromStr;
use log::info;
use rand::{rng, Rng};
use rand::prelude::*;
use crate::common::{setup_database, log_progress, log_operation_start, log_operation_complete};

pub async fn run_seed_database(
    num_users: usize,
    games_per_user: usize,
    database_url: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    log_operation_start("database seeding");
    
    info!("Setting up database connection...");
    let mut conn = setup_database(database_url).await
        .map_err(|e| {
            log::error!("Failed to setup database: {}", e);
            e
        })?;
    info!("Connected to database");
    
    // Wrap the entire seeding workflow in a single database transaction
    info!("Beginning transactional seeding (all-or-nothing)...");
    let (created_users, created_games) = conn
        .transaction(|conn| Box::pin(async move {
            info!("Creating {} test users...", num_users);
            let user_ids = create_test_users(conn, num_users).await
                .map_err(|e| {
                    log::error!("Failed to create test users: {}", e);
                    diesel::result::Error::RollbackTransaction
                })?;
            info!("Created {} users", user_ids.len());

            info!("Playing {} games per user...", games_per_user);
            let total_games = play_test_games(conn, &user_ids, games_per_user).await
                .map_err(|e| {
                    log::error!("Failed to play test games: {}", e);
                    diesel::result::Error::RollbackTransaction
                })?;
            info!("Created {} total games", total_games);

            Ok::<(usize, usize), diesel::result::Error>((user_ids.len(), total_games))
        }))
        .await
        .map_err(|e| {
            log::error!("Seeding transaction failed and was rolled back: {}", e);
            e
        })?;

    log_operation_complete("Database seeding", created_users + created_games, 0);
    Ok(())
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
        
        info!("Creating user {}...", username);
        let new_user = NewUser::new(&username, &password_hash, &email)
            .map_err(|e| {
                log::error!("Failed to create NewUser for {}: {}", username, e);
                e
            })?;
        
        let user = User::create(new_user, conn).await
            .map_err(|e| {
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
        info!("Creating games for user {} ({}/{})", user_id, user_idx + 1, user_ids.len());
        
        for game_idx in 0..games_per_user {
            let opponent_id = get_random_opponent(user_id, user_ids);
            let game_speed = GameSpeed::Bullet;
            
            let (white_id, black_id) = {
                let mut rng = rng();
                if rng.random_bool(0.5) {
                    (*user_id, opponent_id)
                } else {
                    (opponent_id, *user_id)
                }
            };
            
            info!("Creating game {} for user {} (game {}/{})", total_games + 1, user_id, game_idx + 1, games_per_user);
            
            let game = create_game(
                conn,
                white_id,
                black_id,
                game_speed,
            ).await
            .map_err(|e| {
                log::error!("Failed to create game {}: {}", total_games + 1, e);
                e
            })?;
            
            info!("Playing game {}...", game.nanoid);
            play_game(&game, conn).await
            .map_err(|e| {
                log::error!("Failed to play game {}: {}", game.nanoid, e);
                e
            })?;
            
            info!("Resigning game {}...", game.nanoid);
            resign_game(&game, conn).await
            .map_err(|e| {
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
    // Start the game
    info!("Starting game {}...", game.nanoid);
    let started_game = game.start(conn).await
        .map_err(|e| {
            log::error!("Failed to start game {}: {}", game.nanoid, e);
            e
        })?;
    info!("Started game: {}", started_game.nanoid);
    
    // Create game state from history
    info!("Creating game state from history...");
    let history = History::new_from_str(&started_game.history)
        .map_err(|e| {
            log::error!("Failed to parse history for game {}: {}", started_game.nanoid, e);
            e
        })?;
    
    let mut state = State::new_from_history(&history)
        .map_err(|e| {
            log::error!("Failed to create state from history for game {}: {}", started_game.nanoid, e);
            e
        })?;
    
    state.game_type = GameType::from_str(&started_game.game_type)
        .map_err(|e| {
            log::error!("Failed to parse game type '{}' for game {}: {}", started_game.game_type, started_game.nanoid, e);
            e
        })?;
    
    // Play up to 100 moves
    for move_num in 0..100 {
        // Check if game is finished
        if matches!(state.game_status, hive_lib::GameStatus::Finished(_)) {
            info!("Game {} finished after {} moves", started_game.nanoid, move_num);
            break;
        }
        
        let current_color = state.turn_color;
        
        // Get available spawns and reserve from the engine
        let available_spawns: Vec<Position> = state.board.spawnable_positions(current_color).collect();
        let mut reserve = state.reserve(current_color);
        
        // Tournament rule: Queen cannot be spawned on turns 0 or 1 (first 2 moves)
        // Must be spawned by turn 6 for white (4th white move) or turn 7 for black (4th black move)
        if state.turn < 2 {
            // Remove queen from reserve for first 2 turns
            reserve.remove(&hive_lib::Bug::Queen);
        }
        
        // Check if we can make moves (only after the current player's queen is played)
        let can_make_moves = state.board.queen_played(current_color);
        let available_moves = if can_make_moves {
            state.board.moves(current_color)
        } else {
            std::collections::HashMap::new()
        };
        
        info!("Game {} - Turn {} (game turn {}): {:?} to play - {} moves, {} spawn positions, {} pieces in reserve", 
              started_game.nanoid, move_num + 1, state.turn, current_color, 
              available_moves.len(), available_spawns.len(), 
              reserve.values().map(|v| v.len()).sum::<usize>());
        
        // If no moves and no spawns available, end the game
        if available_moves.is_empty() && (available_spawns.is_empty() || reserve.is_empty()) {
            info!("Game {} - No valid moves or spawns available, ending game", started_game.nanoid);
            break;
        }
        
        let mut move_made = false;
        
        // Decide: make a move or spawn
        // Prefer spawning early (50% chance), then moving later (if queen is out and 70% chance)
        let should_try_move = can_make_moves && !available_moves.is_empty() && { 
            let mut rng = rng(); 
            rng.random_bool(0.3)
        };
        
        if should_try_move {
            // Make a move - pick from the valid moves returned by the engine
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
                        info!("Game {} - Turn {}: Move {} from {} to {}", 
                              started_game.nanoid, move_num + 1, piece, from_pos, target_pos);
                        move_made = true;
                    }
                }
            }
        }
        
        // If we didn't make a move, try to spawn
        if !move_made && !available_spawns.is_empty() && !reserve.is_empty() {
            // Pick a random piece from the reserve
            let reserve_entries: Vec<_> = reserve.iter()
                .flat_map(|(bug, pieces)| pieces.iter().map(move |p| (*bug, p.clone())))
                .collect();
            
            if reserve_entries.is_empty() {
                info!("Game {} - Turn {}: Reserve is empty after filtering", 
                      started_game.nanoid, move_num + 1);
            } else if let Some((_bug, piece_str)) = { 
                let mut rng = rng(); 
                reserve_entries.choose(&mut rng) 
            } {
                // Parse the piece string to get the actual piece
                match piece_str.parse::<Piece>() {
                    Ok(piece) => {
                        // Pick a random spawn position
                        if let Some(spawn_pos) = { 
                            let mut rng = rng(); 
                            available_spawns.choose(&mut rng) 
                        } {
                            match state.play_turn_from_position(piece, *spawn_pos) {
                                Ok(()) => {
                                    info!("Game {} - Turn {}: Spawn {} at {}", 
                                          started_game.nanoid, move_num + 1, piece, spawn_pos);
                                    move_made = true;
                                }
                                Err(e) => {
                                    log::warn!("Game {} - Turn {}: Failed to spawn {} at {}: {:?}", 
                                               started_game.nanoid, move_num + 1, piece, spawn_pos, e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Game {} - Turn {}: Failed to parse piece '{}': {:?}", 
                                   started_game.nanoid, move_num + 1, piece_str, e);
                    }
                }
            }
        }
        
        if !move_made {
            info!("Game {} - Turn {}: No valid move could be made", started_game.nanoid, move_num + 1);
            break;
        }
    }
    
    // Update the game in the database with the new state
    if state.turn > 0 {
        info!("Updating game state in database...");
        started_game.update_gamestate(&state, 0.0, conn).await
            .map_err(|e| {
                log::error!("Failed to update game state for game {}: {}", started_game.nanoid, e);
                e
            })?;
    }
    
    info!("Game {} completed with {} moves", started_game.nanoid, state.turn);
    Ok(())
}

async fn resign_game(
    game: &Game,
    conn: &mut db_lib::DbConn<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Pick a random player to resign
    let resigning_user = {
        let mut rng = rng();
        if rng.random_bool(0.5) {
            game.white_id
        } else {
            game.black_id
        }
    };
    
    // Create the resignation game control
    let user_color = game.user_color(resigning_user)
        .ok_or_else(|| {
            let error = format!("User {} is not a player in game {}", resigning_user, game.nanoid);
            log::error!("{}", error);
            error
        })?;
    
    let game_control = GameControl::Resign(user_color);
    
    // Resign the game
    info!("Resigning game {} for user {} ({})", game.nanoid, resigning_user, user_color);
    game.resign(&game_control, conn).await
        .map_err(|e| {
            log::error!("Failed to resign game {}: {}", game.nanoid, e);
            e
        })?;
    
    info!("Game {} resigned by user {}", game.nanoid, resigning_user);
    
    Ok(())
}

pub async fn cleanup_test_data(database_url: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    log_operation_start("test data cleanup");
    
    info!("Setting up database connection...");
    let mut conn = setup_database(database_url).await
        .map_err(|e| {
            log::error!("Failed to setup database: {}", e);
            e
        })?;
    info!("Connected to database");
    
    // First, find all test users (by username and email patterns)
    info!("Finding test users...");
    let test_users = db_lib::schema::users::table
        .filter(
            db_lib::schema::users::username.like("testuser%")
                .and(db_lib::schema::users::email.like("test%@example.com"))
        )
        .select(db_lib::schema::users::id)
        .load::<Uuid>(&mut conn)
        .await
        .map_err(|e| {
            log::error!("Failed to find test users: {}", e);
            e
        })?;
    
    info!("Found {} test users to clean up", test_users.len());
    
    if test_users.is_empty() {
        info!("No test users found, nothing to clean up");
        log_operation_complete("Test data cleanup", 0, 0);
        return Ok(());
    }
    
    // Delete all games involving test users
    info!("Deleting games involving test users...");
    let deleted_games = diesel::delete(
        db_lib::schema::games::table.filter(
            db_lib::schema::games::white_id.eq_any(&test_users)
                .or(db_lib::schema::games::black_id.eq_any(&test_users))
        )
    )
    .execute(&mut conn)
    .await
    .map_err(|e| {
        log::error!("Failed to delete games: {}", e);
        e
    })?;
    
    info!("Deleted {} games", deleted_games);
    
    // Note: Ratings and game_users entries will be automatically deleted 
    // due to foreign key constraints with CASCADE DELETE
    
    // Finally, delete the test users themselves
    info!("Deleting test users...");
    let deleted_users = diesel::delete(
        db_lib::schema::users::table.filter(
            db_lib::schema::users::username.like("testuser%")
                .and(db_lib::schema::users::email.like("test%@example.com"))
        )
    )
    .execute(&mut conn)
    .await
    .map_err(|e| {
        log::error!("Failed to delete test users: {}", e);
        e
    })?;
    
    info!("Deleted {} test users", deleted_users);
    
    let total_deleted = deleted_games + deleted_users;
    log_operation_complete("Test data cleanup", total_deleted, 0);
    
    info!("Cleanup completed successfully!");
    info!("Summary:");
    info!("  - Deleted {} games", deleted_games);
    info!("  - Deleted {} test users (ratings and game_users entries cascaded)", deleted_users);
    info!("  - Total records deleted: {}", total_deleted);
    
    Ok(())
}
