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
        TournamentStandings,
    },
    websocket::messages::{
        HandlerOutput,
        InternalServerMessage,
        MessageDestination,
        TournamentAudience,
    },
};
use db_lib::{
    models::Tournament,
    tournaments::public::{
        load_sections,
        TournamentLoadRequest,
        TournamentProjectionRequest,
        TournamentSnapshot,
    },
    DbConn,
};
use shared_types::TournamentId;
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq)]
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

pub(crate) async fn load_public_tournament_snapshot(
    tournament_id: Uuid,
    request: TournamentLoadRequest,
    conn: &mut DbConn<'_>,
) -> Option<TournamentSnapshot> {
    match load_sections(tournament_id, request, conn).await {
        Ok(snapshot) => Some(snapshot),
        Err(error) => {
            log::error!("Tournament {tournament_id} committed but its public projection could not be built: {error}");
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

pub(crate) fn append_public_tournament_patches_from_snapshot(
    snapshot: TournamentSnapshot,
    sections: &[PublicTournamentSection],
    output: &mut HandlerOutput,
) {
    let tournament_id = TournamentId(snapshot.tournament.nanoid.clone());
    for section in sections {
        let missing = match section {
            PublicTournamentSection::Lifecycle => false,
            PublicTournamentSection::Memberships => snapshot.memberships.is_none(),
            PublicTournamentSection::Availability
            | PublicTournamentSection::Format
            | PublicTournamentSection::Standings => snapshot.projection.is_none(),
        };
        if missing {
            log::error!(
                "Tournament {tournament_id} requested a {section:?} patch but its section was not loaded"
            );
        }
    }
    let mut memberships = snapshot.memberships;
    let (mut format, mut standings) = match snapshot.projection {
        Some(projection) => (
            Some(projection.format),
            Some(TournamentStandings {
                player_stats: projection.player_stats,
                snapshot: projection.standings,
            }),
        ),
        None => (None, None),
    };
    for section in sections {
        let patch = match section {
            PublicTournamentSection::Availability
                if sections.contains(&PublicTournamentSection::Format) =>
            {
                // Format replacement already carries availability, regardless of section order.
                continue;
            }
            PublicTournamentSection::Availability => match format.as_ref() {
                Some(TournamentFormatResponse::RoundRobin(state)) => {
                    TournamentPatch::RoundRobinAvailabilityReplace {
                        withdrawable_entrants: state.withdrawable_entrants.clone(),
                        closeout_eligible_slots: state.closeout_eligible_slots,
                    }
                }
                Some(TournamentFormatResponse::Swiss(state)) => {
                    TournamentPatch::SwissAvailabilityReplace {
                        withdrawable_entrants: state.withdrawable_entrants.clone(),
                        closeout_eligible_slots: state.closeout_eligible_slots,
                    }
                }
                Some(TournamentFormatResponse::Elimination(state)) => {
                    TournamentPatch::EliminationAvailabilityReplace {
                        withdrawable_entrants: state.withdrawable_entrants.clone(),
                    }
                }
                _ => continue,
            },
            PublicTournamentSection::Format => {
                let Some(format) = format.take() else {
                    continue;
                };
                TournamentPatch::FormatReplace(format)
            }
            PublicTournamentSection::Lifecycle => TournamentPatch::LifecycleDetailsReplace(
                TournamentLifecycleDetails::from_tournament(&snapshot.tournament),
            ),
            PublicTournamentSection::Memberships => {
                let Some(memberships) = memberships.take() else {
                    continue;
                };
                match TournamentMemberships::from_snapshot(memberships) {
                    Ok(memberships) => TournamentPatch::MembershipsReplace(memberships),
                    Err(error) => {
                        log::error!("Tournament {tournament_id} membership patch could not be built: {error}");
                        continue;
                    }
                }
            }
            PublicTournamentSection::Standings => {
                let Some(standings) = standings.take() else {
                    continue;
                };
                TournamentPatch::StandingsReplace(standings)
            }
        };
        output.messages.push(public_tournament_patch_message(
            tournament_id.clone(),
            patch,
        ));
    }
}

pub(crate) async fn append_public_tournament_patches(
    tournament_id: Uuid,
    sections: &[PublicTournamentSection],
    output: &mut HandlerOutput,
    conn: &mut DbConn<'_>,
) {
    let memberships = sections.contains(&PublicTournamentSection::Memberships);
    let projection = sections
        .iter()
        .any(|section| {
            matches!(
                section,
                PublicTournamentSection::Availability
                    | PublicTournamentSection::Format
                    | PublicTournamentSection::Standings
            )
        })
        .then(|| {
            if sections.contains(&PublicTournamentSection::Format) {
                TournamentProjectionRequest::All
            } else {
                TournamentProjectionRequest::Slots(Vec::new())
            }
        });
    if let Some(snapshot) = load_public_tournament_snapshot(
        tournament_id,
        TournamentLoadRequest {
            memberships,
            projection,
        },
        conn,
    )
    .await
    {
        append_public_tournament_patches_from_snapshot(snapshot, sections, output);
    }
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
    let Some(snapshot) = load_public_tournament_snapshot(
        tournament_id,
        TournamentLoadRequest {
            memberships: false,
            projection: Some(TournamentProjectionRequest::Slots(slot_ids.to_vec())),
        },
        conn,
    )
    .await
    else {
        return;
    };
    append_public_slot_patches_from_snapshot(&snapshot, slot_ids, output);
}

pub(crate) fn append_public_slot_patches_from_snapshot(
    snapshot: &TournamentSnapshot,
    slot_ids: &[Uuid],
    output: &mut HandlerOutput,
) {
    let Some(slots) = snapshot
        .projection
        .as_ref()
        .and_then(|projection| projection.format.slots())
    else {
        log::error!(
            "Tournament {} requested Slot patches but its fixed-field projection was not loaded",
            snapshot.tournament.nanoid
        );
        return;
    };
    let tournament_id = TournamentId(snapshot.tournament.nanoid.clone());
    for slot_id in slot_ids.iter().copied().collect::<BTreeSet<_>>() {
        match slots.get(&slot_id) {
            Some(slot) => output.messages.push(public_tournament_patch_message(
                tournament_id.clone(),
                TournamentPatch::SlotUpsert(slot.clone()),
            )),
            None => log::error!(
                "Tournament {tournament_id} committed but Slot {slot_id} was not projected"
            ),
        }
    }
}

async fn membership_removed_messages(
    tournament: &Tournament,
    user_id: Uuid,
    retained_chat_access: bool,
    conn: &mut DbConn<'_>,
) -> HandlerOutput {
    let tournament_id = TournamentId(tournament.nanoid.clone());
    let mut output = HandlerOutput::from(vec![InternalServerMessage {
        destination: MessageDestination::User(user_id),
        message: ServerMessage::Tournament(TournamentUpdate::Left {
            tournament_id: tournament_id.clone(),
            retained_chat_access,
        }),
    }]);
    if !retained_chat_access {
        output
            .tournament_chat_revocations
            .push((user_id, tournament_id));
    }
    append_public_tournament_patches(
        tournament.id,
        &[PublicTournamentSection::Memberships],
        &mut output,
        conn,
    )
    .await;
    output
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
