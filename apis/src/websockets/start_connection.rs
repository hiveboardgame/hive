use crate::websockets::{chat::Chats, connection::WsConnection, lobby::Lobby};
use actix::Addr;
use actix_identity::Identity;
use actix_web::{get, web::Data, web::Payload, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use db_lib::{get_conn, models::User, DbPool};
use uuid::Uuid;

use super::tournament_game_start::TournamentGameStart;

#[get("/ws/")]
pub async fn start_connection(
    req: HttpRequest,
    stream: Payload,
    srv: Data<Addr<Lobby>>,
    chat_storage: Data<Chats>,
    game_start: Data<TournamentGameStart>,
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
                                pool.get_ref().clone(),
                            );
                            let resp = ws::start(ws, &req, stream)?;
                            return Ok(resp);
                        }
                    }
                    Err(err) => println!("Could not establish database connection: {err}"),
                }
            }
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
        pool.get_ref().clone(),
    );

    let resp = ws::start(ws, &req, stream)?;
    Ok(resp)
}
