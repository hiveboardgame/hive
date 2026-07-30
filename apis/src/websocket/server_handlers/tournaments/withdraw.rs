use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{InternalServerMessage, MessageDestination, TournamentAudience},
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use diesel_async::AsyncConnection;
use shared_types::TournamentId;
use uuid::Uuid;

/// Which way round: leaving a running tournament, or being put back into one.
#[derive(Debug, Clone, Copy)]
pub enum Membership {
    Withdraw,
    Reinstate,
}

pub struct WithdrawHandler {
    tournament_id: TournamentId,
    /// The player joining or leaving, who is not necessarily the actor — an
    /// organizer may withdraw somebody, and only an organizer may reinstate.
    player: Uuid,
    actor: Uuid,
    membership: Membership,
    pool: DbPool,
}

impl WithdrawHandler {
    pub fn new(
        tournament_id: TournamentId,
        player: Uuid,
        actor: Uuid,
        membership: Membership,
        pool: &DbPool,
    ) -> Self {
        Self {
            tournament_id,
            player,
            actor,
            membership,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;

        // Withdrawal forfeits games and reshapes the field, so it has to be one
        // unit with the row lock the engine takes inside it.
        conn.transaction::<_, anyhow::Error, _>(async move |tc| {
            match self.membership {
                Membership::Withdraw => {
                    tournament
                        .withdraw_player(&self.player, &self.actor, tc)
                        .await?
                }
                Membership::Reinstate => {
                    tournament
                        .reinstate_player(&self.player, &self.actor, tc)
                        .await?
                }
            };
            Ok(())
        })
        .await?;

        let to_player = match self.membership {
            Membership::Withdraw => TournamentUpdate::Withdrawn(self.tournament_id.clone()),
            Membership::Reinstate => TournamentUpdate::Reinstated(self.tournament_id.clone()),
        };

        Ok(vec![
            InternalServerMessage {
                destination: MessageDestination::User(self.player),
                message: ServerMessage::Tournament(to_player),
            },
            InternalServerMessage {
                destination: MessageDestination::Tournament {
                    tournament_id: self.tournament_id.clone(),
                    audience: TournamentAudience::Updates,
                },
                message: ServerMessage::Tournament(TournamentUpdate::StateChanged(
                    self.tournament_id.clone(),
                )),
            },
        ])
    }
}
