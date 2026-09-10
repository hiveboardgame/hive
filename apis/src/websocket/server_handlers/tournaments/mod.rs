use shared_types::tournament_view::TournamentFormatResponse;
pub mod adjudicate_result;
pub mod arena;
pub mod berserk;
pub mod closeout;
pub mod create;
pub mod delete;
pub mod handler;
pub mod invitation_accept;
pub mod invitation_create;
pub mod invitation_decline;
pub mod invitation_retract;
pub mod join;
pub mod kick;
pub mod leave;
pub mod organizers;
pub mod start;
pub mod withdraw;

pub(crate) use start::{setup_messages, start_outcome_messages};

use crate::{
    common::{ServerMessage, TournamentUpdate},
    responses::{
        TournamentLifecycleDetails,
        TournamentMemberships,
        TournamentPatch,
        TournamentResponse,
        TournamentStandings,
    },
    websocket::messages::{
        HandlerOutput,
        InternalServerMessage,
        MessageDestination,
        TournamentAudience,
    },
};
use db_lib::{models::Tournament, tournaments::public::load_by_id, DbConn};
use shared_types::TournamentId;
use std::collections::{BTreeSet, HashSet};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub(crate) enum PublicTournamentSection {
    Availability,
    Format,
    Lifecycle,
    Memberships,
    Standings,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum PlayerTournamentLifecycle {
    Started,
    Finished,
}

pub(crate) fn player_lifecycle_messages(
    tournament_id: TournamentId,
    player_ids: impl IntoIterator<Item = Uuid>,
    lifecycle: PlayerTournamentLifecycle,
) -> Vec<InternalServerMessage> {
    let mut messages = player_ids
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|player_id| {
            let update = match lifecycle {
                PlayerTournamentLifecycle::Started => {
                    TournamentUpdate::Started(tournament_id.clone())
                }
                PlayerTournamentLifecycle::Finished => {
                    TournamentUpdate::Finished(tournament_id.clone())
                }
            };
            InternalServerMessage {
                destination: MessageDestination::User(player_id),
                message: ServerMessage::Tournament(update),
            }
        })
        .collect::<Vec<_>>();
    if matches!(lifecycle, PlayerTournamentLifecycle::Finished) {
        messages.push(InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::OrganizerInvitationsClosed(
                tournament_id,
            )),
        });
    }
    messages
}

pub(crate) async fn load_player_ids_after_commit(
    tournament: &Tournament,
    lifecycle: PlayerTournamentLifecycle,
    conn: &mut DbConn<'_>,
) -> Vec<Uuid> {
    match tournament.players(conn).await {
        Ok(players) => players.into_iter().map(|player| player.id).collect(),
        Err(error) => {
            log::error!(
                "Tournament {} committed its {lifecycle:?} lifecycle transition but participant messages could not be built: {error}",
                tournament.nanoid,
            );
            Vec::new()
        }
    }
}

