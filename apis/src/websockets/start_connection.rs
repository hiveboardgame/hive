use crate::websockets::{connection::WsConnection, lobby::Lobby};
use actix::Addr;
use actix_identity::Identity;
use actix_web::{get, web::Data, web::Payload, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use db_lib::{models::user::User, DbPool};
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
            let uuid = Uuid::parse_str(&user.id().expect("User has id")).expect("Valid uuid");
            let username = User::find_by_uuid(&uuid, &pool)
                .await
                .expect("Username to exist")
                .username;
            println!("Welcome {}!", username);
            WsConnection::new(
                Some(uuid),
                username,
                srv.get_ref().clone(),
                pool.get_ref().clone(),
            )
        }
        None => {
            println!("Welcome Anonymous!");
            WsConnection::new(
                None,
                String::from("Anonymous"),
                srv.get_ref().clone(),
                pool.get_ref().clone(),
            )
        }
    };

    let resp = ws::start(ws, &req, stream)?;
    Ok(resp)
}
