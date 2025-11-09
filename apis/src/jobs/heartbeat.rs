use crate::common::{GameUpdate, ServerMessage};
use crate::responses::HeartbeatResponse;
use crate::websocket::{ServerData,InternalServerMessage,MessageDestination};
use actix_web::web::Data;
use db_lib::{DbPool, get_conn, models::Game};
use hive_lib::GameStatus;
use shared_types::TimeMode;
use std::time::Duration;

pub fn run(ws_server: Data<ServerData>, pool: DbPool) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(3));
        loop {
            interval.tick().await;
            if let Ok(mut conn) = get_conn(&pool).await {
                for game_id in ws_server.active_games() {
                    if let Ok(game) = Game::find_by_game_id(&game_id, &mut conn).await {
                        if game.game_status == GameStatus::InProgress.to_string()
                            && game.time_mode != TimeMode::Untimed.to_string()
                        {
                            if let Ok((id, white, black)) = game.get_heartbeat() {
                                let hb = HeartbeatResponse {
                                    game_id: id,
                                    white_time_left: white,
                                    black_time_left: black,
                                };
                                
                                let message = ServerMessage::Game(
                                    Box::new(GameUpdate::Heartbeat(hb)),
                                );
                                let message = InternalServerMessage {
                                    destination: MessageDestination::Game(game_id),
                                    message 
                                };
                                let _ = ws_server.send(message);
                            }
                        }
                    }
                }
            }
        }
    });
}
