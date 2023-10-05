use super::challenge_response::ChallengeResponse;
use leptos::*;
use uuid::Uuid;

#[server]
pub async fn get_challenge_by_uuid(id: Uuid) -> Result<ChallengeResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::models::challenge::Challenge;
    let pool = pool()?;
    let challenge = Challenge::find_by_uuid(&id, &pool).await?;
    ChallengeResponse::from_model(&challenge, &pool).await
}

#[server]
pub async fn get_challenge_by_url(url: String) -> Result<ChallengeResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::models::challenge::Challenge;
    let pool = pool()?;
    let challenge = Challenge::find_by_url(&url, &pool).await?;
    ChallengeResponse::from_model(&challenge, &pool).await
}
