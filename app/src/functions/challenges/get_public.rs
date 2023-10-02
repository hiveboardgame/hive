use super::challenge_response::ChallengeResponse;
use leptos::*;

#[server(GetPublicChallenges)]
pub async fn get_public_challenges() -> Result<Vec<ChallengeResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::models::challenge::Challenge;
    let pool = pool()?;
    let public = Challenge::get_public(&pool).await?;
    let mut challenges = Vec::new();
    for challenge in public {
        challenges.push(ChallengeResponse::from_model(&challenge, &pool).await?);
    }
    Ok(challenges)
}
