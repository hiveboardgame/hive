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
        let _new_games = conn
            .transaction::<_, DbError, _>(async move |tc| {
                tournament.swiss_create_next_round(&self.user_id, tc).await
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
