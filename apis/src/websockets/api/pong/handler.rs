use crate::ping::pings::Pings;
use actix_web::web::Data;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

pub struct PongHandler {
    user_id: Uuid,
    nonce: u64,
    pings: Data<Arc<RwLock<Pings>>>,
}

impl PongHandler {
    pub fn new(user_id: Uuid, nonce: u64, pings: Data<Arc<RwLock<Pings>>>) -> Self {
        Self {
            user_id,
            nonce,
            pings,
        }
    }

    pub fn handle(&mut self) {
        self.pings.write().unwrap().update(self.user_id, self.nonce);
    }
}
