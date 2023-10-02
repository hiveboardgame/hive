use super::challenge_response::{ChallengeResponse, ColorChoice};
use hive_lib::game_type::GameType;
use leptos::*;

#[server(CreateChallenge)]
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
    use crate::functions::auth::identity::identity;
    use crate::functions::db::pool;
    use chrono::prelude::*;
    use db_lib::models::challenge::NewChallenge;
    use db_lib::models::{challenge::Challenge, user::User};
    let uid = identity()?.id()?;
    let pool = pool()?;
    let user = User::find_by_uid(&uid, &pool).await?;
    let new_challenge = NewChallenge {
        challenger_uid: user.uid,
        game_type: game_type.to_string(),
        rated,
        public,
        tournament_queen_rule,
        color_choice: color_choice.to_string(),
        created_at: Utc::now(),
    };
    let challenge = Challenge::create(&new_challenge, &pool).await?;
    ChallengeResponse::from_model(&challenge, &pool).await
}
