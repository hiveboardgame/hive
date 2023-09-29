#[post("/game/challenge")]
pub async fn create_game_challenge(
    game: web::Json<NewGameChallengeRequest>,
    auth_user: AuthenticatedUser,
    pool: web::Data<DbPool>,
) -> Result<Json<GameChallengeResponse>, ServerError> {
    let challenge = GameChallenge::create(&auth_user, &game, &pool).await?;
    let response = GameChallengeResponse::from_model(&challenge, &pool).await?;
    Ok(web::Json(response))
}
