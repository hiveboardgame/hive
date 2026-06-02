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

// No `ensure_admin` here: this is read by the home page for every visitor
// to disable the Quick Play UI when realtime games are off. The value is
// operational, not sensitive.
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
    use crate::websocket::{MessageDestination, WebsocketData, WsHub};
    use actix_web::web::Data;
    use bytes::Bytes;
    use codee::{binary::MsgpackSerdeCodec, Encoder};
    use db_lib::get_conn;
    use std::sync::{atomic::Ordering, Arc};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_admin(&mut conn).await?;
    let req: actix_web::HttpRequest = leptos_actix::extract().await?;
    let ws_data = req
        .app_data::<Data<WebsocketData>>()
        .ok_or("Failed to get websocket data")
        .map_err(ServerFnError::new)?;
    // `swap` returns the previous value atomically; if it matches the new
    // value the toggle was a no-op (admin double-click, repeated dispatch)
    // and we skip the global broadcast + msgpack encode.
    let prev = ws_data
        .realtime_games_enabled
        .swap(enabled, Ordering::Relaxed);
    if prev == enabled {
        return Ok(());
    }
    if let Some(hub) = req.app_data::<Data<Arc<WsHub>>>() {
        let message = ServerResult::Ok(Box::new(ServerMessage::RealtimeEnabled(enabled)));
        if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
            hub.dispatch(&MessageDestination::Global, Bytes::from(serialized), None)
                .await;
        }
    }
    Ok(())
}
