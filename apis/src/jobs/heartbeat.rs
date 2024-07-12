use crate::websockets::lobby::Lobby;
use crate::websockets::messages::GameHB;
use actix::Addr;
use actix_web::web::Data;
use std::time::Duration;

pub fn run(lobby: Data<Addr<Lobby>>) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(3));
        loop {
            interval.tick().await;
            lobby.do_send(GameHB {});
        }
    });
}
