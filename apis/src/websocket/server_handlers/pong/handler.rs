use crate::websocket::lag_tracking::Pings;
use actix_web::web::Data;
use uuid::Uuid;

pub struct PongHandler {
    user_id: Uuid,
    nonce: u64,
    pings: Data<Pings>,
}

impl PongHandler {
    pub fn new(user_id: Uuid, nonce: u64, pings: Data<Pings>) -> Self {
        Self {
            user_id,
            nonce,
            pings,
        }
    }

    pub fn handle(&self) {
        self.pings.update(self.user_id, self.nonce);
    }
}
