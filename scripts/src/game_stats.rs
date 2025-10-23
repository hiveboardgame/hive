use db_lib::models::Game;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use hive_lib::{Color, History, State};

use crate::common::{
    log_operation_complete, log_operation_start, log_progress, log_warning, setup_database,
};
use log::{info, warn};
use rand::{prelude::*, rng};
use std::error::Error;
use tempfile::NamedTempFile;

pub async fn run_game_stats(
    database_url: Option<String>,
    sample_size: Option<usize>,
    no_bots: bool,
) -> Result<(), Box<dyn Error>> {
    log_operation_start("game statistics collection");

    let mut conn = setup_database(database_url).await?;

    // Build the query based on filters
    let mut query = db_lib::schema::games::table.into_boxed();

    // Apply bot filter if requested
    if no_bots {
        query = query
            .filter(
                db_lib::schema::games::white_id.ne_all(
                    db_lib::schema::users::table
                        .filter(db_lib::schema::users::bot.eq(true))
                        .select(db_lib::schema::users::id),
                ),
            )
            .filter(
                db_lib::schema::games::black_id.ne_all(
                    db_lib::schema::users::table
                        .filter(db_lib::schema::users::bot.eq(true))
                        .select(db_lib::schema::users::id),
                ),
            );
    }

    // Load all games first
    let all_games = query.load::<Game>(&mut conn).await?;
    info!("Found {} games to analyze", all_games.len());

    // Apply sampling if requested
    let games_to_analyze = if let Some(sample_size) = sample_size {
        if sample_size >= all_games.len() {
            info!(
                "Sample size {} is >= total games {}, analyzing all games",
                sample_size,
                all_games.len()
            );
            all_games
        } else {
            info!(
                "Randomly sampling {} games from {} total games",
                sample_size,
                all_games.len()
            );
            sample_games_randomly(&all_games, sample_size)
        }
    } else {
        // For very large datasets, consider chunked processing
        if all_games.len() > 100_000 {
            warn!("Large dataset detected ({} games). Consider using --sample-size for faster processing.", all_games.len());
        }
        all_games
    };

    info!("Analyzing {} games", games_to_analyze.len());

    // Create temporary file for safer CSV writing
    let mut temp_file = NamedTempFile::new()?;
    let mut wtr = csv::Writer::from_writer(&mut temp_file);

    let max_turns = games_to_analyze
        .iter()
        .map(|game| game.turn)
        .max()
        .unwrap_or(0) as usize;

    let mut header = vec!["nanoid".to_string(), "total_turns".to_string()];
    for turn in 0..max_turns {
        header.push(format!("moves_turn_{}", turn));
        header.push(format!("spawns_turn_{}", turn));
    }

    wtr.write_record(&header)?;

    let mut processed = 0;
    let mut errors = 0;

    let total_games = games_to_analyze.len();
    for game in games_to_analyze {
        match analyze_game(&game, max_turns).await {
            Ok(stats) => {
                if let Err(e) = wtr.write_record(&stats) {
                    log_warning(&format!(
                        "Failed to write game {} to CSV: {}",
                        game.nanoid, e
                    ));
                    errors += 1;
                }
            }
            Err(e) => {
                log_warning(&format!("Failed to analyze game {}: {}", game.nanoid, e));
                errors += 1;
            }
        }

        processed += 1;
        log_progress(processed, total_games, "Analyzing games");
    }

    wtr.flush()?;
    drop(wtr); // Explicitly drop the writer to release the borrow

    // Persist the temporary file to the final location
    temp_file.persist("game_statistics.csv")?;

    log_operation_complete("Game statistics collection", processed, errors);
    info!("Results written to game_statistics.csv");

    Ok(())
}

async fn analyze_game(game: &Game, max_turns: usize) -> Result<Vec<String>, Box<dyn Error>> {
    if game.history.is_empty() {
        return Err("Game has empty history".into());
    }

    let history = History::new_from_str(&game.history)
        .map_err(|e| format!("Failed to parse game history: {}", e))?;

    let mut stats = vec![game.nanoid.clone()];
    let mut moves_per_turn = Vec::new();
    let mut spawns_per_turn = Vec::new();

    let mut current_history = History::new();
    let mut turn = 0;

    for (piece, position) in &history.moves {
        let current_state = State::new_from_history(&current_history).map_err(|e| {
            format!(
                "Failed to create state from history at turn {}: {}",
                turn, e
            )
        })?;

        let current_color = if turn % 2 == 0 {
            Color::White
        } else {
            Color::Black
        };

        // Count available moves
        let available_moves = current_state.board.moves(current_color);
        let total_moves: usize = available_moves.values().map(|moves| moves.len()).sum();

        // Count available spawn positions
        let available_spawns = current_state.board.spawnable_positions(current_color);
        let total_spawns = available_spawns.count();

        moves_per_turn.push(total_moves.to_string());
        spawns_per_turn.push(total_spawns.to_string());

        current_history
            .moves
            .push((piece.clone(), position.clone()));
        turn += 1;
    }

    stats.push(turn.to_string());

    // Pad with empty strings to match max_turns
    while moves_per_turn.len() < max_turns {
        moves_per_turn.push("".to_string());
        spawns_per_turn.push("".to_string());
    }

    // Interleave moves and spawns data
    for i in 0..max_turns {
        stats.push(moves_per_turn[i].clone());
        stats.push(spawns_per_turn[i].clone());
    }

    Ok(stats)
}

fn sample_games_randomly(games: &[Game], sample_size: usize) -> Vec<Game> {
    if sample_size >= games.len() {
        return games.to_vec();
    }

    let mut rng = rng();
    games
        .choose_multiple(&mut rng, sample_size)
        .cloned()
        .collect()
}
