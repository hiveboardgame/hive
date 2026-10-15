use crate::{
    common::ServerMessage,
    websocket::{
        messages::{HandlerOutput, InternalServerMessage, MessageDestination, SocketTx},
        WsHub,
    },
};
use anyhow::Result;
use db_lib::{db_error::DbError, get_conn, models::User, DbPool};
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

    fn retry(&self) -> HandlerOutput {
        HandlerOutput::from(vec![InternalServerMessage {
            destination: MessageDestination::Direct(self.received_from.clone()),
            message: ServerMessage::ResyncRetry {
                retry_after_ms: 500,
            },
        }])
    }

    pub async fn handle(self) -> Result<HandlerOutput> {
        let Some(_snapshot_guard) = self
            .hub
            .try_begin_lobby_snapshot(self.received_from.socket_id)
        else {
            return Ok(self.retry());
        };
        if !self.hub.allow_resync(self.received_from.socket_id) {
            return Ok(self.retry());
        }
        let mut conn = match get_conn(&self.pool).await {
            Ok(conn) => conn,
            Err(error) => {
                log::error!("Lobby resync database connection failed: {error}");
                return Ok(self.retry());
            }
        };
        if !self
            .hub
            .is_socket_connected(self.user_id, self.received_from.socket_id)
        {
            return Ok(HandlerOutput::empty());
        }
        let user = if self.authed {
            match User::find_active_by_uuid(&self.user_id, &mut conn).await {
                Ok(user) => Some(user),
                Err(e) => {
                    error!(
                        "Failed to load authenticated websocket user {} for resync: {e}",
                        self.user_id
                    );
                    return if matches!(e, DbError::NotFound { .. }) {
                        Err(e.into())
                    } else {
                        Ok(self.retry())
                    };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::WebsocketData;
    use db_lib::get_pool;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn throttled_or_inflight_resync_explicitly_replies_to_its_requesting_socket() {
        let pool = get_pool("postgresql://test:test@127.0.0.1:9/test")
            .await
            .expect("lazy pool");
        let hub = WsHub::new(Arc::new(WebsocketData::default()), pool.clone());
        let socket_id = Uuid::new_v4();
        let (tx, _rx) = mpsc::channel(2);
        let socket = SocketTx { socket_id, tx };
        assert!(hub.allow_resync(socket_id));
        let output = ResyncHandler::new(
            hub.clone(),
            pool.clone(),
            socket.clone(),
            Uuid::new_v4(),
            false,
        )
        .handle()
        .await
        .expect("throttled resync response");
        assert!(matches!(&output.messages[..], [InternalServerMessage {
            destination: MessageDestination::Direct(destination),
            message: ServerMessage::ResyncRetry { .. },
        }] if destination.socket_id == socket_id && destination.tx.same_channel(&socket.tx)));
        let busy_id = Uuid::new_v4();
        let guard = hub
            .try_begin_lobby_snapshot(busy_id)
            .expect("first snapshot lease");
        let output = ResyncHandler::new(
            hub.clone(),
            pool,
            SocketTx {
                socket_id: busy_id,
                tx: socket.tx.clone(),
            },
            Uuid::new_v4(),
            false,
        )
        .handle()
        .await
        .expect("inflight resync response");
        assert!(matches!(&output.messages[..], [InternalServerMessage {
            destination: MessageDestination::Direct(destination),
            message: ServerMessage::ResyncRetry { .. },
        }] if destination.socket_id == busy_id));
        drop(guard);
    }
}
