use super::membership_removed_messages;
use crate::websocket::messages::InternalServerMessage;
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use diesel_async::AsyncConnection;
use shared_types::TournamentId;
use uuid::Uuid;

pub struct KickHandler {
    tournament_id: TournamentId,
    organizer: Uuid,
    player: Uuid,
    pool: DbPool,
}

impl KickHandler {
    pub fn new(tournament_id: TournamentId, organizer: Uuid, player: Uuid, pool: &DbPool) -> Self {
        Self {
            tournament_id,
            organizer,
            player,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;
        let tournament = conn
            .transaction::<_, anyhow::Error, _>(async move |tc| {
                Ok(tournament.kick(&self.organizer, &self.player, tc).await?)
            })
            .await?;
        Ok(membership_removed_messages(
            TournamentId(tournament.nanoid.clone()),
            self.player,
        ))
    }
}
