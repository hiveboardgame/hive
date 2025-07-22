use crate::responses::GameResponse;
use chrono::{DateTime, Utc};
use leptos::prelude::*;

#[server]
pub async fn mark_schedule_seen(schedule_id: String) -> Result<(), ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Schedule;
    use uuid::Uuid;
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let schedule_uuid = Uuid::parse_str(&schedule_id).map_err(ServerFnError::new)?;
    Schedule::mark_notified(schedule_uuid, user_id, &mut conn)
        .await
        .map_err(ServerFnError::new)?;

    Ok(())
}

#[server]
pub async fn get_upcoming_tournament_games(
) -> Result<Vec<(DateTime<Utc>, GameResponse)>, ServerFnError> {
    use crate::functions::db::pool;
    use crate::responses::GameResponse;
    use db_lib::get_conn;
    use db_lib::models::Schedule;

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;

    let upcoming_games = Schedule::get_upcoming_agreed_games(&mut conn)
        .await
        .map_err(ServerFnError::new)?;

    let game_ids: Vec<uuid::Uuid> = upcoming_games.iter().map(|(game_id, _)| *game_id).collect();
    let game_responses = GameResponse::from_game_ids(&game_ids, &mut conn)
        .await
        .map_err(ServerFnError::new)?;

    let mut game_response_map = std::collections::HashMap::new();
    for game_response in game_responses {
        game_response_map.insert(game_response.uuid, game_response);
    }

    let mut result = Vec::new();
    for (game_id, start_t) in upcoming_games {
        if let Some(game_response) = game_response_map.remove(&game_id) {
            result.push((start_t, game_response));
        }
    }

    result.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(result)
}
