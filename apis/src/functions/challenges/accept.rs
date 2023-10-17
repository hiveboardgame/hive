use crate::functions::games::game_response::GameStateResponse;
use leptos::*;

#[server]
pub async fn accept_challenge(nanoid: String) -> Result<Option<GameStateResponse>, ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::challenges::challenge_response::ChallengeError;
    use crate::functions::db::pool;
    use db_lib::models::challenge::Challenge;
    use db_lib::models::{
        game::{Game, NewGame},
        rating::Rating,
    };
    let pool = pool()?;
    let uuid = match uuid() {
        Ok(uuid) => uuid,
        Err(_) => {
            leptos_actix::redirect("/login");
            return Ok(None);
        }
    };
    let play_link = &format!("/play/{}", &nanoid);
    let challenge = Challenge::find_by_nanoid(&nanoid, &pool).await?;
    if challenge.challenger_id == uuid {
        return Err(ChallengeError::OwnChallenge.into());
    }
    let (white_id, black_id) = match challenge.color_choice.to_lowercase().as_str() {
        "black" => (uuid, challenge.challenger_id.clone()),
        "white" => (challenge.challenger_id.clone(), uuid),
        _ => {
            if rand::random() {
                (challenge.challenger_id.clone(), uuid)
            } else {
                (uuid, challenge.challenger_id.clone())
            }
        }
    };
    let new_game = NewGame {
        white_id: white_id.clone(),
        black_id: black_id.clone(),
        nanoid: challenge.nanoid.to_owned(),
        game_status: "NotStarted".to_string(),
        game_type: challenge.game_type.clone(),
        history: String::new(),
        game_control_history: String::new(),
        tournament_queen_rule: challenge.tournament_queen_rule,
        turn: 0,
        rated: challenge.rated,
        white_rating: Some(Rating::for_uuid(&white_id, &pool).await?.rating),
        black_rating: Some(Rating::for_uuid(&black_id, &pool).await?.rating),
        white_rating_change: None,
        black_rating_change: None,
    };
    let game = Game::create(&new_game, &pool).await?;
    challenge.delete(&pool).await?;
    leptos_actix::redirect(play_link);
    let game_state = GameStateResponse::new_from_db(&game, &pool).await?;
    Ok(Some(game_state))
}
