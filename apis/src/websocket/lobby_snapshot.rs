use crate::{
    common::{ChallengeUpdate, GameUpdate, ServerMessage, ServerResult},
    responses::{ChallengeResponse, GameResponse, UserResponse},
    websocket::WsHub,
};
use bytes::Bytes;
use codee::{binary::MsgpackSerdeCodec, Encoder};
use db_lib::{
    models::{Challenge, Game, User},
    DbConn,
};
use log::error;
use shared_types::GameId;
use tokio::sync::mpsc;
use uuid::Uuid;

impl WsHub {
    /// Authoritative lobby snapshot — urgent games (authed only), challenges,
    /// TV set, online roster — sent to one socket. Driven by both the
    /// initial connect and the `Resync` request; client handlers REPLACE
    /// rather than merge so state that vanished during tab suspension clears.
    /// The caller owns the DB connection so the connect path can reuse the
    /// one it already holds without round-tripping the pool.
    pub(in crate::websocket) async fn send_lobby_snapshot(
        &self,
        conn: &mut DbConn<'_>,
        socket_id: Uuid,
        user_id: Uuid,
        tx: &mpsc::Sender<Bytes>,
        authed: bool,
    ) {
        if !self.is_socket_connected(user_id, socket_id) {
            return;
        }

        // from_games_batch coalesces user/tournament lookups into ~2 queries
        // total rather than 3 per game.
        if authed {
            if let Ok(user) = User::find_by_uuid(&user_id, conn).await {
                let urgent = match user.get_games_with_notifications(conn).await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("Failed to get urgent games for user {user_id}: {e}");
                        Vec::new()
                    }
                };
                if let Ok(games) = GameResponse::from_games_batch(urgent, conn).await {
                    let message = ServerResult::Ok(Box::new(ServerMessage::Game(Box::new(
                        GameUpdate::Urgent(games),
                    ))));
                    if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                        self.send_own_state_via_tx(tx, &Bytes::from(serialized));
                    }
                }
            }
        }

        // One bulk Challenges payload so the client REPLACEs its local map
        // — anything that disappeared while the tab was hidden is cleared.
        let mut responses = Vec::new();
        if authed {
            if let Ok(challenges) = Challenge::get_public_exclude_user(user_id, conn).await {
                for challenge in challenges {
                    if let Ok(response) = ChallengeResponse::from_model(&challenge, conn).await
                    {
                        responses.push(response);
                    }
                }
            }
            if let Ok(challenges) = Challenge::get_own(user_id, conn).await {
                for challenge in challenges {
                    if let Ok(response) = ChallengeResponse::from_model(&challenge, conn).await
                    {
                        responses.push(response);
                    }
                }
            }
            if let Ok(challenges) = Challenge::direct_challenges(user_id, conn).await {
                for challenge in challenges {
                    if let Ok(response) = ChallengeResponse::from_model(&challenge, conn).await
                    {
                        responses.push(response);
                    }
                }
            }
        } else if let Ok(challenges) = Challenge::get_public(conn).await {
            for challenge in challenges {
                if let Ok(response) = ChallengeResponse::from_model(&challenge, conn).await {
                    responses.push(response);
                }
            }
        }
        let message = ServerResult::Ok(Box::new(ServerMessage::Challenge(
            ChallengeUpdate::Challenges(responses),
        )));
        if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
            self.send_own_state_via_tx(tx, &Bytes::from(serialized));
        }

        // last_tv_broadcast doubles as the "currently on TV" set. Empty
        // payload still matters — client REPLACEs, clearing stale entries.
        let tv_ids: Vec<GameId> = self
            .last_tv_broadcast
            .iter()
            .map(|entry| entry.key().clone())
            .collect();
        let tv_responses = if tv_ids.is_empty() {
            Vec::new()
        } else {
            let in_progress: Vec<Game> = Game::find_by_nanoids(&tv_ids, conn)
                .await
                .unwrap_or_default()
                .into_iter()
                .filter(|g| !g.finished)
                .collect();
            GameResponse::from_games_batch(in_progress, conn)
                .await
                .unwrap_or_default()
        };
        let message = ServerResult::Ok(Box::new(ServerMessage::Game(Box::new(
            GameUpdate::TvSnapshot(tv_responses),
        ))));
        if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
            self.send_own_state_via_tx(tx, &Bytes::from(serialized));
        }

        // Online roster — last so own-state messages above are never crowded
        // out in a large lobby. One batched query + one channel send, so a
        // 150-user lobby doesn't overflow the 128-slot socket buffer.
        let existing_user_ids: Vec<Uuid> = self.sessions.iter().map(|e| *e.key()).collect();
        if !existing_user_ids.is_empty() {
            if let Ok(map) = UserResponse::from_uuids(&existing_user_ids, conn).await {
                let users: Vec<UserResponse> = map.into_values().collect();
                let message = ServerResult::Ok(Box::new(ServerMessage::UserStatusBatch(users)));
                if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
                    self.send_own_state_via_tx(tx, &Bytes::from(serialized));
                }
            }
        }
    }
}
