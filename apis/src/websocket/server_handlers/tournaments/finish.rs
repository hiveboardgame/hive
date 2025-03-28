use crate::{
    common::{ServerMessage, TournamentUpdate},
    responses::TournamentResponse,
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{db_error::DbError, get_conn, models::Tournament, DbPool};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use shared_types::TournamentId;
use uuid::Uuid;

pub struct FinishHandler {
    tournament_id: TournamentId,
    user_id: Uuid,
    pool: DbPool,
}

impl FinishHandler {
    pub async fn new(tournament_id: TournamentId, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            tournament_id,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let mut messages = Vec::new();
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;
        let tournament = conn
            .transaction::<_, DbError, _>(move |tc| {
                async move { tournament.finish(&self.user_id, tc).await }.scope_boxed()
            })
            .await?;
        let tournament_response = TournamentResponse::from_model(&tournament, &mut conn).await?;

        let players = tournament.players(&mut conn).await?;
        for player in players {
            messages.push(InternalServerMessage {
                destination: MessageDestination::User(player.id),
                message: ServerMessage::Tournament(TournamentUpdate::Finished(
                    tournament_response.clone(),
                )),
            });
        }

        Ok(messages)
    }
}
