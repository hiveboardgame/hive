use anyhow::{Context, Result};
use db_lib::models::Game;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use hive_lib::{Color, History, State};

use crate::common::{log_operation_complete, log_operation_start, log_progress, setup_database};
use log::{info, warn};
use rand::prelude::*;
use rand::rng;
use tempfile::NamedTempFile;

const LARGE_DATASET_THRESHOLD: usize = 100_000;

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
    let game_count = all_games.len();
    info!("Found {game_count} games to analyze");

    let games_to_analyze = select_games_for_analysis(all_games, sample_size)?;

    let analyze_count = games_to_analyze.len();
    info!("Analyzing {analyze_count} games");

    let max_turns = find_maximum_turns(&games_to_analyze);
    let mut temp_file = create_temp_file_for_csv()?;
    let mut writer = csv::Writer::from_writer(&mut temp_file);
    write_csv_header(&mut writer, max_turns)?;

    let (processed, errors) = process_games_and_write_to_csv(games_to_analyze, &mut writer, max_turns).await;

    writer.flush().context("Failed to flush CSV writer")?;
    drop(writer);

    persist_csv_file(temp_file, "game_statistics.csv")?;

    log_operation_complete("Game statistics collection", processed, errors);
    info!("Results written to game_statistics.csv");

    Ok(())
}

async fn analyze_game(game: &Game, max_turns: usize) -> Result<Vec<String>> {
    if game.history.is_empty() {
        return Err(anyhow::anyhow!("Game has empty history"));
    }

    let history = History::new_from_str(&game.history).context("Failed to parse game history")?;

    let mut stats = Vec::with_capacity(1 + 1 + max_turns * 2);
    stats.push(game.nanoid.clone());
    let mut moves_per_turn = Vec::with_capacity(max_turns);
    let mut spawns_per_turn = Vec::with_capacity(max_turns);

    let mut current_history = History::new();
    let mut turn = 0;

    for (piece, position) in &history.moves {
        let current_state = State::new_from_history(&current_history)
            .with_context(|| format!("Failed to create state from history at turn {turn}"))?;

        let current_color = if turn % 2 == 0 {
            Color::White
        } else {
            Color::Black
        };

        let available_moves = current_state.board.moves(current_color);
        let total_moves: usize = available_moves.values().map(|moves| moves.len()).sum();

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

    pad_turn_vectors_to_max_turns(&mut moves_per_turn, &mut spawns_per_turn, max_turns);

    for turn_index in 0..max_turns {
        stats.push(moves_per_turn[turn_index].clone());
        stats.push(spawns_per_turn[turn_index].clone());
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

fn select_games_for_analysis(all_games: Vec<Game>, sample_size: Option<usize>) -> Result<Vec<Game>> {
    match sample_size {
        Some(size) if size >= all_games.len() => {
            let total = all_games.len();
            info!("Sample size {size} is >= total games {total}, analyzing all games");
            Ok(all_games)
        }
        Some(size) => {
            let total = all_games.len();
            info!("Randomly sampling {size} games from {total} total games");
            Ok(sample_games_randomly(&all_games, size))
        }
        None => {
            if all_games.len() > LARGE_DATASET_THRESHOLD {
                let total = all_games.len();
                warn!("Large dataset detected ({total} games). Consider using --sample-size for faster processing.");
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

fn create_temp_file_for_csv() -> Result<NamedTempFile> {
    NamedTempFile::new().context("Failed to create temporary file for CSV writing")
}

fn write_csv_header(writer: &mut csv::Writer<&mut NamedTempFile>, max_turns: usize) -> Result<()> {
    let mut header = Vec::with_capacity(2 + max_turns * 2);
    header.push("nanoid".to_string());
    header.push("total_turns".to_string());
    for turn in 0..max_turns {
        header.push(format!("moves_turn_{turn}"));
        header.push(format!("spawns_turn_{turn}"));
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
                    warn!("Failed to write game {} to CSV: {e}", game.nanoid);
                    errors += 1;
                }
            }
            Err(e) => {
                warn!("Failed to analyze game {}: {e}", game.nanoid);
                errors += 1;
            }
        }

        processed += 1;
        log_progress(processed, total_games, "Analyzing games");
    }

    (processed, errors)
}

fn persist_csv_file(temp_file: NamedTempFile, filename: &str) -> Result<()> {
    temp_file
        .persist(filename)
        .map(|_| ())
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

fn pad_turn_vectors_to_max_turns(
    moves_per_turn: &mut Vec<String>,
    spawns_per_turn: &mut Vec<String>,
    max_turns: usize,
) {
    let current_len = moves_per_turn.len();
    if current_len < max_turns {
        moves_per_turn.reserve_exact(max_turns - current_len);
        spawns_per_turn.reserve_exact(max_turns - current_len);
        for _ in current_len..max_turns {
            moves_per_turn.push(String::new());
            spawns_per_turn.push(String::new());
        }
    }
}
