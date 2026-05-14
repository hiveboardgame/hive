use crate::websocket::{
    messages::{HandlerOutput, SocketTx},
    WsHub,
};
use anyhow::Result;
use db_lib::{get_conn, DbPool};
use std::sync::Arc;
use uuid::Uuid;

/// `ClientRequest::Resync` handler — pushes a lobby snapshot back to the
/// requesting socket. Anonymous callers get the public subset.
pub struct ResyncHandler {
    hub: Arc<WsHub>,
    pool: DbPool,
    received_from: SocketTx,
    user_id: Uuid,
    authed: bool,
}

impl ResyncHandler {
    pub fn new(
        hub: Arc<WsHub>,
        pool: DbPool,
        received_from: SocketTx,
        user_id: Uuid,
        authed: bool,
    ) -> Self {
        Self {
            hub,
            pool,
            received_from,
            user_id,
            authed,
        }
    }

    pub async fn handle(self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        self.hub
            .send_lobby_snapshot(
                &mut conn,
                self.received_from.socket_id,
                self.user_id,
                &self.received_from.tx,
                self.authed,
            )
            .await;
        Ok(HandlerOutput::empty())
    }
}
