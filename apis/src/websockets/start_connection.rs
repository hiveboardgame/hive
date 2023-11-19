use crate::websockets::{lobby::Lobby, ws::WsConn};
use actix::Addr;
use actix_identity::Identity;
use actix_web::{get, web::Data, web::Payload, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use db_lib::{DbPool, models::user::User};
use uuid::Uuid;

#[get("/ws/")]
pub async fn start_connection(
    req: HttpRequest,
    stream: Payload,
    // group_id: Path<String>,
    srv: Data<Addr<Lobby>>,
    pool: Data<DbPool>,
    user: Option<Identity>,
) -> Result<HttpResponse, Error> {
    let ws = match user {
        Some(user) => {
            // TODO: handle the unwraps
            let uuid = Uuid::parse_str(&user.id().unwrap()).unwrap();
            let username = User::find_by_uuid(&uuid, &pool).await.unwrap().username;
            println!("Welcome {}!", username);
            WsConn::new(
                Some(uuid),
                username,
                srv.get_ref().clone(),
                pool.get_ref().clone(),
            )
        }
        None => {
            println!("Welcome Anonymous!");
            WsConn::new(None, String::from("Anonymous"), srv.get_ref().clone(), pool.get_ref().clone())
        }
    };

    let resp = ws::start(ws, &req, stream)?;
    Ok(resp)
}
