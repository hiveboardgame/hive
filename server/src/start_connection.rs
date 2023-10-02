use crate::ws::WsConn;
use crate::lobby::Lobby;
use actix::Addr;
//use actix_identity::Identity;
use actix_web::{get, web::Data, web::Path, web::Payload, Error, HttpResponse, HttpRequest};
use actix_web_actors::ws;
use uuid::Uuid;

#[get("/ws/{group_id}")]
pub async fn start_connection(
    req: HttpRequest,
    stream: Payload,
    group_id: Path<Uuid>,
    srv: Data<Addr<Lobby>>,
//    identity: Identity,
) -> Result<HttpResponse, Error> {
    println!("Setting up WS");
    println!("Lobby from app data: {:?}", srv);
//  println!("Identity: {:?}", identity.id());
    let ws = WsConn::new(
        group_id.into_inner(),
        srv.get_ref().clone(),
    );

    let resp = ws::start(ws, &req, stream)?;
    Ok(resp)
}
