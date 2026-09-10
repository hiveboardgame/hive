use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{InternalServerMessage, MessageDestination, SocketTx},
};
use anyhow::Result;
use db_lib::{
    get_conn,
    helpers::run_serializable,
    models::{NewTournament, Tournament},
    DbPool,
};
use shared_types::{TournamentDetails, TournamentId};
use std::sync::Arc;
use uuid::Uuid;

pub struct CreateHandler {
    details: TournamentDetails,
    user_id: Uuid,
    received_from: SocketTx,
    pool: DbPool,
}

impl CreateHandler {
    pub fn new(
        details: TournamentDetails,
        user_id: Uuid,
        received_from: SocketTx,
        pool: &DbPool,
    ) -> Self {
        Self {
            details,
            user_id,
            received_from,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let new_tournament = NewTournament::new(self.details.clone())?;
        let new_tournament = Arc::new(new_tournament);
        let user_id = self.user_id;
        let mut conn = get_conn(&self.pool).await?;
        let tournament = run_serializable(&mut conn, move |tc| {
            let new_tournament = Arc::clone(&new_tournament);
            Box::pin(async move { Tournament::create(user_id, &new_tournament, tc).await })
        })
        .await?;

        let tournament_id = TournamentId(tournament.nanoid);
        // Only the submitting socket may treat Created as an acknowledgement
        // and navigate. Catalog consumers refresh independently.
        Ok(vec![
            InternalServerMessage {
                destination: MessageDestination::Direct(self.received_from.clone()),
                message: ServerMessage::Tournament(TournamentUpdate::Created(
                    tournament_id.clone(),
                )),
            },
            InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Tournament(TournamentUpdate::CatalogChanged(tournament_id)),
            },
        ])
    }
}
