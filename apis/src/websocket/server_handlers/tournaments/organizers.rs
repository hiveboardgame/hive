use super::{append_public_tournament_patches, PublicTournamentSection};
use crate::{
    common::{ServerMessage, TournamentUpdate},
    notifications::{notify, Event},
    websocket::{HandlerOutput, InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, helpers::run_serializable, models::Tournament, DbPool};
use shared_types::TournamentId;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub enum OrganizerOperation {
    Invite(Uuid),
    Accept,
    Decline,
    Retract(Uuid),
    Leave,
}

pub async fn handle(
    tournament_id: TournamentId,
    actor: Uuid,
    operation: OrganizerOperation,
    pool: &DbPool,
) -> Result<HandlerOutput> {
    let mut conn = get_conn(pool).await?;
    let (tournament, changed, still_member) = run_serializable(&mut conn, move |tc| {
        let tournament_id = tournament_id.clone();
        Box::pin(async move {
            let tournament =
                Tournament::find_by_tournament_id_for_update(&tournament_id, tc).await?;
            let changed = match operation {
                OrganizerOperation::Invite(invitee) => {
                    tournament.invite_organizer(actor, invitee, tc).await?
                }
                OrganizerOperation::Accept => {
                    tournament.accept_organizer_invitation(actor, tc).await?;
                    true
                }
                OrganizerOperation::Decline => {
                    tournament.decline_organizer_invitation(actor, tc).await?;
                    true
                }
                OrganizerOperation::Retract(invitee) => {
                    tournament
                        .retract_organizer_invitation(actor, invitee, tc)
                        .await?;
                    true
                }
                OrganizerOperation::Leave => {
                    tournament.leave_organizers(actor, tc).await?;
                    true
                }
            };
            let still_member = tournament.retains_chat_access(actor, tc).await?;
            Ok((tournament, changed, still_member))
        })
    })
    .await?;
    if !changed {
        return Ok(HandlerOutput::empty());
    }
    let id = TournamentId(tournament.nanoid.clone());
    let (recipient, update) = match operation {
        OrganizerOperation::Invite(invitee) => {
            notify(Event::TournamentOrganizerInvite {
                recipient: invitee,
                tournament_name: tournament.name.clone(),
                tournament_nanoid: tournament.nanoid.clone(),
            });
            (invitee, TournamentUpdate::OrganizerInvited(id.clone()))
        }
        OrganizerOperation::Accept => (actor, TournamentUpdate::OrganizerJoined(id.clone())),
        OrganizerOperation::Decline => (actor, TournamentUpdate::OrganizerUninvited(id.clone())),
        OrganizerOperation::Retract(invitee) => {
            (invitee, TournamentUpdate::OrganizerUninvited(id.clone()))
        }
        OrganizerOperation::Leave => (
            actor,
            TournamentUpdate::OrganizerLeft(id.clone(), still_member),
        ),
    };
    let mut output = HandlerOutput::from(vec![
        InternalServerMessage {
            destination: MessageDestination::User(recipient),
            message: ServerMessage::Tournament(update),
        },
        InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::CatalogChanged(id)),
        },
    ]);
    append_public_tournament_patches(
        tournament.id,
        &[
            PublicTournamentSection::Memberships,
            PublicTournamentSection::Lifecycle,
        ],
        &mut output,
        &mut conn,
    )
    .await;
    Ok(output)
}
