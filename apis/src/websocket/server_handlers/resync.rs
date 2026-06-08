use crate::websocket::{
    messages::{HandlerOutput, SocketTx},
    WsHub,
};
use anyhow::Result;
use db_lib::{get_conn, models::User, DbPool};
use log::error;
use std::sync::Arc;
use uuid::Uuid;

/// `ClientRequest::Resync` handler — pushes a lobby snapshot back to the
/// requesting socket. Anonymous callers get the public subset.
///
/// Server-side cooldown via `WsHub::allow_resync` protects against
/// `visibilitychange` + `pageshow` double-fires and against a misbehaving
/// client that would otherwise let one tab burn pool connections in a tight
/// loop.
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
        if !self.hub.allow_resync(self.received_from.socket_id) {
            return Ok(HandlerOutput::empty());
        }
        let mut conn = get_conn(&self.pool).await?;
        if !self
            .hub
            .is_socket_connected(self.user_id, self.received_from.socket_id)
        {
            return Ok(HandlerOutput::empty());
        }
        let user = if self.authed {
            match User::find_by_uuid(&self.user_id, &mut conn).await {
                Ok(user) => Some(user),
                Err(e) => {
                    error!(
                        "Failed to load authenticated websocket user {} for resync: {e}",
                        self.user_id
                    );
                    return Ok(HandlerOutput::empty());
                }
            }
        } else {
            None
        };
        self.hub
            .send_lobby_snapshot(&mut conn, self.user_id, &self.received_from, user.as_ref())
            .await;
        Ok(HandlerOutput::empty())
    }
}
