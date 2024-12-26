use actix_web::{web, post, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize)]
pub struct PlayRequest {
    bot_id: String,
    game_id: String,
    move_: String,
}

#[cfg(feature = "ssr")]
pub async fn play(req: web::Json<PlayRequest>) -> HttpResponse {
    println!(
        "Bot {} playing game {}: {}",
        req.bot_id, req.game_id, req.move_
    );

    HttpResponse::Ok().body(format!(
        "Bot {} played game {}",
        req.bot_id, req.game_id
    ))
}

