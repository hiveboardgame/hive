use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use diesel_async::AsyncConnection;
use shared_types::{TournamentId, TournamentStatus};
use uuid::Uuid;

pub struct JoinHandler {
    tournament_id: TournamentId,
    user_id: Uuid,
    pool: DbPool,
}

impl JoinHandler {
    pub fn new(tournament_id: TournamentId, user_id: Uuid, pool: &DbPool) -> Self {
        Self {
            tournament_id,
            user_id,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;
        // An arena admits players for as long as its clock runs, and seats them
        // by arrival rather than rating, so joining one that has already
        // started is a different operation. `Tournament::join` refuses outright
        // once a tournament is under way.
        let joining_running_arena = tournament.mode()?.is_arena()
            && tournament.status == TournamentStatus::InProgress.to_string();
        let response = TournamentId(tournament.nanoid.clone());
        conn.transaction::<_, anyhow::Error, _>(async move |tc| {
            if joining_running_arena {
                tournament.join_arena(&self.user_id, tc).await?;
            } else {
                tournament.join(&self.user_id, tc).await?;
            }
            Ok(())
        })
        .await?;
        Ok(vec![
            InternalServerMessage {
                destination: MessageDestination::User(self.user_id),
                message: ServerMessage::Tournament(TournamentUpdate::Joined(response.clone())),
            },
            InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Tournament(TournamentUpdate::StateChanged(response)),
            },
        ])
    }
}
