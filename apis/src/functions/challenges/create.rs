use super::challenge_response::ChallengeResponse;
use hive_lib::{color::ColorChoice, game_type::GameType};
use leptos::*;

#[server]
pub async fn create_challenge(
    public: bool,
    // Whether the game will be rated
    rated: bool,
    // Whether the game follows the "tournament" rules, i.e. the queen
    // cannot be played first.
    tournament_queen_rule: bool,
    // The challenger's color choice
    color_choice: ColorChoice,
    // Base, PLM, ...
    game_type: GameType,
) -> Result<ChallengeResponse, ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;
    use db_lib::models::challenge::NewChallenge;
    use db_lib::models::{challenge::Challenge, user::User};
    let uuid = uuid()?;
    let pool = pool()?;
    let user = User::find_by_uuid(&uuid, &pool).await?;
    let new_challenge = NewChallenge::new(
        user.id,
        game_type,
        rated,
        public,
        tournament_queen_rule,
        color_choice.to_string(),
    );

    let challenge = Challenge::create(&new_challenge, &pool).await?;
    let challenge_response = ChallengeResponse::from_model(&challenge, &pool).await;
    if !challenge.public {
        let redirect_path = &format!(
            "/challenge/{}",
            challenge_response
                .as_ref()
                .expect("challenge to have been created")
                .nanoid
        );
        leptos_actix::redirect(redirect_path);
    }
    challenge_response
}
