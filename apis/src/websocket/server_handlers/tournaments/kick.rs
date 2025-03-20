use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use diesel_async::scoped_futures::ScopedFutureExt;
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
    pub async fn new(
        tournament_id: TournamentId,
        organizer: Uuid,
        player: Uuid,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            tournament_id,
            organizer,
            player,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        // TODO: This needs to go into a one commit
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;
        let tournament = conn
            .transaction::<_, anyhow::Error, _>(move |tc| {
                async move { Ok(tournament.kick(&self.organizer, &self.player, tc).await?) }
                    .scope_boxed()
            })
            .await?;
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Modified(TournamentId(
                tournament.nanoid.clone(),
            ))),
        }])
    }
}
