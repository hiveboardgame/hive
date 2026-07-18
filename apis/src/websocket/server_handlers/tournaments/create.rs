use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{NewTournament, Tournament},
    DbPool,
};
use diesel_async::AsyncConnection;
use shared_types::{TournamentDetails, TournamentId};
use uuid::Uuid;

pub struct CreateHandler {
    details: TournamentDetails,
    user_id: Uuid,
    pool: DbPool,
}

impl CreateHandler {
    pub fn new(details: TournamentDetails, user_id: Uuid, pool: &DbPool) -> Self {
        Self {
            details,
            user_id,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let new_tournament = NewTournament::new(self.details.clone())?;
        let tournament = conn
            .transaction::<_, anyhow::Error, _>(async move |tc| {
                Ok(Tournament::create(self.user_id, &new_tournament, tc).await?)
            })
            .await?;

        Ok(vec![InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Created(TournamentId(
                tournament.nanoid.clone(),
            ))),
        }])
    }
}
