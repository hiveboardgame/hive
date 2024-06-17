use crate::responses::ChallengeResponse;
use leptos::*;
use uuid::Uuid;

#[server]
pub async fn get_challenge_by_uuid(id: Uuid) -> Result<ChallengeResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Challenge;
    let pool = pool()?;
    let mut conn = get_conn(&pool).await?;
    let challenge = Challenge::find_by_uuid(&id, &mut conn).await?;
    ChallengeResponse::from_model(&challenge, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server]
pub async fn get_challenge_by_nanoid(nanoid: String) -> Result<ChallengeResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Challenge;
    let pool = pool()?;
    let mut conn = get_conn(&pool).await?;
    let challenge = Challenge::find_by_nanoid(&nanoid, &mut conn).await?;
    ChallengeResponse::from_model(&challenge, &mut conn)
        .await
        .map_err(ServerFnError::new)
}
