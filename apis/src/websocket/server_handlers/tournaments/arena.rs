use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{InternalServerMessage, MessageDestination, TournamentAudience},
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use diesel_async::AsyncConnection;
use shared_types::{ArenaEventKind, TournamentId};
use uuid::Uuid;

/// Joining a running arena, or stepping out of and back into its pairing pool.
/// One handler because the engine treats them as the same kind of timeline
/// event, recorded against the arena clock rather than the wall clock.
pub struct ArenaHandler {
    tournament_id: TournamentId,
    user_id: Uuid,
    /// `None` is a join; the rest are breaks.
    kind: Option<ArenaEventKind>,
    pool: DbPool,
}

impl ArenaHandler {
    pub fn join(tournament_id: TournamentId, user_id: Uuid, pool: &DbPool) -> Self {
        Self {
            tournament_id,
            user_id,
            kind: None,
            pool: pool.clone(),
        }
    }

    pub fn break_(
        tournament_id: TournamentId,
        user_id: Uuid,
        kind: ArenaEventKind,
        pool: &DbPool,
    ) -> Self {
        Self {
            tournament_id,
            user_id,
            kind: Some(kind),
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;

        // Each of these validates against a replayed arena and only persists if
        // the engine accepts it, under the tournament's row lock — so the whole
        // thing has to be one transaction or the lock buys nothing.
        conn.transaction::<_, anyhow::Error, _>(async move |tc| {
            match self.kind {
                None => {
                    tournament.join_arena(&self.user_id, tc).await?;
                }
                Some(ArenaEventKind::Pause) => {
                    tournament.pause_in_arena(&self.user_id, tc).await?;
                }
                Some(ArenaEventKind::Resume) => {
                    tournament.resume_in_arena(&self.user_id, tc).await?;
                }
                Some(ArenaEventKind::Withdraw) => {
                    tournament.withdraw_from_arena(&self.user_id, tc).await?;
                }
            }
            Ok(())
        })
        .await?;

        let to_user = match self.kind {
            None => TournamentUpdate::Joined(self.tournament_id.clone()),
            Some(ArenaEventKind::Withdraw) => {
                TournamentUpdate::Withdrawn(self.tournament_id.clone())
            }
            // Pausing does not change membership, so the player's own view only
            // needs the same refresh everyone else gets.
            Some(_) => TournamentUpdate::StateChanged(self.tournament_id.clone()),
        };

        Ok(vec![
            InternalServerMessage {
                destination: MessageDestination::User(self.user_id),
                message: ServerMessage::Tournament(to_user),
            },
            InternalServerMessage {
                destination: MessageDestination::Tournament {
                    tournament_id: self.tournament_id.clone(),
                    audience: TournamentAudience::Updates,
                },
                message: ServerMessage::Tournament(TournamentUpdate::StateChanged(
                    self.tournament_id.clone(),
                )),
            },
        ])
    }
}
