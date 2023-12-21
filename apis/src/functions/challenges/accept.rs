use crate::functions::games::game_response::GameStateResponse;
use leptos::*;

#[server]
pub async fn accept_challenge(nanoid: String) -> Result<Option<GameStateResponse>, ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::challenges::challenge_response::ChallengeError;
    use crate::functions::db::pool;
    use db_lib::models::challenge::Challenge;
    use db_lib::models::game::{Game, NewGame};
    let pool = pool()?;
    let uuid = match uuid() {
        Ok(uuid) => uuid,
        Err(_) => {
            leptos_actix::redirect("/login");
            return Ok(None);
        }
    };
    let play_link = &format!("/game/{}", &nanoid);
    let challenge = Challenge::find_by_nanoid(&nanoid, &pool).await?;
    if challenge.challenger_id == uuid {
        return Err(ChallengeError::OwnChallenge.into());
    }
    let (white_id, black_id) = match challenge.color_choice.to_lowercase().as_str() {
        "black" => (uuid, challenge.challenger_id),
        "white" => (challenge.challenger_id, uuid),
        _ => {
            if rand::random() {
                (challenge.challenger_id, uuid)
            } else {
                (uuid, challenge.challenger_id)
            }
        }
    };

    let new_game = NewGame::new(white_id, black_id, &challenge);
    let game = Game::create(&new_game, &pool).await?;
    challenge.delete(&pool).await?;
    leptos_actix::redirect(play_link);
    let game_state = GameStateResponse::new_from_db(&game, &pool)
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    Ok(Some(game_state))
}
