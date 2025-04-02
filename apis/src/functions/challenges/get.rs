use crate::responses::ChallengeResponse;
use leptos::prelude::*;
use server_fn::codec;
use shared_types::ChallengeId;
use uuid::Uuid;

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_challenge_by_uuid(id: Uuid) -> Result<ChallengeResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Challenge;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let challenge = Challenge::find_by_uuid(&id, &mut conn).await?;
    ChallengeResponse::from_model(&challenge, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_challenge(challenge_id: ChallengeId) -> Result<ChallengeResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Challenge;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let challenge = Challenge::find_by_challenge_id(&challenge_id, &mut conn).await?;
    ChallengeResponse::from_model(&challenge, &mut conn)
        .await
        .map_err(ServerFnError::new)
}
