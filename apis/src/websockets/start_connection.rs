use super::tournament_game_start::TournamentGameStart;
use crate::{
    lag_tracking::lags::Lags,
    ping::pings::Pings,
    websockets::{chat::Chats, ws_connection::WsConnection, ws_server::WsServer},
};
use actix::Addr;
use actix_identity::Identity;
use actix_web::{get, web::Data, web::Payload, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use db_lib::{get_conn, models::User, DbPool};
use uuid::Uuid;

#[get("/ws/")]
pub async fn start_connection(
    req: HttpRequest,
    stream: Payload,
    srv: Data<Addr<WsServer>>,
    chat_storage: Data<Chats>,
    game_start: Data<TournamentGameStart>,
    pings: Data<Pings>,
    lags: Data<Lags>,
    pool: Data<DbPool>,
    identity: Option<Identity>,
) -> Result<HttpResponse, Error> {
    if let Some(id) = identity {
        if let Ok(id_string) = id.id() {
            if let Ok(uuid) = Uuid::parse_str(&id_string) {
                match get_conn(&pool).await {
                    Ok(mut conn) => {
                        if let Ok(user) = User::find_by_uuid(&uuid, &mut conn).await {
                            println!("Welcome {}!", user.username);
                            let ws = WsConnection::new(
                                Some(uuid),
                                Some(user.username),
                                Some(user.admin),
                                srv.get_ref().clone(),
                                chat_storage.clone(),
                                game_start.clone(),
                                pings.clone(),
                                lags.clone(),
                                pool.get_ref().clone(),
                            );
                            let resp = ws::start(ws, &req, stream)?;
                            return Ok(resp);
                        }
                    }
                    Err(err) => println!("Could not establish database connection: {err}"),
                }
            } else {
                println!("Can't parse to Uuid");
            }
        } else {
            println!("Wrong id");
        }
    };

    println!("Welcome Anonymous!");
    let ws = WsConnection::new(
        None,
        None,
        None,
        srv.get_ref().clone(),
        chat_storage.clone(),
        game_start.clone(),
        pings.clone(),
        lags.clone(),
        pool.get_ref().clone(),
    );

    let resp = ws::start(ws, &req, stream)?;
    Ok(resp)
}
