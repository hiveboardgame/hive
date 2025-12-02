use anyhow::{Context, Result};
use db_lib::models::{Game, Rating, User};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use shared_types::GameSpeed;
use std::collections::HashMap;

use crate::common::{log_operation_complete, log_operation_start, log_progress, setup_database};
use log::info;
use tempfile::NamedTempFile;
use uuid::Uuid;

const PROVISIONAL_DEVIATION_THRESHOLD: f64 = 200.0;

pub async fn run_games_report(database_url: Option<String>) -> Result<()> {
    log_operation_start("games report generation");

    let mut conn = setup_database(database_url)
        .await
        .context("Failed to setup database connection")?;

    let games = load_non_bot_games(&mut conn).await?;
    let game_count = games.len();
    info!("Found {game_count} games (excluding bot games)");

    let mut temp_file = NamedTempFile::new().context("Failed to create temporary file for CSV writing")?;
    let mut writer = csv::Writer::from_writer(&mut temp_file);

    write_games_report_header(&mut writer)?;
    let (processed, errors) = process_games_for_report(games, &mut writer, &mut conn).await;

    writer.flush().context("Failed to flush CSV writer")?;
    drop(writer);

    persist_games_report_csv(temp_file)?;

    log_operation_complete("Games report generation", processed, errors);
    info!("Results written to games_report.csv");

    Ok(())
}

async fn process_game_with_users(
    game: &Game,
    users: &HashMap<Uuid, User>,
    conn: &mut db_lib::DbConn<'_>,
) -> Result<Vec<String>> {
    let white_user = users
        .get(&game.white_id)
        .context("White player not found in batch load")?;

    let black_user = users
        .get(&game.black_id)
        .context("Black player not found in batch load")?;

    let time_control_category = categorize_game_speed(&game.speed);

    let game_speed = game
        .speed
        .parse::<GameSpeed>()
        .context("Failed to parse game speed")?;
    
    let white_rating = Rating::for_uuid(&game.white_id, &game_speed, conn)
        .await
        .context("Failed to load white player rating")?;
    let black_rating = Rating::for_uuid(&game.black_id, &game_speed, conn)
        .await
        .context("Failed to load black player rating")?;

    let white_certainty = get_rating_certainty(white_rating.deviation);
    let black_certainty = get_rating_certainty(black_rating.deviation);

    let tournament_game = game.tournament_id.is_some();
    let tournament_id = game
        .tournament_id
        .map(|id| id.to_string())
        .unwrap_or_default();

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
        time_control_category,
        tournament_game.to_string(),
        tournament_id,
        game.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
    ])
}

fn get_rating_certainty(deviation: f64) -> String {
    use shared_types::RANKABLE_DEVIATION;

    if deviation <= RANKABLE_DEVIATION {
        "Rankable".to_string()
    } else if deviation <= PROVISIONAL_DEVIATION_THRESHOLD {
        "Provisional".to_string()
    } else {
        "Clueless".to_string()
    }
}

async fn load_non_bot_games(conn: &mut db_lib::DbConn<'_>) -> Result<Vec<Game>> {
    db_lib::schema::games::table
        .filter(db_lib::schema::games::game_type.ne("Bot"))
        .load(conn)
        .await
        .context("Failed to load games from database")
}

fn write_games_report_header(writer: &mut csv::Writer<&mut NamedTempFile>) -> Result<()> {
    writer.write_record([
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
        "game_created_at",
    ])
    .context("Failed to write CSV header")
}

async fn process_games_for_report(
    games: Vec<Game>,
    writer: &mut csv::Writer<&mut NamedTempFile>,
    conn: &mut db_lib::DbConn<'_>,
) -> (usize, usize) {
    let total_games = games.len();
    
    let user_ids: Vec<Uuid> = games
        .iter()
        .flat_map(|g| [g.white_id, g.black_id])
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    
    let users: HashMap<Uuid, User> = db_lib::schema::users::table
        .filter(db_lib::schema::users::id.eq_any(&user_ids))
        .load::<User>(conn)
        .await
        .ok()
        .unwrap_or_default()
        .into_iter()
        .map(|u| (u.id, u))
        .collect();
    
    let mut processed = 0;
    let mut errors = 0;

    for game in games {
        match process_game_with_users(&game, &users, conn).await {
            Ok(record) => {
                if let Err(e) = writer.write_record(&record) {
                    log::warn!("Failed to write game {} to CSV: {e}", game.nanoid);
                    errors += 1;
                }
            }
            Err(e) => {
                log::warn!("Failed to process game {}: {e}", game.nanoid);
                errors += 1;
            }
        }

        processed += 1;
        log_progress(processed, total_games, "Processing games");
    }

    (processed, errors)
}

fn persist_games_report_csv(temp_file: NamedTempFile) -> Result<()> {
    temp_file
        .persist("games_report.csv")
        .map(|_| ())
        .context("Failed to persist CSV file to games_report.csv")
}

fn categorize_game_speed(speed_str: &str) -> String {
    match speed_str.parse::<GameSpeed>() {
        Ok(speed) => match speed {
            GameSpeed::Bullet => "Bullet",
            GameSpeed::Blitz => "Blitz",
            GameSpeed::Rapid => "Rapid",
            GameSpeed::Classic => "Classic",
            GameSpeed::Correspondence => "Correspondence",
            _ => "Other",
        },
        Err(_) => "Unknown",
    }
    .to_string()
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
