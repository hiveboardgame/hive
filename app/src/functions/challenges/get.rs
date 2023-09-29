#[get("/game/challenge/{id}")]
pub async fn get_game_challenge(
    id: web::Path<Uuid>,
    pool: web::Data<DbPool>,
) -> Result<Json<GameChallengeResponse>, ServerError> {
    let challenge = GameChallenge::get(&id, &pool).await?;
    let response = GameChallengeResponse::from_model(&challenge, &pool).await?;
    Ok(web::Json(response))
}
