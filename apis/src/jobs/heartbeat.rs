use crate::websockets::messages::GameHB;
use crate::websockets::ws_server::WsServer;
use actix::Addr;
use actix_web::web::Data;
use std::time::Duration;

pub fn run(ws_server: Data<Addr<WsServer>>) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(3));
        loop {
            interval.tick().await;
            ws_server.do_send(GameHB {});
        }
    });
}
