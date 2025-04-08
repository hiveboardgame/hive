use crate::functions::games::get::server_fn::codec;
use crate::responses::GameResponse;
use hive_lib::GameStatus;
use leptos::prelude::*;
use shared_types::{GameId, GamesQueryOptions};
use uuid::Uuid;

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_game_from_uuid(game_id: Uuid) -> Result<GameResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    GameResponse::new_from_uuid(game_id, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_game_from_nanoid(game_id: GameId) -> Result<GameResponse, ServerFnError> {
    use crate::functions::db::pool;
    use crate::websocket::UserToGame;
    use crate::websocket::WebsocketData;
    use crate::websocket::WsServer;
    use actix::Addr;
    use actix_web::web::Data;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let ret = GameResponse::new_from_game_id(&game_id, &mut conn)
        .await
        .map_err(ServerFnError::new);
    if ret.clone().is_ok_and(|game| !matches!(game.game_status, GameStatus::Finished(_))) {
        let req: actix_web::HttpRequest = leptos_actix::extract().await?;
        let ws_data = req
            .app_data::<Data<WebsocketData>>()
            .ok_or("Failed to get ws adress")
            .map_err(ServerFnError::new)?
            .get_ref();
        let server = req
            .app_data::<Data<Addr<WsServer>>>()
            .ok_or("Failed to get ws adress")
            .map_err(ServerFnError::new)?
            .get_ref();
        server.do_send(UserToGame {
            user_id: *ws_data.uid.read().unwrap(),
            game_id: game_id.0.clone(),
        });
    }
    ret
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_batch_from_options(
    options: GamesQueryOptions,
) -> Result<Vec<GameResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    GameResponse::vec_from_options(options, &mut conn)
        .await
        .map_err(ServerFnError::new)
}
