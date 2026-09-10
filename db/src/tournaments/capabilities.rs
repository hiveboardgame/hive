use crate::{
    db_error::DbError,
    models::{Game, TournamentSlot, TournamentSwissRound},
    tournaments::fixed_field::supports_closeout,
};
use hive_lib::GameStatus;
use shared_types::{
    tournament::{Format, GameOutcome, Resolution, SlotAdminAction, SlotKey},
    TournamentStatus,
};
use std::collections::{HashMap, HashSet};
use tournamint::{
    elimination::{EliminationNodeFactState, EliminationSource},
    series::SeriesGameId,
    swiss::{SwissGameId, SwissLeg},
    PlayerId,
};
use uuid::Uuid;

use super::{
    elimination::EliminationFactsProjection,
    sequential_release_eligible_slots,
    state::TournamentState,
    RoundRobinFactsProjection,
};

#[derive(Clone, Copy)]
pub(crate) enum FormatFacts<'a> {
    RoundRobin(&'a RoundRobinFactsProjection),
    Swiss,
    Elimination(&'a EliminationFactsProjection),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SlotCapabilities {
    pub(crate) admin_actions: HashSet<SlotAdminAction>,
    pub(crate) waits_for: Option<Uuid>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CapabilityProjection {
    pub(crate) slots: HashMap<Uuid, SlotCapabilities>,
    pub(crate) withdrawable_entrants: HashSet<Uuid>,
    pub(crate) closeout_eligible_slots: Option<u32>,
    pub(crate) closeout_slot_ids: Option<Vec<Uuid>>,
}

struct CapabilityIndex<'a> {
    games_by_slot: HashMap<Uuid, &'a Game>,
    slots_by_key: HashMap<SlotKey, &'a TournamentSlot>,
    round_robin_predecessors: Vec<Option<SlotKey>>,
}

/// Evaluates objective action availability from one coherent aggregate snapshot.
/// Mutation handlers authorize their actor separately and re-evaluate these facts
/// after acquiring locks.
pub(crate) fn evaluate_capabilities(
    state: &TournamentState,
    facts: FormatFacts<'_>,
) -> Result<CapabilityProjection, DbError> {
    let format = state.configuration.format();
    let in_progress = state.tournament.status() == TournamentStatus::InProgress;
    if !in_progress {
        return Ok(CapabilityProjection {
            slots: state
                .slots
                .iter()
                .map(|slot| (slot.id, SlotCapabilities::default()))
                .collect(),
            ..CapabilityProjection::default()
        });
    }
    let index = CapabilityIndex {
        games_by_slot: state.games_by_slot(),
        slots_by_key: state.slots.iter().map(|slot| (slot.key, slot)).collect(),
        round_robin_predecessors: match facts {
            FormatFacts::RoundRobin(projected) => projected
                .projection
                .rounds
                .iter()
                .flat_map(|round| &round.games)
                .map(|game| {
                    game.scheduled_game
                        .previous_repeat
                        .map(|slot| SlotKey::RoundRobin { slot })
                })
                .collect(),
            FormatFacts::Swiss | FormatFacts::Elimination(_) => Vec::new(),
        },
    };
    let gameless = gameless_adjudication_slots(state, &index)?;
    let withdrawn = state
        .memberships
        .iter()
        .filter(|membership| membership.withdrawn_at.is_some())
        .map(|membership| membership.user_id)
        .collect::<HashSet<_>>();
    let mut slots = HashMap::with_capacity(state.slots.len());
    for slot in &state.slots {
        let game = index.games_by_slot.get(&slot.id).copied();
        let waits_for = unresolved_predecessor(facts, slot, game, &index);
        let admin_actions = admin_actions(
            state,
            facts,
            slot,
            &index,
            gameless.contains(&slot.id),
            &withdrawn,
        );
        slots.insert(
            slot.id,
            SlotCapabilities {
                admin_actions,
                waits_for,
            },
        );
    }

    let closeout_slot_ids =
        supports_closeout(format).then(|| closeout_slot_ids(state, format, &index.games_by_slot));
    let closeout_eligible_slots = closeout_slot_ids
        .as_ref()
        .map(Vec::len)
        .map(|count| u32::try_from(count).expect("creation-bounded Slot count fits u32"));
    Ok(CapabilityProjection {
        slots,
        withdrawable_entrants: withdrawable_entrants(state, facts),
        closeout_eligible_slots,
        closeout_slot_ids,
    })
}

fn unresolved_predecessor(
    facts: FormatFacts<'_>,
    slot: &TournamentSlot,
    game: Option<&Game>,
    index: &CapabilityIndex<'_>,
) -> Option<Uuid> {
    if slot.resolution.is_some() || game.is_some() {
        return None;
    }
    let predecessor_key = match facts {
        FormatFacts::RoundRobin(_) => {
            let SlotKey::RoundRobin { slot: game } = slot.key else {
                return None;
            };
            index
                .round_robin_predecessors
                .get(game.value())
                .copied()
                .flatten()?
        }
        FormatFacts::Swiss => match slot.key {
            SlotKey::Swiss { slot: game } if game.leg == SwissLeg::Second => SlotKey::Swiss {
                slot: SwissGameId {
                    leg: SwissLeg::First,
                    ..game
                },
            },
            _ => return None,
        },
        FormatFacts::Elimination(_) => {
            let SlotKey::Elimination { node, slot: game } = slot.key else {
                return None;
            };
            let predecessor = game.value().checked_sub(1).map(SeriesGameId::new)?;
            SlotKey::Elimination {
                node,
                slot: predecessor,
            }
        }
    };
    let predecessor = index.slots_by_key.get(&predecessor_key)?;
    if predecessor.resolution.is_some() {
        return None;
    }
    Some(predecessor.id)
}

fn admin_actions(
    state: &TournamentState,
    facts: FormatFacts<'_>,
    slot: &TournamentSlot,
    index: &CapabilityIndex<'_>,
    gameless_record_allowed: bool,
    withdrawn: &HashSet<Uuid>,
) -> HashSet<SlotAdminAction> {
    let game = index.games_by_slot.get(&slot.id).copied();
    let mut actions = HashSet::new();
    if slot.resolution.is_none() {
        actions.insert(SlotAdminAction::SetDeadline);
    }
    if slot.resolution.is_none()
        && (game.is_some_and(unstarted_game) || (game.is_none() && gameless_record_allowed))
    {
        actions.insert(SlotAdminAction::RecordResult);
        return actions;
    }
    let game_is_correctable = game.map(correctable_admin_adjudication).unwrap_or(true);
    if !matches!(
        slot.resolution,
        Some(Resolution::Result(GameOutcome::Adjudicated(_)))
    ) || !game_is_correctable
        || !correction_frontier_allows(state, facts, slot, false, index)
    {
        return actions;
    }

    actions.insert(SlotAdminAction::ReplaceResult);
    if !withdrawn.contains(&slot.white)
        && !withdrawn.contains(&slot.black)
        && correction_frontier_allows(state, facts, slot, true, index)
    {
        actions.insert(SlotAdminAction::ClearResult);
    }
    actions
}

fn correctable_admin_adjudication(game: &Game) -> bool {
    game.finished
        && game.turn == 0
        && game.game_status == GameStatus::Adjudicated.to_string()
        && matches!(game.conclusion.as_str(), "Committee" | "Forfeit")
}

fn unstarted_game(game: &Game) -> bool {
    !game.finished && game.turn == 0 && game.game_status == GameStatus::NotStarted.to_string()
}

fn correction_frontier_allows(
    state: &TournamentState,
    facts: FormatFacts<'_>,
    slot: &TournamentSlot,
    clearing: bool,
    index: &CapabilityIndex<'_>,
) -> bool {
    match facts {
        FormatFacts::RoundRobin(_) => true,
        FormatFacts::Swiss => swiss_frontier_allows(state, slot, clearing, index),
        FormatFacts::Elimination(projected) => elimination_frontier_allows(projected, slot),
    }
}

fn swiss_frontier_allows(
    state: &TournamentState,
    slot: &TournamentSlot,
    clearing: bool,
    index: &CapabilityIndex<'_>,
) -> bool {
    let SlotKey::Swiss { slot: game } = slot.key else {
        return false;
    };
    if state
        .swiss_rounds
        .last()
        .map(TournamentSwissRound::native_round_id)
        != Some(game.round_index)
    {
        return false;
    }
    if !(clearing && state.configuration.format() == Format::DoubleSwiss) {
        return true;
    }
    if game.leg != SwissLeg::First {
        return true;
    }
    let second_key = SlotKey::Swiss {
        slot: SwissGameId {
            leg: SwissLeg::Second,
            ..game
        },
    };
    let Some(second) = index.slots_by_key.get(&second_key).copied() else {
        return false;
    };
    second.resolution.is_none() && !index.games_by_slot.contains_key(&second.id)
}

fn elimination_frontier_allows(
    projected: &EliminationFactsProjection,
    slot: &TournamentSlot,
) -> bool {
    let SlotKey::Elimination { node, .. } = slot.key else {
        return false;
    };
    if projected.definition.nodes().get(node.value()).is_none() {
        return false;
    }
    // A materialized descendant necessarily has a materialized direct predecessor.
    // Checking the direct consumers also covers automatic advances through byes.
    !projected.definition.nodes().iter().any(|descriptor| {
        descriptor.sources.iter().any(|source| matches!(source,
            EliminationSource::WinnerOf(parent) | EliminationSource::LoserOf(parent) if *parent == node
        )) && !matches!(projected.facts[descriptor.id.value()].state, EliminationNodeFactState::Planned)
    })
}

fn gameless_adjudication_slots(
    state: &TournamentState,
    index: &CapabilityIndex<'_>,
) -> Result<HashSet<Uuid>, DbError> {
    let round_robin_eligible = if state.configuration.format() == Format::RoundRobin {
        sequential_release_eligible_slots(state)?
    } else {
        HashSet::new()
    };
    let mut slots = HashSet::new();
    for slot in &state.slots {
        if slot.resolution.is_some() || index.games_by_slot.contains_key(&slot.id) {
            continue;
        }
        let eligible = match slot.key {
            SlotKey::RoundRobin { .. } => round_robin_eligible.contains(&slot.id),
            SlotKey::Swiss { slot: game } if game.leg == SwissLeg::Second => {
                let first_key = SlotKey::Swiss {
                    slot: SwissGameId {
                        leg: SwissLeg::First,
                        ..game
                    },
                };
                index
                    .slots_by_key
                    .get(&first_key)
                    .is_some_and(|slot| slot.resolution.is_some())
            }
            SlotKey::Swiss { .. } | SlotKey::Elimination { .. } => false,
        };
        if eligible {
            slots.insert(slot.id);
        }
    }
    Ok(slots)
}

fn closeout_slot_ids(
    state: &TournamentState,
    format: Format,
    games_by_slot: &HashMap<Uuid, &Game>,
) -> Vec<Uuid> {
    let mut slot_ids = Vec::new();
    for slot in state.slots.iter().filter(|slot| slot.resolution.is_none()) {
        match games_by_slot.get(&slot.id).copied() {
            Some(game) if unstarted_game(game) => slot_ids.push(slot.id),
            Some(_) => {}
            None if format == Format::RoundRobin => slot_ids.push(slot.id),
            None => {}
        }
    }
    slot_ids.sort_unstable();
    slot_ids
}

fn withdrawable_entrants(state: &TournamentState, facts: FormatFacts<'_>) -> HashSet<Uuid> {
    if state.tournament.status() != TournamentStatus::InProgress {
        return HashSet::new();
    }
    let active = state
        .memberships
        .iter()
        .enumerate()
        .filter(|(_, membership)| membership.withdrawn_at.is_none())
        .map(|(index, membership)| (PlayerId::new(index), membership.user_id))
        .collect::<HashMap<_, _>>();
    let FormatFacts::Elimination(projected) = facts else {
        return active.into_values().collect();
    };
    let mut possible = HashSet::new();
    for node in &projected.projection.nodes {
        match node.state {
            EliminationNodeFactState::Planned => {
                possible.extend(node.possible_entrants.iter().flatten().copied());
            }
            EliminationNodeFactState::Active { entrants } => {
                possible.extend(entrants);
            }
            EliminationNodeFactState::Resolved { .. } | EliminationNodeFactState::Skipped => {}
        }
    }
    possible
        .into_iter()
        .filter_map(|player| active.get(&player).copied())
        .collect()
}
