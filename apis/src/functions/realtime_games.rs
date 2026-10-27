use crate::security::csrf::CsrfClient;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use shared_types::GameId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealtimeGamesStatus {
    pub paused: bool,
    pub live_games: usize,
    pub starting: usize,
    pub started_while_paused: Vec<StartedWhilePaused>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartedWhilePaused {
    pub game_id: GameId,
    pub white: String,
    pub black: String,
}

#[cfg(feature = "ssr")]
async fn admin_hub(
) -> Result<actix_web::web::Data<std::sync::Arc<crate::websocket::WsHub>>, ServerFnError> {
    use crate::functions::{auth::identity::ensure_admin, db::pool};
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_admin(&mut conn).await?;
    Ok(leptos_actix::extract().await?)
}

#[server(client = CsrfClient)]
pub async fn status() -> Result<RealtimeGamesStatus, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{
        get_conn,
        models::{Game, User},
    };
    use std::collections::HashMap;

    let hub = admin_hub().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let paused_since = hub.data.realtime_gate.paused_since();
    // Read before the games: a start that finishes in between is then in the games instead.
    let starting = hub.data.realtime_gate.starting();
    let live = Game::live_realtime(&mut conn).await?;
    let late = live
        .iter()
        .filter(|game| paused_since.is_some_and(|since| game.created_at >= since))
        .collect::<Vec<_>>();
    let player_ids = late
        .iter()
        .flat_map(|game| [game.white_id, game.black_id])
        .collect::<Vec<_>>();
    let usernames = if player_ids.is_empty() {
        HashMap::new()
    } else {
        User::find_by_uuids(&player_ids, &mut conn)
            .await?
            .into_iter()
            .map(|user| (user.id, user.username))
            .collect()
    };
    let username = |id| usernames.get(&id).cloned().unwrap_or_default();
    Ok(RealtimeGamesStatus {
        paused: paused_since.is_some(),
        live_games: live.len(),
        starting,
        started_while_paused: late
            .into_iter()
            .map(|game| StartedWhilePaused {
                game_id: GameId(game.nanoid.clone()),
                white: username(game.white_id),
                black: username(game.black_id),
            })
            .collect(),
    })
}

#[server(client = CsrfClient)]
pub async fn set_paused(paused: bool) -> Result<bool, ServerFnError> {
    use crate::{
        common::ServerMessage,
        functions::auth::identity::uuid,
        websocket::{InternalServerMessage, MessageDestination},
    };
    let hub = admin_hub().await?;
    let admin_id = uuid().await?;
    if hub.data.realtime_gate.set_paused(paused) {
        log::warn!(
            "Realtime games {} by admin {admin_id}",
            if paused { "paused" } else { "resumed" }
        );
        hub.dispatch_message(InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::RealtimeGamesPaused(paused),
        })
        .await;
    }
    Ok(hub.data.realtime_gate.is_paused())
}
