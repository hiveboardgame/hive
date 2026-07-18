use leptos::prelude::*;

#[server]
pub async fn count_active_realtime_clocks() -> Result<i64, ServerFnError> {
    use crate::functions::{auth::identity::ensure_admin, db::pool};
    use db_lib::{get_conn, models::Game};

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_admin(&mut conn).await?;
    Ok(Game::count_active_realtime_clocks(&mut conn).await?)
}

#[server]
pub async fn set_realtime_enabled(enabled: bool) -> Result<bool, ServerFnError> {
    use crate::{
        common::{ServerMessage, ServerResult},
        functions::{auth::identity::ensure_admin, db::pool},
        websocket::{MessageDestination, WsHub},
    };
    use actix_web::web::Data;
    use bytes::Bytes;
    use codee::{binary::MsgpackSerdeCodec, Encoder};
    use db_lib::get_conn;
    use std::sync::Arc;

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_admin(&mut conn).await?;
    drop(conn);

    let request: actix_web::HttpRequest = leptos_actix::extract().await?;
    let hub = request
        .app_data::<Data<Arc<WsHub>>>()
        .cloned()
        .ok_or_else(|| ServerFnError::new("WebSocket hub is unavailable"))?;

    let transition = actix_rt::spawn(async move {
        let broadcast_hub = hub.clone();
        hub.data
            .realtime_gate
            .transition(enabled, async move {
                let result = ServerResult::Ok(Box::new(ServerMessage::RealtimeEnabled(enabled)));
                if let Ok(serialized) = MsgpackSerdeCodec::encode(&result) {
                    broadcast_hub
                        .dispatch(&MessageDestination::Global, Bytes::from(serialized))
                        .await;
                }
            })
            .await;
        enabled
    });
    Ok(transition
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))?)
}
