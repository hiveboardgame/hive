use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{db_error::DbError, get_conn, models::Tournament, DbPool};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use shared_types::{ScoringMode, TournamentId};
use uuid::Uuid;

pub struct UpdateScoringModeHandler {
    tournament_id: TournamentId,
    scoring_mode: ScoringMode,
    user_id: Uuid,
    pool: DbPool,
}

impl UpdateScoringModeHandler {
    pub async fn new(
        tournament_id: TournamentId,
        scoring_mode: ScoringMode,
        user_id: Uuid,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            tournament_id,
            scoring_mode,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let mut messages = Vec::new();
        
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;

        let _updated_tournament = conn
            .transaction::<_, DbError, _>(move |tc| {
                async move {
                    tournament
                        .update_scoring_mode_by_organizer(&self.scoring_mode, &self.user_id, tc)
                        .await
                }
                .scope_boxed()
            })
            .await?;

        let server_message = ServerMessage::Tournament(TournamentUpdate::Modified(self.tournament_id.clone()));

        messages.push(InternalServerMessage {
            destination: MessageDestination::Tournament(self.tournament_id.clone()),
            message: server_message,
        });

        Ok(messages)
    }
} 