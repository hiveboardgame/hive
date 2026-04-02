use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::{messages::{InternalServerMessage, MessageDestination}, WebsocketData},
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection};
use shared_types::TournamentId;
use std::sync::Arc;
use uuid::Uuid;

pub struct JoinHandler {
    tournament_id: TournamentId,
    user_id: Uuid,
    pool: DbPool,
}

impl JoinHandler {
    pub async fn new(
        tournament_id: TournamentId,
        user_id: Uuid,
        pool: &DbPool,
        _data: Arc<WebsocketData>,
    ) -> Result<Self> {
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
                async move { Ok(tournament.join(&self.user_id, tc).await?) }.scope_boxed()
            })
            .await?;
        let response = TournamentId(tournament.nanoid.clone());

        // Chat history is fetched by the client via REST when viewing the tournament chat.
        Ok(vec![
            InternalServerMessage {
                destination: MessageDestination::User(self.user_id),
                message: ServerMessage::Tournament(TournamentUpdate::Joined(response.clone())),
            },
            InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Tournament(TournamentUpdate::Modified(response)),
            },
        ])
    }
}
