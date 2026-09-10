use super::{append_public_tournament_patches, PublicTournamentSection};
use crate::{
    common::{ServerMessage, TournamentUpdate},
    notifications::{notify, Event},
    websocket::messages::{HandlerOutput, InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, helpers::run_serializable, models::Tournament, DbPool};
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
        let tournament_id = self.tournament_id.clone();
        let user_id = self.user_id;
        let invitee = self.invitee;
        let outcome = run_serializable(&mut conn, move |tc| {
            let tournament_id = tournament_id.clone();
            Box::pin(async move {
                let tournament =
                    Tournament::find_by_tournament_id_for_update(&tournament_id, tc).await?;
                tournament.create_invitation(&user_id, &invitee, tc).await
            })
        })
        .await?;

        if !outcome.changed {
            return Ok(Vec::new());
        }
        let tournament = outcome.tournament;

        notify(Event::TournamentInvite {
            recipient: self.invitee,
            tournament_name: tournament.name.clone(),
            tournament_nanoid: tournament.nanoid.clone(),
        });

        let response = TournamentId(tournament.nanoid.clone());
        let mut output = HandlerOutput::from(vec![InternalServerMessage {
            destination: MessageDestination::User(self.invitee),
            message: ServerMessage::Tournament(TournamentUpdate::Invited(response)),
        }]);
        append_public_tournament_patches(
            tournament.id,
            &[PublicTournamentSection::Memberships],
            &mut output,
            &mut conn,
        )
        .await;
        Ok(output.messages)
    }
}
