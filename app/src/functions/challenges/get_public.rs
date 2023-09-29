#[get("/game/lobby")]
pub async fn get_lobby_challenges(
    pool: web::Data<DbPool>,
) -> Result<Json<Vec<GameChallengeResponse>>, ServerError> {
    let challenges = GameChallenge::get_public(&pool).await?;
    let mut resp = Vec::new();
    // TODO: batch all users into one query
    for challenge in challenges {
        resp.push(GameChallengeResponse::from_model(&challenge, &pool).await?);
    }
    Ok(web::Json(resp))
}
