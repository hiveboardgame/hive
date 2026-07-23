use crate::responses::ChallengeResponse;
use leptos::prelude::*;
use server_fn::codec;
use shared_types::ChallengeId;
use uuid::Uuid;

#[cfg(feature = "ssr")]
async fn current_user_id() -> Option<Uuid> {
    use crate::functions::auth::identity::uuid;

    uuid().await.ok()
}

#[cfg(feature = "ssr")]
fn can_read_challenge(challenge: &db_lib::models::Challenge, viewer_id: Option<Uuid>) -> bool {
    use shared_types::ChallengeVisibility;

    if challenge.visibility != ChallengeVisibility::Direct.to_string() {
        return true;
    }

    let Some(viewer_id) = viewer_id else {
        return false;
    };

    challenge.challenger_id == viewer_id
        || challenge
            .opponent_id
            .is_some_and(|opponent_id| opponent_id == viewer_id)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_challenge_by_uuid(id: Uuid) -> Result<ChallengeResponse, ServerFnError> {
    use crate::{functions::db::pool, responses::ChallengeResponseDb};
    use db_lib::{get_conn, models::Challenge};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let challenge = Challenge::find_by_uuid(&id, &mut conn).await?;
    if !can_read_challenge(&challenge, current_user_id().await) {
        return Err(ServerFnError::new("Challenge not found"));
    }
    ChallengeResponse::from_model(&challenge, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_challenge(challenge_id: ChallengeId) -> Result<ChallengeResponse, ServerFnError> {
    use crate::{functions::db::pool, responses::ChallengeResponseDb};
    use db_lib::{get_conn, models::Challenge};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let challenge = Challenge::find_by_challenge_id(&challenge_id, &mut conn).await?;
    if !can_read_challenge(&challenge, current_user_id().await) {
        return Err(ServerFnError::new("Challenge not found"));
    }
    ChallengeResponse::from_model(&challenge, &mut conn)
        .await
        .map_err(ServerFnError::new)
}
