#[post("/game/challenge/{id}/accept")]
pub async fn accept_game_challenge(
    id: web::Path<Uuid>,
    auth_user: AuthenticatedUser,
    pool: web::Data<DbPool>,
) -> Result<Json<GameStateResponse>, ServerError> {
    let challenge = GameChallenge::get(&id, &pool).await?;
    if challenge.challenger_uid == auth_user.uid {
        return Err(ChallengeError::OwnChallenge.into());
    }
    let (white_uid, black_uid) = match challenge.color_choice.to_lowercase().as_str() {
        "black" => (auth_user.uid, challenge.challenger_uid.clone()),
        "white" => (challenge.challenger_uid.clone(), auth_user.uid),
        _ => {
            if rand::random() {
                (challenge.challenger_uid.clone(), auth_user.uid)
            } else {
                (auth_user.uid, challenge.challenger_uid.clone())
            }
        }
    };
    let new_game = NewGame {
        white_uid: white_uid.clone(),
        black_uid: black_uid.clone(),
        game_status: "NotStarted".to_string(),
        game_type: challenge.game_type.clone(),
        history: String::new(),
        game_control_history: String::new(),
        tournament_queen_rule: challenge.tournament_queen_rule,
        turn: 0,
        rated: challenge.rated,
        white_rating: Some(Rating::for_uid(&white_uid, &pool).await?.rating),
        black_rating: Some(Rating::for_uid(&black_uid, &pool).await?.rating),
        white_rating_change: None,
        black_rating_change: None,
    };
    let game = Game::create(&new_game, &pool).await?;
    challenge.delete(&pool).await?;
    let resp = GameStateResponse::new_from_db(&game, &pool).await?;
    Ok(web::Json(resp))
}
