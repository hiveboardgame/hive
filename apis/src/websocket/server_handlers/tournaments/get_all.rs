use crate::{
    common::{ServerMessage, TournamentResponseDepth, TournamentUpdate},
    responses::{TournamentAbstractResponse, TournamentResponse},
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbConn, DbPool};
use shared_types::TournamentSortOrder;
use uuid::Uuid;

pub struct GetAllHandler {
    user_id: Uuid,
    depth: TournamentResponseDepth,
    pool: DbPool,
}

impl GetAllHandler {
    pub async fn new(user_id: Uuid, depth: TournamentResponseDepth, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            user_id,
            depth,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        match self.depth {
            TournamentResponseDepth::Full => self.handle_full(&mut conn).await,
            TournamentResponseDepth::Abstract => self.handle_abstract(&mut conn).await,
        }
    }

    async fn handle_full(&self, conn: &mut DbConn<'_>) -> Result<Vec<InternalServerMessage>> {
        let mut responses = vec![];
        let tournaments = Tournament::get_all(TournamentSortOrder::CreatedAtDesc, conn).await?;
        for tournament in tournaments {
            responses.push(TournamentResponse::from_model(&tournament, conn).await?);
        }
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::User(self.user_id),
            message: ServerMessage::Tournament(TournamentUpdate::Tournaments(responses)),
        }])
    }

    async fn handle_abstract(&self, conn: &mut DbConn<'_>) -> Result<Vec<InternalServerMessage>> {
        let mut responses = vec![];
        let tournaments = Tournament::get_all(TournamentSortOrder::CreatedAtDesc, conn).await?;
        for tournament in tournaments {
            responses.push(TournamentAbstractResponse::from_model(&tournament, conn).await?);
        }
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::User(self.user_id),
            message: ServerMessage::Tournament(TournamentUpdate::AbstractTournaments(responses)),
        }])
    }
}
