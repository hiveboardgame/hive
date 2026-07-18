use crate::{
    common::{LobbySnapshot, ServerMessage, ServerResult},
    responses::{ChallengeResponse, GameResponse, ScheduleResponse, UserResponse},
    websocket::{messages::SocketTx, WsHub},
};
use bytes::Bytes;
use codee::{binary::MsgpackSerdeCodec, Encoder};
use db_lib::{
    models::{Challenge, Game, Schedule, Tournament, TournamentInvitation, User},
    DbConn,
};
use hive_lib::GameStatus;
use log::error;
use shared_types::{GameId, TournamentId};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
struct SocketDisconnected;

type SnapshotSection<T> = Result<T, SocketDisconnected>;

impl WsHub {
    /// Authoritative lobby snapshot for one socket — tournament invitations,
    /// schedule notifications, urgent games, challenges, TV set, and the
    /// online roster.
    ///
    /// Sent on initial connect and in response to `ClientRequest::Resync`. The
    /// receiving client applies the best-effort snapshot through `snapshot_apply`
    /// handlers that preserve any IDs touched by incremental updates during the
    /// resync window — without that, an incremental update that arrived between
    /// snapshot collection and snapshot delivery would be erased when the client
    /// applied the snapshot.
    ///
    /// The caller owns the DB connection so the connect path can reuse the
    /// one it already holds without round-tripping the pool. Each `await`
    /// re-checks `is_socket_connected` so a fast disconnect doesn't waste
    /// the remaining DB work or fan out to a dead socket.
    pub(in crate::websocket) async fn send_lobby_snapshot(
        &self,
        conn: &mut DbConn<'_>,
        user_id: Uuid,
        socket: &SocketTx,
        user: Option<&User>,
    ) {
        let snapshot = match async {
            self.ensure_socket_connected(user_id, socket)?;
            let snapshot = LobbySnapshot {
                realtime_enabled: self.data.realtime_gate.enabled().await,
                tournament_invitations: self
                    .invitation_notification_snapshot(conn, user_id, socket, user)
                    .await?,
                schedule_notifications: self
                    .schedule_notification_snapshot(conn, user_id, socket, user)
                    .await?,
                urgent_games: self
                    .urgent_games_snapshot(conn, user_id, socket, user)
                    .await?,
                challenges: self.challenge_snapshot(conn, user_id, socket, user).await?,
                tv_games: self.tv_snapshot(conn, user_id, socket).await?,
                online_users: self.online_roster_snapshot(conn, user_id, socket).await?,
            };
            self.ensure_socket_connected(user_id, socket)?;
            Ok(snapshot)
        }
        .await
        {
            Ok(snapshot) => snapshot,
            Err(SocketDisconnected) => return,
        };
        let message = ServerResult::Ok(Box::new(ServerMessage::LobbySnapshot(Box::new(snapshot))));
        if let Ok(serialized) = MsgpackSerdeCodec::encode(&message) {
            self.send_own_state_via_tx(&socket.tx, &Bytes::from(serialized));
        }
    }

    fn ensure_socket_connected(
        &self,
        user_id: Uuid,
        socket: &SocketTx,
    ) -> Result<(), SocketDisconnected> {
        if self.is_socket_connected(user_id, socket.socket_id) {
            Ok(())
        } else {
            Err(SocketDisconnected)
        }
    }

