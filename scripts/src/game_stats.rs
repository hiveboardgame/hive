use anyhow::{Context, Result};
use db_lib::models::Game;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use hive_lib::{Color, History, State};

use crate::common::{log_operation_complete, log_operation_start, log_progress, setup_database};
use log::{info, warn};
use rand::{prelude::*, rng};
use tempfile::NamedTempFile;

pub async fn run_game_stats(
    database_url: Option<String>,
    sample_size: Option<usize>,
    no_bots: bool,
) -> Result<()> {
    log_operation_start("game statistics collection");

    let mut conn = setup_database(database_url)
        .await
        .context("Failed to setup database connection")?;

    let all_games = load_games_with_filters(&mut conn, no_bots).await?;
    info!("Found {} games to analyze", all_games.len());

    let games_to_analyze = select_games_for_analysis(all_games, sample_size).await?;

    info!("Analyzing {} games", games_to_analyze.len());

    let max_turns = find_maximum_turns(&games_to_analyze);
    let mut temp_file = create_csv_writer_for_game_stats().await?;
    let mut writer = csv::Writer::from_writer(&mut temp_file);
    write_csv_header(&mut writer, max_turns).await?;
    
    let (processed, errors) = process_games_and_write_to_csv(games_to_analyze, &mut writer, max_turns).await;
    
    persist_csv_file(temp_file, writer, "game_statistics.csv").await?;

    log_operation_complete("Game statistics collection", processed, errors);
    info!("Results written to game_statistics.csv");

    Ok(())
}

async fn analyze_game(game: &Game, max_turns: usize) -> Result<Vec<String>> {
    if game.history.is_empty() {
        return Err("Game has empty history".into());
    }

    let history = History::new_from_str(&game.history).context("Failed to parse game history")?;

    let mut stats = vec![game.nanoid.clone()];
    let mut moves_per_turn = Vec::new();
    let mut spawns_per_turn = Vec::new();

    let mut current_history = History::new();
    let mut turn = 0;

    for (piece, position) in &history.moves {
        let current_state = State::new_from_history(&current_history)
            .with_context(|| format!("Failed to create state from history at turn {}", turn))?;

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

async fn load_games_with_filters(conn: &mut db_lib::DbConn<'_>, no_bots: bool) -> Result<Vec<Game>> {
    let mut query = db_lib::schema::games::table.into_boxed();

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

    query
        .load::<Game>(conn)
        .await
        .context("Failed to load games from database")
}

async fn select_games_for_analysis(all_games: Vec<Game>, sample_size: Option<usize>) -> Result<Vec<Game>> {
    match sample_size {
        Some(size) if size >= all_games.len() => {
            info!("Sample size {} is >= total games {}, analyzing all games", size, all_games.len());
            Ok(all_games)
        }
        Some(size) => {
            info!("Randomly sampling {} games from {} total games", size, all_games.len());
            Ok(sample_games_randomly(&all_games, size))
        }
        None => {
            if all_games.len() > 100_000 {
                warn!("Large dataset detected ({} games). Consider using --sample-size for faster processing.", all_games.len());
            }
            Ok(all_games)
        }
    }
}

fn find_maximum_turns(games: &[Game]) -> usize {
    games
        .iter()
        .map(|game| game.turn)
        .max()
        .unwrap_or(0) as usize
}

async fn create_csv_writer_for_game_stats() -> Result<NamedTempFile> {
    NamedTempFile::new().context("Failed to create temporary file for CSV writing")
}

async fn write_csv_header(writer: &mut csv::Writer<&mut NamedTempFile>, max_turns: usize) -> Result<()> {
    let mut header = vec!["nanoid".to_string(), "total_turns".to_string()];
    for turn in 0..max_turns {
        header.push(format!("moves_turn_{}", turn));
        header.push(format!("spawns_turn_{}", turn));
    }

    writer
        .write_record(&header)
        .context("Failed to write CSV header")
}

async fn process_games_and_write_to_csv(
    games: Vec<Game>,
    writer: &mut csv::Writer<&mut NamedTempFile>,
    max_turns: usize,
) -> (usize, usize) {
    let mut processed = 0;
    let mut errors = 0;
    let total_games = games.len();

    for game in games {
        match analyze_game(&game, max_turns).await {
            Ok(stats) => {
                if let Err(e) = writer.write_record(&stats) {
                    warn!("Failed to write game {} to CSV: {}", game.nanoid, e);
                    errors += 1;
                }
            }
            Err(e) => {
                warn!("Failed to analyze game {}: {}", game.nanoid, e);
                errors += 1;
            }
        }

        processed += 1;
        log_progress(processed, total_games, "Analyzing games");
    }

    (processed, errors)
}

async fn persist_csv_file(
    temp_file: NamedTempFile,
    mut writer: csv::Writer<&mut NamedTempFile>,
    filename: &str,
) -> Result<()> {
    writer.flush().context("Failed to flush CSV writer")?;
    drop(writer);
    temp_file
        .persist(filename)
        .context("Failed to persist CSV file")
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
