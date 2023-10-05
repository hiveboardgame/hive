use crate::functions::games::game_response::GameStateResponse;
use leptos::*;

#[server]
pub async fn accept_challenge(id: String) -> Result<GameStateResponse, ServerFnError> {
    use crate::functions::auth::identity::identity;
    use crate::functions::challenges::challenge_response::ChallengeError;
    use crate::functions::db::pool;
    use db_lib::models::challenge::Challenge;
    use db_lib::models::{
        game::{Game, NewGame},
        rating::Rating,
    };
    use uuid::Uuid;
    let pool = pool()?;
    let uid = identity()?.id()?;
    let uuid = Uuid::parse_str(&id)?;
    let challenge = Challenge::get(&uuid, &pool).await?;
    if challenge.challenger_uid == uid {
        return Err(ChallengeError::OwnChallenge.into());
    }
    let (white_uid, black_uid) = match challenge.color_choice.to_lowercase().as_str() {
        "black" => (uid, challenge.challenger_uid.clone()),
        "white" => (challenge.challenger_uid.clone(), uid),
        _ => {
            if rand::random() {
                (challenge.challenger_uid.clone(), uid)
            } else {
                (uid, challenge.challenger_uid.clone())
            }
        }
    };
    let new_game = NewGame {
        white_uid: white_uid.clone(),
        black_uid: black_uid.clone(),
        game_status: "NotStarted".to_string(),
        game_type: challenge.game_type.clone(),
        history: String::new(),
        game_control_history: String::new(),
        tournament_queen_rule: challenge.tournament_queen_rule,
        turn: 0,
        rated: challenge.rated,
        white_rating: Some(Rating::for_uid(&white_uid, &pool).await?.rating),
        black_rating: Some(Rating::for_uid(&black_uid, &pool).await?.rating),
        white_rating_change: None,
        black_rating_change: None,
    };
    let game = Game::create(&new_game, &pool).await?;
    challenge.delete(&pool).await?;
    GameStateResponse::new_from_db(&game, &pool).await
}
