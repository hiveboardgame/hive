use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{InternalServerMessage, MessageDestination, TournamentAudience},
};
use anyhow::Result;
use db_lib::{db_error::DbError, get_conn, models::Tournament, DbPool};
use diesel_async::AsyncConnection;
use shared_types::TournamentId;
use uuid::Uuid;

pub struct SwissRoundHandler {
    tournament_id: TournamentId,
    user_id: Uuid,
    pool: DbPool,
}

impl SwissRoundHandler {
    pub fn new(tournament_id: TournamentId, user_id: Uuid, pool: &DbPool) -> Self {
        Self {
            tournament_id,
            user_id,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let mut messages = Vec::new();
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;
        let nanoid = tournament.nanoid.clone();
        // Advances whatever the format needs advancing: a Swiss or bracket
        // round, an arena pairing tick, or nothing if the round is still being
        // played. The outcome only says which of those happened.
        let _outcome = conn
            .transaction::<_, DbError, _>(async move |tc| {
                tournament.progress_by_organizer(&self.user_id, tc).await
            })
            .await?;
        messages.push(InternalServerMessage {
            destination: MessageDestination::Tournament {
                tournament_id: self.tournament_id.clone(),
                audience: TournamentAudience::Updates,
            },
            message: ServerMessage::Tournament(TournamentUpdate::StateChanged(TournamentId(
                nanoid,
            ))),
        });

        Ok(messages)
    }
}
