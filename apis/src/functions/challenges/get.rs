use crate::responses::ChallengeResponse;
use leptos::*;
use uuid::Uuid;

#[server]
pub async fn get_challenge_by_uuid(id: Uuid) -> Result<ChallengeResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::models::Challenge;
    let pool = pool()?;
    let challenge = Challenge::find_by_uuid(&id, &pool).await?;
    ChallengeResponse::from_model(&challenge, &pool)
        .await
        .map_err(ServerFnError::new)
}

#[server]
pub async fn get_challenge_by_nanoid(nanoid: String) -> Result<ChallengeResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::models::Challenge;
    let pool = pool()?;
    let challenge = Challenge::find_by_nanoid(&nanoid, &pool).await?;
    ChallengeResponse::from_model(&challenge, &pool)
        .await
        .map_err(ServerFnError::new)
}
