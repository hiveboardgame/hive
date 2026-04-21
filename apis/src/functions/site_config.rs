use leptos::prelude::*;

#[server]
pub async fn count_ongoing_realtime_games() -> Result<i64, ServerFnError> {
    use crate::functions::{auth::identity::ensure_admin, db::pool};
    use db_lib::{get_conn, models::Game};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_admin(&mut conn).await?;
    Ok(Game::count_ongoing_realtime(&mut conn).await?)
}

#[server]
pub async fn get_realtime_enabled() -> Result<bool, ServerFnError> {
    use crate::websocket::WebsocketData;
    use actix_web::web::Data;
    use std::sync::atomic::Ordering;
    let req: actix_web::HttpRequest = leptos_actix::extract().await?;
    let ws_data = req
        .app_data::<Data<WebsocketData>>()
        .ok_or("Failed to get websocket data")
        .map_err(ServerFnError::new)?;
    Ok(ws_data.realtime_games_enabled.load(Ordering::Relaxed))
}

#[server]
pub async fn set_realtime_enabled(enabled: bool) -> Result<(), ServerFnError> {
    use crate::common::{ServerMessage, ServerResult};
    use crate::functions::{auth::identity::ensure_admin, db::pool};
    use crate::websocket::{ClientActorMessage, MessageDestination, WebsocketData, WsServer};
    use actix::Addr;
    use actix_web::web::Data;
    use codee::{binary::MsgpackSerdeCodec, Encoder};
    use db_lib::get_conn;
    use std::sync::atomic::Ordering;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_admin(&mut conn).await?;
    let req: actix_web::HttpRequest = leptos_actix::extract().await?;
    let ws_data = req
        .app_data::<Data<WebsocketData>>()
        .ok_or("Failed to get websocket data")
        .map_err(ServerFnError::new)?;
    ws_data
        .realtime_games_enabled
        .store(enabled, Ordering::Relaxed);
    if let Some(ws_server) = req.app_data::<Data<Addr<WsServer>>>() {
        let message = ServerResult::Ok(Box::new(ServerMessage::RealtimeEnabled(enabled)));
        if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
            ws_server.do_send(ClientActorMessage::new(
                None,
                MessageDestination::Global,
                &serialized,
            ));
        }
    }
    Ok(())
}
