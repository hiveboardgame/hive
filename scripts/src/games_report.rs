use db_lib::{models::{Game, Rating, User}};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use shared_types::GameSpeed;

use std::error::Error;
use tempfile::NamedTempFile;
use log::info;
use crate::common::{setup_database, log_progress, log_operation_start, log_operation_complete, log_warning};

pub async fn run_games_report(database_url: Option<String>) -> Result<(), Box<dyn Error>> {
    log_operation_start("games report generation");
    
    let mut conn = setup_database(database_url).await?;
    
    // Get all games excluding bot games
    let games: Vec<Game> = db_lib::schema::games::table
        .filter(db_lib::schema::games::game_type.ne("Bot"))
        .load(&mut conn)
        .await?;
    
    info!("Found {} games (excluding bot games)", games.len());
    
    // Create temporary file for safer CSV writing
    let mut temp_file = NamedTempFile::new()?;
    let mut wtr = csv::Writer::from_writer(&mut temp_file);
    
    // Write CSV header
    wtr.write_record(&[
        "game_nanoid",
        "result",
        "white_player_username",
        "black_player_username",
        "white_elo",
        "black_elo",
        "white_elo_deviation",
        "black_elo_deviation",
        "white_rating_certainty",
        "black_rating_certainty",
        "time_control_category",
        "tournament_game",
        "tournament_id",
        "game_created_at"
    ])?;
    
    let mut processed = 0;
    let mut errors = 0;
    
    let total_games = games.len();
    for game in games {
        match process_game(&game, &mut conn).await {
            Ok(record) => {
                if let Err(e) = wtr.write_record(&record) {
                    log_warning(&format!("Failed to write game {} to CSV: {}", game.nanoid, e));
                    errors += 1;
                }
            }
            Err(e) => {
                log_warning(&format!("Failed to process game {}: {}", game.nanoid, e));
                errors += 1;
            }
        }
        
        processed += 1;
        log_progress(processed, total_games, "Processing games");
    }
    
    wtr.flush()?;
    drop(wtr); // Explicitly drop the writer to release the borrow
    
    // Persist the temporary file to the final location
    temp_file.persist("games_report.csv")?;
    
    log_operation_complete("Games report generation", processed, errors);
    info!("Results written to games_report.csv");
    
    Ok(())
}

async fn process_game(
    game: &Game,
    conn: &mut db_lib::DbConn<'_>
) -> Result<Vec<String>, Box<dyn Error>> {
    // Get white player info
    let white_user = db_lib::schema::users::table
        .filter(db_lib::schema::users::id.eq(game.white_id))
        .first::<User>(conn)
        .await?;
    
    // Get black player info
    let black_user = db_lib::schema::users::table
        .filter(db_lib::schema::users::id.eq(game.black_id))
        .first::<User>(conn)
        .await?;
    // Parse game speed to get time control category
    let time_control_category = match game.speed.parse::<GameSpeed>() {
        Ok(speed) => match speed {
            GameSpeed::Bullet => "Bullet",
            GameSpeed::Blitz => "Blitz", 
            GameSpeed::Rapid => "Rapid",
            GameSpeed::Classic => "Classic",
            GameSpeed::Correspondence => "Correspondence",
            _ => "Other",
        },
        Err(_) => "Unknown",
    };
    
    // Get ratings for both players
    let game_speed = game.speed.parse::<GameSpeed>()?;
    let white_rating = Rating::for_uuid(&game.white_id, &game_speed, conn).await?;
    let black_rating = Rating::for_uuid(&game.black_id, &game_speed, conn).await?;
    
    // Determine rating certainty based on deviation
    let white_certainty = get_rating_certainty(white_rating.deviation);
    let black_certainty = get_rating_certainty(black_rating.deviation);
    
    // Check if it's a tournament game
    let tournament_game = game.tournament_id.is_some();
    let tournament_id = game.tournament_id.map(|id| id.to_string()).unwrap_or_default();
    
    // Format the result
    let result = format_result(&game.conclusion);
    
    Ok(vec![
        game.nanoid.clone(),
        result,
        white_user.username.clone(),
        black_user.username.clone(),
        format!("{:.0}", white_rating.rating),
        format!("{:.0}", black_rating.rating),
        format!("{:.0}", white_rating.deviation),
        format!("{:.0}", black_rating.deviation),
        white_certainty,
        black_certainty,
        time_control_category.to_string(),
        tournament_game.to_string(),
        tournament_id,
        game.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
    ])
}

fn get_rating_certainty(deviation: f64) -> String {
    use shared_types::RANKABLE_DEVIATION;
    
    if deviation <= RANKABLE_DEVIATION {
        "Rankable".to_string()
    } else if deviation <= 200.0 {
        "Provisional".to_string()
    } else {
        "Clueless".to_string()
    }
}

fn format_result(conclusion: &str) -> String {
    match conclusion {
        "Winner(White)" => "White Wins".to_string(),
        "Winner(Black)" => "Black Wins".to_string(), 
        "Draw" => "Draw".to_string(),
        "Resignation(White)" => "White Resigns".to_string(),
        "Resignation(Black)" => "Black Resigns".to_string(),
        "Timeout(White)" => "White Timeout".to_string(),
        "Timeout(Black)" => "Black Timeout".to_string(),
        _ => conclusion.to_string(),
    }
}