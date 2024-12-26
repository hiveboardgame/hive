use actix_web::{web, post, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize)]
struct GameRequest {
    bot_id: String,
    game_id: String,
    move_: String,
}

#[cfg(feature = "ssr")]
#[post("/api/bot/games")]
pub async fn games(req: web::Json<GameRequest>) -> HttpResponse {
    println!(
        "Bot {} playing game {}: {}",
        req.bot_id, req.game_id, req.move_
    );

    HttpResponse::Ok().body(format!(
        "Bot {} played game {}",
        req.bot_id, req.game_id
    ))
}