    async fn invitation_notification_snapshot(
        &self,
        conn: &mut DbConn<'_>,
        user_id: Uuid,
        socket: &SocketTx,
        user: Option<&User>,
    ) -> SnapshotSection<Vec<TournamentId>> {
        let Some(user) = user else {
            return Ok(Vec::new());
        };

        let invitations = TournamentInvitation::find_by_user(&user.id, conn).await;
        self.ensure_socket_connected(user_id, socket)?;
        let invitations = match invitations {
            Ok(invitations) => invitations,
            Err(e) => {
                error!(
                    "Failed to get tournament invitations for user {}: {e}",
                    user_id
                );
                return Ok(Vec::new());
            }
        };
        let tournament_uuids: Vec<Uuid> = invitations
            .into_iter()
            .map(|invitation| invitation.tournament_id)
            .collect();
        if tournament_uuids.is_empty() {
            return Ok(Vec::new());
        }

        let tournaments = Tournament::find_by_uuids(&tournament_uuids, conn).await;
        self.ensure_socket_connected(user_id, socket)?;
        match tournaments {
            Ok(tournaments) => Ok(tournaments
                .into_iter()
                .map(|tournament| TournamentId(tournament.nanoid))
                .collect()),
            Err(e) => {
                error!(
                    "Failed to get tournament invitation snapshot for user {}: {e}",
                    user_id
                );
                Ok(Vec::new())
            }
        }
    }

    async fn schedule_notification_snapshot(
        &self,
        conn: &mut DbConn<'_>,
        user_id: Uuid,
        socket: &SocketTx,
        user: Option<&User>,
    ) -> SnapshotSection<Vec<ScheduleResponse>> {
        let Some(user) = user else {
            return Ok(Vec::new());
        };

        let schedules = Schedule::find_user_notifications(user.id, conn).await;
        self.ensure_socket_connected(user_id, socket)?;
        let schedules = match schedules {
            Ok(schedules) => schedules,
            Err(e) => {
                error!(
                    "Failed to get schedule notification snapshot for user {}: {e}",
                    user_id
                );
                return Ok(Vec::new());
            }
        };

        let responses = ScheduleResponse::from_models_batch(schedules, conn).await;
        self.ensure_socket_connected(user_id, socket)?;
        let responses = match responses {
            Ok(responses) => responses,
            Err(e) => {
                error!(
                    "Failed to build schedule notification snapshot for user {}: {e}",
                    user_id
                );
                return Ok(Vec::new());
            }
        };
        Ok(responses)
    }

    async fn urgent_games_snapshot(
        &self,
        conn: &mut DbConn<'_>,
        user_id: Uuid,
        socket: &SocketTx,
        user: Option<&User>,
    ) -> SnapshotSection<Vec<GameResponse>> {
        let Some(user) = user else {
            return Ok(Vec::new());
        };

        // `get_games_with_notifications` filters finished=false server-side;
        // `from_games_batch` coalesces user/tournament lookups into ~2 queries
        // total rather than 3 per game.
        let urgent = user.get_games_with_notifications(conn).await;
        self.ensure_socket_connected(user_id, socket)?;
        let urgent = match urgent {
            Ok(urgent) => urgent,
            Err(e) => {
                error!("Failed to get urgent games for user {}: {e}", user_id);
                return Ok(Vec::new());
            }
        };
        let games = GameResponse::from_games_batch(urgent, conn).await;
        self.ensure_socket_connected(user_id, socket)?;
        match games {
            Ok(games) => Ok(games),
            Err(e) => {
                error!(
                    "Failed to build urgent game snapshot for user {}: {e}",
                    user_id
                );
                Ok(Vec::new())
            }
        }
    }

