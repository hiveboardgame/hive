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

pub enum BulkAdjudication {
    DoubleForfeitUnstarted,
    ResetAdjudicated,
}

pub struct BulkAdjudicateHandler {
    tournament_id: TournamentId,
    user_id: Uuid,
    pool: DbPool,
    action: BulkAdjudication,
}

impl BulkAdjudicateHandler {
    pub async fn new(
        tournament_id: TournamentId,
        user_id: Uuid,
        action: BulkAdjudication,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            tournament_id,
            user_id,
            pool: pool.clone(),
            action,
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament =
            Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;

        conn.transaction::<_, anyhow::Error, _>(move |tc| {
            let tournament = tournament.clone();
            let user_id = self.user_id;
            let action = &self.action;
            async move {
                match action {
                    BulkAdjudication::DoubleForfeitUnstarted => {
                        tournament
                            .double_forfeit_unstarted_games(&user_id, tc)
                            .await?;
                    }
                    BulkAdjudication::ResetAdjudicated => {
                        tournament.reset_adjudicated_games(&user_id, tc).await?;
                    }
                }
                Ok(())
            }
            .scope_boxed()
        })
        .await?;

        Ok(vec![InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Adjudicated(
                self.tournament_id.clone(),
            )),
        }])
    }
}
