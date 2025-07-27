use db_lib::{get_conn, models::Game, DbPool, schema::games};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use hive_lib::{History, State, Color};
use std::fs::File;
use std::error::Error;
use log::{info, warn};

pub async fn run_once(pool: DbPool) -> Result<(), Box<dyn Error>> {
    info!("Starting game statistics collection job...");
    
    let mut conn = get_conn(&pool).await
        .map_err(|e| format!("Failed to get database connection: {}", e))?;
    
    let all_games: Vec<Game> = games::table
        .load(&mut conn)
        .await
        .map_err(|e| format!("Failed to load games from database: {}", e))?;
    
    info!("Found {} games to analyze", all_games.len());
    
    let file = File::create("game_statistics.csv")
        .map_err(|e| format!("Failed to create CSV file: {}", e))?;
    
    let mut wtr = csv::Writer::from_writer(file);
    
    let max_turns = all_games.iter()
        .map(|game| game.turn)
        .max()
        .unwrap_or(0) as usize;
    
    let mut header = vec!["nanoid".to_string(), "total_turns".to_string()];
    for turn in 0..max_turns {
        header.push(format!("moves_turn_{}", turn));
    }
    
    wtr.write_record(&header)
        .map_err(|e| format!("Failed to write CSV header: {}", e))?;
    
    let mut processed = 0;
    let mut errors = 0;
    
    for game in all_games {
        match analyze_game(&game, max_turns).await {
            Ok(stats) => {
                if let Err(e) = wtr.write_record(&stats) {
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
        if processed % 1000 == 0 {
            info!("Processed {} games...", processed);
        }
    }
    
    wtr.flush()
        .map_err(|e| format!("Failed to flush CSV writer: {}", e))?;
    
    info!("Game statistics collection completed! Processed {} games with {} errors", processed, errors);
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
    
    let mut current_history = History::new();
    let mut turn = 0;
    
    for (piece, position) in &history.moves {
        let current_state = State::new_from_history(&current_history)
            .map_err(|e| format!("Failed to create state from history at turn {}: {}", turn, e))?;
        
        let current_color = if turn % 2 == 0 { Color::White } else { Color::Black };
        
        let available_moves = current_state.board.moves(current_color);
        let total_moves: usize = available_moves.values().map(|moves| moves.len()).sum();
        
        moves_per_turn.push(total_moves.to_string());
        
        current_history.moves.push((piece.clone(), position.clone()));
        turn += 1;
    }
    
    stats.push(turn.to_string());
    
    while moves_per_turn.len() < max_turns {
        moves_per_turn.push("".to_string());
    }
    
    stats.extend(moves_per_turn);
    
    Ok(stats)
} 