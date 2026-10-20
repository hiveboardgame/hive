use super::messages::Outbound;
use crate::{
    common::{LobbySnapshot, ServerMessage, ServerResult},
    responses::{ChallengeResponse, GameResponse, ScheduleResponse, UserResponse},
    websocket::{messages::SocketTx, WsHub},
};
use db_lib::{
    models::{
        Challenge,
        ScheduleOffer,
        Tournament,
        TournamentInvitation,
        TournamentOrganizerInvitation,
        User,
    },
    DbConn,
};
use log::error;
use shared_types::TournamentId;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
struct SocketDisconnected;

type SnapshotSection<T> = Result<T, SocketDisconnected>;

impl WsHub {
    /// Authoritative lobby snapshot for one socket — tournament invitations,
    /// schedule notifications, urgent games,
    /// challenges, and the online roster.
    ///
    /// Sent on initial connect and in response to `ClientRequest::Resync`. The
    /// receiving client applies the best-effort snapshot through `snapshot_apply`
    /// handlers that preserve any IDs touched by incremental updates during the
    /// resync window — without that, an incremental update that arrived between
    /// snapshot collection and snapshot delivery would be erased when the client
    /// applied the snapshot.
    ///
    /// The caller owns the DB connection so the connect path can reuse the
    /// one it already holds without round-tripping the pool. Section loaders
    /// re-check `is_socket_connected` around slow DB work so a fast disconnect
    /// does not fan out to a dead socket.
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
                tournament_invitations: self
                    .invitation_notification_snapshot(conn, user_id, socket, user)
                    .await?,
                tournament_organizer_invitations: self
                    .organizer_invitation_snapshot(conn, user_id, socket, user)
                    .await?,
                schedule_notifications: self
                    .schedule_notification_snapshot(conn, user_id, socket, user)
                    .await?,
                urgent_games: self
                    .urgent_games_snapshot(conn, user_id, socket, user)
                    .await?,
                challenges: self.challenge_snapshot(conn, user_id, socket, user).await?,
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
        self.send_own_state_via_tx(&socket.tx, socket.format, &mut Outbound::typed(&message));
        let paused = ServerResult::Ok(Box::new(ServerMessage::RealtimeGamesPaused(
            self.data.realtime_gate.is_paused(),
        )));
        self.send_own_state_via_tx(&socket.tx, socket.format, &mut Outbound::typed(&paused));
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

    async fn organizer_invitation_snapshot(
        &self,
        conn: &mut DbConn<'_>,
        user_id: Uuid,
        socket: &SocketTx,
        user: Option<&User>,
    ) -> SnapshotSection<Vec<TournamentId>> {
        let Some(user) = user else {
            return Ok(Vec::new());
        };
        let invitations = TournamentOrganizerInvitation::find_by_user(user.id, conn).await;
        self.ensure_socket_connected(user_id, socket)?;
        let result = match invitations {
            Ok(invitations) => {
                let ids = invitations
                    .into_iter()
                    .map(|row| row.tournament_id)
                    .collect::<Vec<_>>();
                Tournament::find_by_uuids(&ids, conn)
                    .await
                    .map(|tournaments| {
                        tournaments
                            .into_iter()
                            .map(|tournament| TournamentId(tournament.nanoid))
                            .collect()
                    })
            }
            Err(error) => Err(error),
        };
        self.ensure_socket_connected(user_id, socket)?;
        Ok(result.unwrap_or_else(|error| {
            error!("Failed to load organizer invitations for {user_id}: {error}");
            Vec::new()
        }))
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

        let schedules = ScheduleOffer::find_user_notifications(user.id, conn).await;
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
