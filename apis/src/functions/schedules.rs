use crate::responses::{GameResponse, ScheduleResponse};
use chrono::{DateTime, Utc};
use leptos::prelude::*;
use shared_types::TournamentId;

#[server]
pub async fn get_tournament_schedules(
    tournament_id: TournamentId,
) -> Result<Vec<ScheduleResponse>, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{
        db_error::DbError,
        get_conn,
        models::{ScheduleOffer, Tournament},
    };

    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let tournament = Tournament::find_by_tournament_id(&tournament_id, &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    let can_view_all = match tournament
        .ensure_user_is_organizer_or_admin(&user_id, &mut conn)
        .await
    {
        Ok(()) => true,
        Err(DbError::Unauthorized) => false,
        Err(error) => return Err(ServerFnError::new(error)),
    };
    let schedules = ScheduleOffer::find_for_tournament(tournament.id, &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    let responses = ScheduleResponse::from_models_batch(schedules, &mut conn)
        .await
        .map_err(ServerFnError::new)?;

    Ok(responses
        .into_iter()
        .filter(|schedule| {
            schedule.proposer_id == user_id || schedule.opponent_id == user_id || can_view_all
        })
        .collect())
}

#[server]
pub async fn mark_schedule_seen(schedule_id: String) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::ScheduleOffer};
    use uuid::Uuid;
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let schedule_uuid = Uuid::parse_str(&schedule_id).map_err(ServerFnError::new)?;
    ScheduleOffer::mark_notified(schedule_uuid, user_id, &mut conn)
        .await
        .map_err(ServerFnError::new)?;

    Ok(())
}

#[server]
pub async fn get_upcoming_tournament_games(
) -> Result<Vec<(DateTime<Utc>, GameResponse)>, ServerFnError> {
    use crate::{functions::db::pool, responses::GameResponse};
    use db_lib::{get_conn, models::ScheduleOffer};

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;

    let upcoming_games = ScheduleOffer::get_upcoming_agreed_games(&mut conn)
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

    result.sort_by_key(|a| a.0);

    Ok(result)
}
