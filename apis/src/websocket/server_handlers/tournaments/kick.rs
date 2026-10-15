use super::membership_removed_messages;
use crate::websocket::messages::HandlerOutput;
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use diesel_async::AsyncConnection;
use shared_types::TournamentId;
use uuid::Uuid;

pub struct KickHandler {
    tournament_id: TournamentId,
    organizer: Uuid,
    player: Uuid,
    pool: DbPool,
}

impl KickHandler {
    pub fn new(tournament_id: TournamentId, organizer: Uuid, player: Uuid, pool: &DbPool) -> Self {
        Self {
            tournament_id,
            organizer,
            player,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let (tournament, retained_chat_access) = conn
            .transaction::<_, anyhow::Error, _>(async move |tc| {
                let tournament =
                    Tournament::find_by_tournament_id_for_update(&self.tournament_id, tc).await?;
                let tournament = tournament.kick(&self.organizer, &self.player, tc).await?;
                let retained_chat_access = tournament.retains_chat_access(self.player, tc).await?;
                Ok((tournament, retained_chat_access))
            })
            .await?;
        Ok(
            membership_removed_messages(&tournament, self.player, retained_chat_access, &mut conn)
                .await,
        )
    }
}