    async fn challenge_snapshot(
        &self,
        conn: &mut DbConn<'_>,
        user_id: Uuid,
        socket: &SocketTx,
        user: Option<&User>,
    ) -> SnapshotSection<Vec<ChallengeResponse>> {
        // One bulk Challenges payload so the client REPLACEs its local map —
        // anything that disappeared while the tab was hidden is cleared.
        let mut challenge_models = Vec::new();
        if let Some(user) = user {
            let public = Challenge::get_public_exclude_user(user.id, conn).await;
            self.ensure_socket_connected(user_id, socket)?;
            match public {
                Ok(public) => challenge_models.extend(public),
                Err(e) => {
                    error!(
                        "Failed to get public challenge snapshot for user {}: {e}",
                        user_id
                    );
                }
            }

            let own = Challenge::get_own(user.id, conn).await;
            self.ensure_socket_connected(user_id, socket)?;
            match own {
                Ok(own) => challenge_models.extend(own),
                Err(e) => {
                    error!(
                        "Failed to get own challenge snapshot for user {}: {e}",
                        user_id
                    );
                }
            }

            let direct = Challenge::direct_challenges(user.id, conn).await;
            self.ensure_socket_connected(user_id, socket)?;
            match direct {
                Ok(direct) => challenge_models.extend(direct),
                Err(e) => {
                    error!(
                        "Failed to get direct challenge snapshot for user {}: {e}",
                        user_id
                    );
                }
            }
        } else {
            let public = Challenge::get_public(conn).await;
            self.ensure_socket_connected(user_id, socket)?;
            match public {
                Ok(public) => challenge_models.extend(public),
                Err(e) => {
                    error!(
                        "Failed to get anonymous challenge snapshot for user {}: {e}",
                        user_id
                    );
                }
            }
        }

        let responses = ChallengeResponse::from_models_batch(challenge_models, conn).await;
        self.ensure_socket_connected(user_id, socket)?;
        match responses {
            Ok(responses) => Ok(responses),
            Err(e) => {
                error!(
                    "Failed to build challenge snapshot for user {}: {e}",
                    user_id
                );
                Ok(Vec::new())
            }
        }
    }

    async fn tv_snapshot(
        &self,
        conn: &mut DbConn<'_>,
        user_id: Uuid,
        socket: &SocketTx,
    ) -> SnapshotSection<Vec<GameResponse>> {
        // `last_tv_broadcast` doubles as the "currently on TV" set. Empty
        // payload still matters — the client REPLACEs, clearing stale entries.
        // We filter out games where the requesting user is a player so the
        // client doesn't have to (and we don't waste payload bytes).
        self.ensure_socket_connected(user_id, socket)?;
        let tv_ids: Vec<GameId> = self
            .last_tv_broadcast
            .iter()
            .map(|entry| entry.key().clone())
            .collect();
        if tv_ids.is_empty() {
            return Ok(Vec::new());
        };

        let games = Game::find_by_nanoids(&tv_ids, conn).await;
        self.ensure_socket_connected(user_id, socket)?;
        let games = match games {
            Ok(games) => games,
            Err(e) => {
                error!("Failed to get TV game snapshot for user {}: {e}", user_id);
                return Ok(Vec::new());
            }
        };
        let in_progress_status = GameStatus::InProgress.to_string();
        let in_progress: Vec<Game> = games
            .into_iter()
            .filter(|g| {
                g.game_status == in_progress_status
                    && g.white_id != user_id
                    && g.black_id != user_id
            })
            .collect();
        let tv_responses = GameResponse::from_games_batch(in_progress, conn).await;
        self.ensure_socket_connected(user_id, socket)?;
        match tv_responses {
            Ok(tv_responses) => Ok(tv_responses),
            Err(e) => {
                error!("Failed to build TV game snapshot for user {}: {e}", user_id);
                Ok(Vec::new())
            }
        }
    }

    async fn online_roster_snapshot(
        &self,
        conn: &mut DbConn<'_>,
        user_id: Uuid,
        socket: &SocketTx,
    ) -> SnapshotSection<Vec<UserResponse>> {
        // Collect the online roster last. It uses one batched query and the
        // final snapshot is a single channel send, so a large lobby does not
        // overflow the socket buffer with one message per user.
        self.ensure_socket_connected(user_id, socket)?;
        let existing_user_ids: Vec<Uuid> = self.sessions.iter().map(|e| *e.key()).collect();
        if existing_user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let map = UserResponse::from_uuids(&existing_user_ids, conn).await;
        self.ensure_socket_connected(user_id, socket)?;
        match map {
            Ok(map) => Ok(map.into_values().collect()),
            Err(e) => {
                error!(
                    "Failed to build online roster snapshot for user {}: {e}",
                    user_id
                );
                Ok(Vec::new())
            }
        }
    }
}
