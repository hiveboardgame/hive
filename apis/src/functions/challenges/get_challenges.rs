use super::challenge_response::ChallengeResponse;
use leptos::*;
use uuid::Uuid;

#[server]
pub async fn get_public_challenges(
    user: Option<Uuid>,
) -> Result<Vec<ChallengeResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::models::challenge::Challenge;
    let pool = pool()?;
    let public = if user.is_some() {
        Challenge::get_public_exclude_user(&pool, user.expect("User is some")).await?
    } else {
        Challenge::get_public(&pool).await?
    };
    let mut challenges = Vec::new();
    for challenge in public {
        challenges.push(ChallengeResponse::from_model(&challenge, &pool).await?);
    }
    Ok(challenges)
}

#[server]
pub async fn get_own_challenges(
    user: Option<Uuid>,
) -> Result<Option<Vec<ChallengeResponse>>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::models::challenge::Challenge;
    let pool = pool()?;
    if user.is_none() {
        return Ok(None);
    }
    let own = Challenge::get_own(&pool, user.expect("User is some here")).await?;
    let mut challenges = Vec::new();
    for challenge in own {
        challenges.push(ChallengeResponse::from_model(&challenge, &pool).await?);
    }
    Ok(Some(challenges))
}