pub(crate) async fn load_public_tournament_response(
    tournament_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Option<TournamentResponse> {
    let projected = async {
        let snapshot = load_by_id(tournament_id, conn).await?;
        Ok::<_, anyhow::Error>(*TournamentResponse::from_snapshot(snapshot)?)
    }
    .await;
    match projected {
        Ok(response) => Some(response),
        Err(error) => {
            log::error!(
                "Tournament {tournament_id} committed but its public projection could not be built: {error}"
            );
            None
        }
    }
}

pub(crate) fn public_tournament_patch_message(
    tournament_id: TournamentId,
    patch: TournamentPatch,
) -> InternalServerMessage {
    InternalServerMessage {
        destination: MessageDestination::Tournament {
            tournament_id: tournament_id.clone(),
            audience: TournamentAudience::Updates,
        },
        message: ServerMessage::Tournament(TournamentUpdate::Patch {
            tournament_id,
            patch: Box::new(patch),
        }),
    }
}

pub(crate) fn append_public_tournament_patches_from_response(
    response: &TournamentResponse,
    sections: &[PublicTournamentSection],
    output: &mut HandlerOutput,
) {
    output
        .messages
        .extend(sections.iter().filter_map(|section| {
            let patch = match section {
                PublicTournamentSection::Availability => match &response.format {
                    TournamentFormatResponse::Arena { .. } => return None,
                    TournamentFormatResponse::RoundRobin {
                        withdrawable_entrants,
                        closeout_eligible_slots,
                        ..
                    } => TournamentPatch::RoundRobinAvailabilityReplace {
                        withdrawable_entrants: withdrawable_entrants.clone(),
                        closeout_eligible_slots: *closeout_eligible_slots,
                    },
                    TournamentFormatResponse::Swiss {
                        withdrawable_entrants,
                        closeout_eligible_slots,
                        ..
                    } => TournamentPatch::SwissAvailabilityReplace {
                        withdrawable_entrants: withdrawable_entrants.clone(),
                        closeout_eligible_slots: *closeout_eligible_slots,
                    },
                    TournamentFormatResponse::Elimination {
                        withdrawable_entrants,
                        ..
                    } => TournamentPatch::EliminationAvailabilityReplace {
                        withdrawable_entrants: withdrawable_entrants.clone(),
                    },
                },
                PublicTournamentSection::Format => {
                    TournamentPatch::FormatReplace(response.format.clone())
                }
                PublicTournamentSection::Lifecycle => TournamentPatch::LifecycleDetailsReplace(
                    TournamentLifecycleDetails::from_response(response),
                ),
                PublicTournamentSection::Memberships => TournamentPatch::MembershipsReplace(
                    TournamentMemberships::from_response(response),
                ),
                PublicTournamentSection::Standings => {
                    TournamentPatch::StandingsReplace(TournamentStandings::from_response(response))
                }
            };
            Some(public_tournament_patch_message(
                response.tournament_id.clone(),
                patch,
            ))
        }));
}

pub(crate) async fn append_public_tournament_patches(
    tournament_id: Uuid,
    sections: &[PublicTournamentSection],
    output: &mut HandlerOutput,
    conn: &mut DbConn<'_>,
) {
    let Some(response) = load_public_tournament_response(tournament_id, conn).await else {
        return;
    };
    append_public_tournament_patches_from_response(&response, sections, output);
}

pub(crate) async fn append_public_tournament_patches_by_id(
    tournament_id: &TournamentId,
    sections: &[PublicTournamentSection],
    output: &mut HandlerOutput,
    conn: &mut DbConn<'_>,
) {
    let tournament = match Tournament::find_by_tournament_id(tournament_id, conn).await {
        Ok(tournament) => tournament,
        Err(error) => {
            log::error!(
                "Tournament {tournament_id} committed but could not be loaded for public patches: {error}"
            );
            return;
        }
    };
    append_public_tournament_patches(tournament.id, sections, output, conn).await;
}

pub(crate) async fn append_public_slot_patches(
    tournament_id: Uuid,
    slot_ids: &[Uuid],
    output: &mut HandlerOutput,
    conn: &mut DbConn<'_>,
) {
    if slot_ids.is_empty() {
        return;
    }
    let Some(response) = load_public_tournament_response(tournament_id, conn).await else {
        return;
    };
    append_public_slot_patches_from_response(&response, slot_ids, output);
}

pub(crate) fn append_public_slot_patches_from_response(
    response: &TournamentResponse,
    slot_ids: &[Uuid],
    output: &mut HandlerOutput,
) {
    let requested = slot_ids.iter().copied().collect::<HashSet<_>>();
    let slots = response
        .fixed_slots()
        .into_iter()
        .filter(|slot| requested.contains(&slot.id))
        .collect::<Vec<_>>();
    if slots.len() != requested.len() {
        log::error!(
            "Tournament {} committed but only {}/{} requested public Slots were projected",
            response.tournament_id,
            slots.len(),
            requested.len(),
        );
    }
    output.messages.extend(slots.into_iter().map(|slot| {
        public_tournament_patch_message(
            response.tournament_id.clone(),
            TournamentPatch::SlotUpsert(slot.clone()),
        )
    }));
}

async fn membership_removed_messages(
    tournament: &Tournament,
    user_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Vec<InternalServerMessage> {
    let tournament_id = TournamentId(tournament.nanoid.clone());
    let mut output = HandlerOutput::from(vec![InternalServerMessage {
        destination: MessageDestination::Global,
        message: ServerMessage::Tournament(TournamentUpdate::CatalogChanged(tournament_id.clone())),
    }]);
    if matches!(
        tournament.retains_chat_access(user_id, conn).await,
        Ok(false)
    ) {
        output.messages.push(InternalServerMessage {
            destination: MessageDestination::User(user_id),
            message: ServerMessage::Tournament(TournamentUpdate::Left(tournament_id)),
        });
    }
    append_public_tournament_patches(
        tournament.id,
        &[PublicTournamentSection::Memberships],
        &mut output,
        conn,
    )
    .await;
    output.messages
}

#[cfg(test)]
mod tests {
    use super::{player_lifecycle_messages, PlayerTournamentLifecycle};
    use crate::{
        common::{ServerMessage, TournamentUpdate},
        websocket::messages::{InternalServerMessage, MessageDestination},
    };
    use shared_types::TournamentId;
    use uuid::Uuid;

    #[test]
    fn lifecycle_notifications_target_each_affected_player_once() {
        let tournament_id = TournamentId(String::from("cup"));
        let first = Uuid::from_u128(1);
        let second = Uuid::from_u128(2);
        let messages = player_lifecycle_messages(
            tournament_id.clone(),
            [second, first, second],
            PlayerTournamentLifecycle::Finished,
        );

        assert_eq!(messages.len(), 3);
        assert!(matches!(
            &messages[2],
            InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Tournament(TournamentUpdate::OrganizerInvitationsClosed(id)),
            } if id == &tournament_id
        ));
        assert!(matches!(
            &messages[..2],
            [
                InternalServerMessage {
                    destination: MessageDestination::User(first_destination),
                    message: ServerMessage::Tournament(TournamentUpdate::Finished(first_id)),
                },
                InternalServerMessage {
                    destination: MessageDestination::User(second_destination),
                    message: ServerMessage::Tournament(TournamentUpdate::Finished(second_id)),
                },
            ] if *first_destination == first
                && *second_destination == second
                && first_id == &tournament_id
                && second_id == &tournament_id
        ));
    }
}
