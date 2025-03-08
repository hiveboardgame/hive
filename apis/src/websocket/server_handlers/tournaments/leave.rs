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

pub struct LeaveHandler {
    tournament_id: TournamentId,
    user_id: Uuid,
    pool: DbPool,
}

impl LeaveHandler {
    pub async fn new(tournament_id: TournamentId, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            tournament_id,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        // TODO: This needs to go into a one commit
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;

        let tournament = conn
            .transaction::<_, anyhow::Error, _>(move |tc| {
                async move { Ok(tournament.leave(&self.user_id, tc).await?) }.scope_boxed()
            })
            .await?;
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Left(
                TournamentId(tournament.nanoid.clone()),
            )),
        }])
    }
}
