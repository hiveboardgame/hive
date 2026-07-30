use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{InternalServerMessage, MessageDestination, TournamentAudience},
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use diesel_async::AsyncConnection;
use shared_types::TournamentId;
use uuid::Uuid;

/// Sits a player out of the round about to be paired, for
/// `points_zero_point_bye` rather than the full-point pairing bye. Organizers
/// only, and only for a round that has not been paired yet.
pub struct ZeroPointByeHandler {
    tournament_id: TournamentId,
    player: Uuid,
    organizer: Uuid,
    pool: DbPool,
}

impl ZeroPointByeHandler {
    pub fn new(tournament_id: TournamentId, player: Uuid, organizer: Uuid, pool: &DbPool) -> Self {
        Self {
            tournament_id,
            player,
            organizer,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;

        conn.transaction::<_, anyhow::Error, _>(async move |tc| {
            tournament
                .grant_zero_point_bye(&self.player, &self.organizer, tc)
                .await?;
            Ok(())
        })
        .await?;

        Ok(vec![InternalServerMessage {
            destination: MessageDestination::Tournament {
                tournament_id: self.tournament_id.clone(),
                audience: TournamentAudience::Updates,
            },
            message: ServerMessage::Tournament(TournamentUpdate::StateChanged(
                self.tournament_id.clone(),
            )),
        }])
    }
}
