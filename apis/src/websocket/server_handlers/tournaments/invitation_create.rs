use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::{
        busybee::Busybee,
        messages::{InternalServerMessage, MessageDestination},
    },
};
use anyhow::Result;
use db_lib::{db_error::DbError, get_conn, models::Tournament, DbPool};
use diesel_async::scoped_futures::ScopedFutureExt;
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
    pub async fn new(
        tournament_id: TournamentId,
        user_id: Uuid,
        invitee: Uuid,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            tournament_id,
            user_id,
            invitee,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;
        let tournament = conn
            .transaction::<_, DbError, _>(move |tc| {
                async move {
                    tournament
                        .create_invitation(&self.user_id, &self.invitee, tc)
                        .await
                }
                .scope_boxed()
            })
            .await?;

        let msg = format!(
            "[Tournament Invitation](<https://hivegame.com/tournament/{}>) - You are invited to join tournament: {}",
            tournament.nanoid,
            tournament.name
        );

        /*if let Err(e) = Busybee::msg(self.invitee, msg).await {
            println!("Failed to send tournament invitation notification: {e}");
        }*/

        let response = TournamentId(tournament.nanoid.clone());
        Ok(vec![
            InternalServerMessage {
                destination: MessageDestination::User(self.invitee),
                message: ServerMessage::Tournament(TournamentUpdate::Invited(response.clone())),
            },
            InternalServerMessage {
                destination: MessageDestination::Tournament(response.clone()),
                message: ServerMessage::Tournament(TournamentUpdate::Modified(response)),
            },
        ])
    }
}
