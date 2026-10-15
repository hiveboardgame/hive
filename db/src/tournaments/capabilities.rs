use crate::{
    db_error::DbError,
    models::{TournamentSlot, TournamentSwissRound},
    tournaments::{
        fixed_field::{adjudicated_outcome, supports_closeout},
        TournamentGame,
    },
};
use hive_lib::{Color, GameStatus};
use shared_types::{
    tournament::{Format, GameOutcome, Resolution, SlotAdminAction, SlotKey},
    TournamentGameResult,
    TournamentStatus,
};
use std::collections::{HashMap, HashSet};
use tournamint::{
    elimination::{
        EliminationNodeFactState,
        EliminationNodeId,
        EliminationNodeResolution,
        EliminationSource,
    },
    series::{self, SeriesGameFact, SeriesGameId, SeriesGameState},
    swiss::{SwissGameId, SwissLeg},
    PlayerId,
};
use uuid::Uuid;

use super::{
    elimination::{elimination_config, series_facts_by_node, EliminationFactsProjection},
    sequential_release_eligible_slots,
    state::{invalid_persisted, TournamentState},
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
    pub(crate) available_results: Vec<TournamentGameResult>,
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
    games_by_slot: HashMap<Uuid, &'a TournamentGame>,
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
    let elimination_series = match facts {
        FormatFacts::Elimination(_) => series_facts_by_node(state),
        _ => HashMap::new(),
    };
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
        let available_results =
            available_results(state, facts, slot, &admin_actions, &elimination_series)?;
        slots.insert(
            slot.id,
            SlotCapabilities {
                admin_actions,
                available_results,
                waits_for,
            },
        );
    }

    let closeout_slot_ids = supports_closeout(format)
        .then(|| closeout_slot_ids(state, format, &index.games_by_slot, &gameless));
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

/// A replacement must leave every already released game meaningful. Replaying
/// each candidate mirrors the correction's reopening of a clinched suffix,
/// while retaining cancellation by withdrawal and all released game identities.
fn available_results(
    state: &TournamentState,
    facts: FormatFacts<'_>,
    slot: &TournamentSlot,
    actions: &HashSet<SlotAdminAction>,
    elimination_series: &HashMap<EliminationNodeId, Vec<SeriesGameFact>>,
) -> Result<Vec<TournamentGameResult>, DbError> {
    if !actions.contains(&SlotAdminAction::RecordResult)
        && !actions.contains(&SlotAdminAction::ReplaceResult)
    {
        return Ok(Vec::new());
    }
    let candidates = vec![
        TournamentGameResult::Winner(Color::White),
        TournamentGameResult::Draw,
        TournamentGameResult::Winner(Color::Black),
        TournamentGameResult::DoubleForfeit,
    ];
    let FormatFacts::Elimination(projected) = facts else {
        return Ok(candidates);
    };
    if !actions.contains(&SlotAdminAction::ReplaceResult) {
        return Ok(candidates);
    }
    let SlotKey::Elimination { node, slot: game } = slot.key else {
        return Err(invalid_persisted(
            "elimination result has a non-elimination slot key",
        ));
    };
    let descriptor = &projected.definition.nodes()[node.value()];
    let fact = &projected.facts[node.value()];
    let entrants = match fact.state {
        EliminationNodeFactState::Active { entrants } => entrants,
        EliminationNodeFactState::Resolved {
            entrants: [Some(first), Some(second)],
            ..
        } => [first, second],
        _ => {
            return Err(invalid_persisted(
                "correctable elimination series has no entrants",
            ))
        }
    };
    let plan = elimination_config(&state.configuration)?
        .effective_plan(descriptor.stage)
        .to_native()
        .map_err(|error| invalid_persisted(&format!("invalid elimination series plan: {error}")))?;
    let original = elimination_series
        .get(&node)
        .ok_or_else(|| invalid_persisted("correctable elimination series has no game facts"))?;
    let corrected_index = original
        .iter()
        .position(|fact| fact.id == game)
        .ok_or_else(|| invalid_persisted("correctable elimination slot has no game fact"))?;
    let project = if matches!(
        fact.state,
        EliminationNodeFactState::Resolved {
            resolution: EliminationNodeResolution::WithdrawalWalkover { .. }
                | EliminationNodeResolution::WithdrawalVacancy { .. }
                | EliminationNodeResolution::MutualWithdrawalVacancy { .. },
            ..
        }
    ) {
        series::project_withdrawal
    } else {
        series::project
    };
    let mut available = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let outcome = adjudicated_outcome(&candidate)?;
        // The mutation accepts an identical outcome without reopening anything.
        if slot.resolution == Some(Resolution::Result(outcome)) {
            available.push(candidate);
            continue;
        }
        let mut corrected = original.clone();
        corrected[corrected_index].state = SeriesGameState::Completed(outcome);
        for fact in &mut corrected {
            if fact.id > game && fact.state == SeriesGameState::Skipped {
                fact.state = SeriesGameState::Planned;
            }
        }
        if project(&plan, entrants, &corrected).is_ok() {
            available.push(candidate);
        }
    }
    Ok(available)
}

fn unresolved_predecessor(
    facts: FormatFacts<'_>,
    slot: &TournamentSlot,
    game: Option<&TournamentGame>,
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

fn correctable_admin_adjudication(game: &TournamentGame) -> bool {
    game.finished
        && game.turn == 0
        && game.game_status == GameStatus::Adjudicated.to_string()
        && matches!(game.conclusion.as_str(), "Committee" | "Forfeit")
}

fn unstarted_game(game: &TournamentGame) -> bool {
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
        FormatFacts::Elimination(projected) => {
            elimination_frontier_allows(projected, slot, clearing, index)
        }
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
    clearing: bool,
    index: &CapabilityIndex<'_>,
) -> bool {
    let SlotKey::Elimination { node, slot: game } = slot.key else {
        return false;
    };
    if projected.definition.nodes().get(node.value()).is_none() {
        return false;
    }
    // A series releases games sequentially. Clearing an earlier result cannot
    // reopen it alongside a later released or resolved game. Clinched slots are
    // safe: the correction path restores them to the planned suffix.
    if clearing && index.slots_by_key.values().any(|candidate| {
        matches!(candidate.key, SlotKey::Elimination { node: candidate_node, slot: candidate_game }
            if candidate_node == node && candidate_game > game)
            && (index.games_by_slot.contains_key(&candidate.id)
                || matches!(
                    candidate.resolution,
                    Some(Resolution::Result(_) | Resolution::Withdrawal(_))
                ))
    }) {
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
    games_by_slot: &HashMap<Uuid, &TournamentGame>,
    gameless: &HashSet<Uuid>,
) -> Vec<Uuid> {
    let mut slot_ids = Vec::new();
    for slot in state.slots.iter().filter(|slot| slot.resolution.is_none()) {
        match games_by_slot.get(&slot.id).copied() {
            Some(game) if unstarted_game(game) => slot_ids.push(slot.id),
            Some(_) => {}
            None if format == Format::RoundRobin && gameless.contains(&slot.id) => {
                slot_ids.push(slot.id)
            }
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
