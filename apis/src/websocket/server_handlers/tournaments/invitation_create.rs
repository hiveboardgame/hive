use crate::{
    common::{ServerMessage, TournamentUpdate},
    notifications::{notify, Event},
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{db_error::DbError, get_conn, models::Tournament, DbPool};
use diesel_async::AsyncConnection;
use shared_types::TournamentId;
use uuid::Uuid;

pub struct InvitationCreate {
    tournament_id: TournamentId,
    user_id: Uuid,
    invitee: Uuid,
    pool: DbPool,
}

impl InvitationCreate {
    pub fn new(tournament_id: TournamentId, user_id: Uuid, invitee: Uuid, pool: &DbPool) -> Self {
        Self {
            tournament_id,
            user_id,
            invitee,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;
        let tournament = conn
            .transaction::<_, DbError, _>(async move |tc| {
                tournament
                    .create_invitation(&self.user_id, &self.invitee, tc)
                    .await
            })
            .await?;

        notify(Event::TournamentInvite {
            recipient: self.invitee,
            tournament_name: tournament.name.clone(),
            tournament_nanoid: tournament.nanoid.clone(),
        });

        let response = TournamentId(tournament.nanoid.clone());
        Ok(vec![
            InternalServerMessage {
                destination: MessageDestination::User(self.invitee),
                message: ServerMessage::Tournament(TournamentUpdate::Invited(response.clone())),
            },
            InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Tournament(TournamentUpdate::StateChanged(response)),
            },
        ])
    }
}
