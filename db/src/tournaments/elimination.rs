use super::{
    apply_slot_resolutions,
    finish_fixed_field,
    materialize_slots,
    release_slot,
    seal_slot_with_cleanup_at,
    state::{invalid_input, invalid_input_with, invalid_persisted},
    terminal_game_outcome,
    InitialArtifacts,
    ProgressionEffects,
    SlotResolutionUpdate,
    SlotSealedEvent,
    TournamentState,
};
use crate::{
    db_error::DbError,
    models::{
        Game,
        TournamentEliminationNode,
        TournamentSlot,
        TournamentSlotInsert,
        TournamentUser,
    },
    DbConn,
};
use chrono::{DateTime, Utc};
use shared_types::{
    tournament::{
        elimination::{Config as EliminationConfig, Resolution, SeriesPlan, Source},
        Config,
        FormatConfig,
        Resolution as SlotResolution,
        SlotKey,
    },
    Clock,
};
use std::collections::HashMap;
use tournamint::{
    elimination::{
        self,
        EliminationDefinition,
        EliminationError,
        EliminationNodeDescriptor,
        EliminationNodeFact,
        EliminationNodeFactState,
        EliminationNodeId as NativeNodeId,
        EliminationNodePatch,
        EliminationNodeResolution,
        EliminationProjection,
        EliminationReleaseCadence,
        EliminationSource,
    },
    series::{self, SeriesAction, SeriesGameFact, SeriesGameId, SeriesGameSpec, SeriesGameState},
    PlayerId,
};
use uuid::Uuid;

mod finished;

pub(crate) use finished::{elimination_finished_snapshot, elimination_player_counts};

#[derive(Clone, Debug)]
pub(crate) struct EliminationFactsProjection {
    pub(crate) definition: EliminationDefinition,
    pub(crate) facts: Vec<EliminationNodeFact>,
    pub(crate) projection: EliminationProjection,
}

impl EliminationFactsProjection {
    pub(crate) fn has_active_node(&self) -> bool {
        self.facts
            .iter()
            .any(|fact| matches!(fact.state, EliminationNodeFactState::Active { .. }))
    }
}

pub(crate) struct PreparedEliminationStart {
    rating_clock: Clock,
    slots: Vec<PreparedEliminationSlot>,
    release: Vec<SlotKey>,
    facts: Vec<EliminationNodeFact>,
}

struct PreparedEliminationSlot {
    key: SlotKey,
    white: PlayerId,
    black: PlayerId,
    clock: Clock,
}

struct PreparedSeriesTemplate {
    opening: PreparedSeriesGame,
    remaining: Vec<PreparedSeriesGame>,
    release: SeriesGameId,
}

#[derive(Clone, Copy)]
struct PreparedSeriesGame {
    id: SeriesGameId,
    white_is_first: bool,
    black_is_first: bool,
    clock: Clock,
}

impl PreparedSeriesTemplate {
    fn new(plan: &SeriesPlan) -> Result<Self, DbError> {
        let first = PlayerId::new(0);
        let second = PlayerId::new(1);
        let (native_plan, clock) = plan.to_native_with_initial_clock().map_err(|error| {
            invalid_persisted(&format!("invalid elimination series plan: {error}"))
        })?;
        let initial = series::initial_set(&native_plan, [first, second]).map_err(|error| {
            invalid_persisted(&format!("invalid elimination series plan: {error}"))
        })?;
        let prepare = |game: SeriesGameSpec| PreparedSeriesGame {
            id: game.id,
            white_is_first: game.white == first,
            black_is_first: game.black == first,
            clock,
        };
        Ok(Self {
            opening: prepare(initial.first),
            remaining: initial.remaining.into_iter().map(prepare).collect(),
            release: initial.release,
        })
    }

    fn slots(
        &self,
        node: NativeNodeId,
        entrants: [PlayerId; 2],
    ) -> (Vec<PreparedEliminationSlot>, SlotKey) {
        let slots = std::iter::once(&self.opening)
            .chain(&self.remaining)
            .map(|game| PreparedEliminationSlot {
                key: SlotKey::Elimination {
                    node,
                    slot: game.id,
                },
                white: if game.white_is_first {
                    entrants[0]
                } else {
                    entrants[1]
                },
                black: if game.black_is_first {
                    entrants[0]
                } else {
                    entrants[1]
                },
                clock: game.clock,
            })
            .collect();
        (
            slots,
            SlotKey::Elimination {
                node,
                slot: self.release,
            },
        )
    }
}

impl PreparedEliminationStart {
    pub(crate) const fn rating_clock(&self) -> Clock {
        self.rating_clock
    }
}

pub(crate) fn prepare_initial_start(
    config: &EliminationConfig,
    participant_count: usize,
    bracket_order: Option<&[usize]>,
) -> Result<PreparedEliminationStart, DbError> {
    let default_template = PreparedSeriesTemplate::new(&config.default_plan)?;
    let override_templates = config
        .stage_overrides
        .iter()
        .map(|stage_override| {
            Ok((
                stage_override.stage,
                PreparedSeriesTemplate::new(&stage_override.plan)?,
            ))
        })
        .collect::<Result<Vec<_>, DbError>>()?;
    let definition = build_definition(config, participant_count, bracket_order, false)?;
    let mut facts = elimination::initial_facts(&definition);
    let initial = elimination::next_release(
        &definition,
        &facts,
        EliminationReleaseCadence::DependencyReady,
        &[],
    )
    .map_err(|error| {
        invalid_input_with(
            "could not materialize the initial elimination matches",
            error,
        )
    })?;
    apply_patches_in_memory(&mut facts, &initial.patches);
    let mut slots = Vec::new();
    let mut release = Vec::new();
    for descriptor in definition.nodes() {
        if let Some(entrants) = initial.patches.iter().find_map(|patch| {
            (patch.node == descriptor.id)
                .then_some(patch.state)
                .and_then(|state| match state {
                    EliminationNodeFactState::Active { entrants } => Some(entrants),
                    _ => None,
                })
        }) {
            let template = override_templates
                .iter()
                .find(|(stage, _)| *stage == descriptor.stage)
                .map_or(&default_template, |(_, template)| template);
            let (created, released) = template.slots(descriptor.id, entrants);
            slots.extend(created);
            release.push(released);
        }
    }
    Ok(PreparedEliminationStart {
        rating_clock: default_template.opening.clock,
        slots,
        release,
        facts,
    })
}

pub(crate) fn initial_artifacts(
    prepared: PreparedEliminationStart,
    memberships: &[TournamentUser],
) -> InitialArtifacts {
    let slots = materialize_prepared_slots(prepared.slots, memberships);
    InitialArtifacts::new(slots, prepared.release).with_elimination_nodes(prepared.facts)
}

fn materialize_prepared_slots(
    slots: Vec<PreparedEliminationSlot>,
    memberships: &[TournamentUser],
) -> Vec<TournamentSlotInsert> {
    slots
        .into_iter()
        .map(|slot| TournamentSlotInsert {
            key: slot.key,
            white: memberships[slot.white.index()].user_id,
            black: memberships[slot.black.index()].user_id,
            clock: slot.clock,
            resolution: None,
        })
        .collect()
}

pub(crate) fn project_elimination_facts(
    state: &TournamentState,
) -> Result<EliminationFactsProjection, DbError> {
    let config = elimination_config(&state.configuration)?;
    let bracket_order = state
        .tournament
        .bracket_order
        .as_ref()
        .map(|value| {
            let users: Vec<Uuid> = serde_json::from_value(value.clone())
                .map_err(|error| invalid_persisted(&format!("invalid bracket order: {error}")))?;
            users
                .into_iter()
                .map(|user_id| {
                    state
                        .memberships
                        .iter()
                        .position(|membership| membership.user_id == user_id)
                        .ok_or_else(|| {
                            invalid_persisted("bracket order contains an unknown entrant")
                        })
                })
                .collect::<Result<Vec<_>, DbError>>()
        })
        .transpose()?;
    let definition = build_definition(
        config,
        state.memberships.len(),
        bracket_order.as_deref(),
        true,
    )?;
    let facts = state
        .elimination_nodes
        .iter()
        .map(TournamentEliminationNode::fact)
        .collect::<Result<Vec<_>, DbError>>()?;
    let withdrawn = state
        .memberships
        .iter()
        .enumerate()
        .filter_map(|(index, membership)| membership.withdrawn_at.map(|_| PlayerId::new(index)))
        .collect::<Vec<_>>();
    let projection = elimination::project(&definition, &facts, &withdrawn)
        .map_err(|error| invalid_persisted(&format!("invalid elimination facts: {error}")))?;
    Ok(EliminationFactsProjection {
        definition,
        facts,
        projection,
    })
}

pub(crate) async fn progress_elimination_game(
    state: &mut TournamentState,
    game_id: Uuid,
    sealed_at: DateTime<Utc>,
    progressed_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<ProgressionEffects, DbError> {
    let slot_id = slot_id_for_game(state, game_id)?;
    let node = slot_coordinates(state, slot_id)?.0;
    let sealed =
        seal_elimination_terminal_for_batch(state, game_id, sealed_at, progressed_at, conn).await?;
    let mut released_games = Vec::new();
    apply_series_actions(state, Some(node), progressed_at, conn, &mut released_games).await?;
    let mut progressed =
        progress_elimination_after_series(state, progressed_at, conn, released_games).await?;
    progressed.sealed = sealed;
    Ok(progressed)
}

pub(crate) async fn progress_elimination(
    state: &mut TournamentState,
    progressed_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<ProgressionEffects, DbError> {
    progress_elimination_for_node(state, None, progressed_at, conn).await
}

pub(crate) async fn progress_elimination_for_node(
    state: &mut TournamentState,
    preferred_node: Option<NativeNodeId>,
    progressed_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<ProgressionEffects, DbError> {
    let mut released_games = Vec::new();
    apply_series_actions(
        state,
        preferred_node,
        progressed_at,
        conn,
        &mut released_games,
    )
    .await?;
    progress_elimination_after_series(state, progressed_at, conn, released_games).await
}

async fn progress_elimination_after_series(
    state: &mut TournamentState,
    progressed_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
    mut released_games: Vec<Game>,
) -> Result<ProgressionEffects, DbError> {
    let mut projected = project_elimination_facts(state)?;
    let mut advanced = false;
    loop {
        if projected.projection.complete {
            finish_fixed_field(state, progressed_at, conn).await?;
            return Ok(ProgressionEffects {
                sealed: false,
                advanced,
                released_games,
                finished_now: true,
            });
        }
        let withdrawn = state
            .memberships
            .iter()
            .enumerate()
            .filter(|(_, member)| member.withdrawn_at.is_some())
            .map(|(index, _)| PlayerId::new(index))
            .collect::<Vec<_>>();
        let ready = match elimination::next_release(
            &projected.definition,
            &projected.facts,
            EliminationReleaseCadence::DependencyReady,
            &withdrawn,
        ) {
            Ok(ready) => ready,
            Err(EliminationError::DependenciesPending) => {
                return Ok(ProgressionEffects {
                    sealed: false,
                    advanced,
                    released_games,
                    finished_now: false,
                })
            }
            Err(error) => {
                return Err(invalid_persisted(&format!(
                    "could not release elimination matches: {error}"
                )))
            }
        };
        persist_node_patches(state, &ready.patches, conn).await?;
        let (slots, release) = slots_for_release(state, &projected.definition, &ready.patches)?;
        released_games.extend(materialize_slots(state, slots, release, progressed_at, conn).await?);
        advanced = true;
        projected = project_elimination_facts(state)?;
    }
}

pub(crate) async fn seal_elimination_terminal_for_batch(
    state: &mut TournamentState,
    game_id: Uuid,
    sealed_at: DateTime<Utc>,
    schedule_cleanup_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<bool, DbError> {
    let game = state.game(game_id)?;
    let slot_id = slot_id_for_game(state, game_id)?;
    let slot = state.slots[state.slot_index(slot_id)?].clone();
    slot_coordinates(state, slot_id)?;
    if !game.finished {
        return Ok(false);
    }
    let outcome = terminal_game_outcome(game)?;
    seal_slot_with_cleanup_at(
        state,
        SlotSealedEvent {
            slot_id: slot.id,
            outcome,
            sealed_at,
        },
        schedule_cleanup_at,
        conn,
    )
    .await
}

async fn apply_series_actions(
    state: &mut TournamentState,
    preferred_node: Option<NativeNodeId>,
    at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
    released_games: &mut Vec<Game>,
) -> Result<(), DbError> {
    let projected = project_elimination_facts(state)?;
    let active = projected
        .projection
        .nodes
        .iter()
        .filter_map(|projection| {
            let EliminationNodeFactState::Active { entrants } = projection.state else {
                return None;
            };
            preferred_node
                .is_none_or(|node| projection.descriptor.id == node)
                .then_some((projection.descriptor, entrants))
        })
        .collect::<Vec<_>>();
    for (descriptor, entrants) in active {
        let config = elimination_config(&state.configuration)?;
        let plan = config.effective_plan(descriptor.stage);
        let native_plan = plan.to_native().map_err(|error| {
            invalid_persisted(&format!("invalid elimination series plan: {error}"))
        })?;
        let facts = series_facts(state, descriptor.id);
        let action = series::next_action(&native_plan, entrants, &facts).map_err(|error| {
            invalid_persisted(&format!("invalid elimination series facts: {error}"))
        })?;
        let Some(action) = action else {
            continue;
        };
        match action {
            SeriesAction::Release { game } => {
                let key = SlotKey::Elimination {
                    node: descriptor.id,
                    slot: game,
                };
                let slots_by_key = state
                    .slots
                    .iter()
                    .map(|slot| (slot.key, slot.id))
                    .collect::<HashMap<_, _>>();
                let slot_id = slots_by_key[&key];
                released_games.push(release_slot(state, slot_id, at, conn).await?);
            }
            SeriesAction::CreateSet { games, release } => {
                let (slots, release_id) =
                    slots_from_specs(plan, &state.memberships, descriptor.id, &games, release);
                released_games
                    .extend(materialize_slots(state, slots, vec![release_id], at, conn).await?);
            }
            SeriesAction::SkipAndDecide { games, winner } => {
                let slots_by_key = state
                    .slots
                    .iter()
                    .map(|slot| (slot.key, slot.id))
                    .collect::<HashMap<_, _>>();
                let transitions = games
                    .into_iter()
                    .map(|game| {
                        let key = SlotKey::Elimination {
                            node: descriptor.id,
                            slot: game,
                        };
                        let slot_id = slots_by_key[&key];
                        SlotResolutionUpdate {
                            slot_id,
                            resolution: SlotResolution::Clinched,
                        }
                    })
                    .collect::<Vec<_>>();
                apply_slot_resolutions(state, &transitions, at, conn).await?;
                persist_winner(state, &projected, descriptor.id, winner, conn).await?;
            }
            SeriesAction::Decide { winner } => {
                persist_winner(state, &projected, descriptor.id, winner, conn).await?;
            }
        }
    }
    Ok(())
}

async fn persist_winner(
    state: &mut TournamentState,
    projected: &EliminationFactsProjection,
    node: NativeNodeId,
    winner: PlayerId,
    conn: &mut DbConn<'_>,
) -> Result<(), DbError> {
    let patch = elimination::record_winner(&projected.definition, &projected.facts, node, winner)
        .expect("validated series action supplies an admissible elimination winner");
    persist_node_patches(state, &[patch], conn).await
}

/// Restores the corrected series to a state which ordinary progression can evaluate.
///
/// The command layer has already proved that the target is a mutable unplayed
/// adjudication and that no dependent node materialized. Reopening the node here is
/// essential: `series::next_action` must decide whether the corrected facts
/// release another game, decide immediately, or mark the suffix clinched again.
pub(crate) async fn reopen_elimination_after_result_correction(
    state: &mut TournamentState,
    slot_id: Uuid,
    corrected_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<NativeNodeId, DbError> {
    let node =
        reopen_corrected_elimination_series_suffix(state, slot_id, corrected_at, conn).await?;
    let projected = project_elimination_facts(state)?;
    let projected_node = &projected.projection.nodes[node.value()];
    let entrants = match projected_node.state {
        EliminationNodeFactState::Active { .. } => return Ok(node),
        EliminationNodeFactState::Resolved {
            entrants: [Some(first), Some(second)],
            resolution: EliminationNodeResolution::MatchDecided { .. },
        } => [first, second],
        EliminationNodeFactState::Planned
        | EliminationNodeFactState::Resolved { .. }
        | EliminationNodeFactState::Skipped => return Ok(node),
    };
    let patch = EliminationNodePatch {
        node,
        state: EliminationNodeFactState::Active { entrants },
    };
    persist_node_patches(state, &[patch], conn).await?;
    Ok(node)
}

async fn reopen_corrected_elimination_series_suffix(
    state: &mut TournamentState,
    slot_id: Uuid,
    corrected_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<NativeNodeId, DbError> {
    let (node, corrected_ordinal) = slot_coordinates(state, slot_id)?;
    let suffix = state
        .slots
        .iter()
        .enumerate()
        .filter_map(|(index, slot)| {
            let SlotKey::Elimination {
                node: candidate,
                slot: game,
            } = slot.key
            else {
                return None;
            };
            (candidate == node
                && game > corrected_ordinal
                && slot.resolution == Some(SlotResolution::Clinched))
            .then_some(index)
        })
        .collect::<Vec<_>>();
    for index in suffix {
        let reopened_slot_id = state.slots[index].id;
        state.slots[index].resolution = None;
        state.schedule_offer_updates.extend(
            TournamentSlot::persist_resolution(
                state.tournament.id,
                &state.slots[index],
                corrected_at,
                conn,
            )
            .await?,
        );
        state.record_slot_resolution_change(reopened_slot_id, true, false);
    }
    Ok(node)
}

pub(crate) async fn record_elimination_withdrawal(
    state: &mut TournamentState,
    player: PlayerId,
    conn: &mut DbConn<'_>,
) -> Result<(), DbError> {
    let projected = project_elimination_facts(state)?;
    let patches = elimination::withdraw(&projected.definition, &projected.facts, player)
        .map_err(|error| invalid_input_with("elimination withdrawal was rejected", error))?;
    persist_node_patches(state, &patches, conn).await
}

fn slots_for_release(
    state: &TournamentState,
    definition: &EliminationDefinition,
    patches: &[EliminationNodePatch],
) -> Result<(Vec<TournamentSlotInsert>, Vec<SlotKey>), DbError> {
    let config = elimination_config(&state.configuration)?;
    let mut slots = Vec::new();
    let mut release = Vec::new();
    for descriptor in definition.nodes() {
        let entrants = patches.iter().find_map(|patch| match patch.state {
            EliminationNodeFactState::Active { entrants } if patch.node == descriptor.id => {
                Some(entrants)
            }
            _ => None,
        });
        if let Some(entrants) = entrants {
            let (created, released) =
                initial_series_slots(config, descriptor, &state.memberships, entrants)?;
            slots.extend(created);
            release.push(released);
        }
    }
    Ok((slots, release))
}

fn initial_series_slots(
    config: &EliminationConfig,
    descriptor: &EliminationNodeDescriptor,
    memberships: &[TournamentUser],
    entrants: [PlayerId; 2],
) -> Result<(Vec<TournamentSlotInsert>, SlotKey), DbError> {
    let stage = descriptor.stage;
    let plan = config.effective_plan(stage);
    let template = PreparedSeriesTemplate::new(plan)?;
    let (slots, release) = template.slots(descriptor.id, entrants);
    Ok((materialize_prepared_slots(slots, memberships), release))
}

fn slots_from_specs(
    plan: &SeriesPlan,
    memberships: &[TournamentUser],
    node: NativeNodeId,
    games: &[SeriesGameSpec],
    release: SeriesGameId,
) -> (Vec<TournamentSlotInsert>, SlotKey) {
    let slots = games
        .iter()
        .map(|game| {
            let phase = &plan.phases[game.phase_index];
            let white = &memberships[game.white.index()];
            let black = &memberships[game.black.index()];
            TournamentSlotInsert {
                key: SlotKey::Elimination {
                    node,
                    slot: game.id,
                },
                white: white.user_id,
                black: black.user_id,
                clock: phase.clock,
                resolution: None,
            }
        })
        .collect::<Vec<_>>();
    (
        slots,
        SlotKey::Elimination {
            node,
            slot: release,
        },
    )
}

pub(crate) fn series_facts(state: &TournamentState, node: NativeNodeId) -> Vec<SeriesGameFact> {
    let slots = state.slots.iter().filter(move |slot| {
        matches!(slot.key, SlotKey::Elimination { node: slot_node, .. } if slot_node == node)
    });
    let mut facts = series_facts_from_slots(state, slots)
        .map(|(_, fact)| fact)
        .collect::<Vec<_>>();
    facts.sort_unstable_by_key(|fact| fact.id);
    facts
}

pub(crate) fn series_facts_by_node(
    state: &TournamentState,
) -> HashMap<NativeNodeId, Vec<SeriesGameFact>> {
    let mut by_node = HashMap::<NativeNodeId, Vec<SeriesGameFact>>::new();
    for (node, fact) in series_facts_from_slots(state, state.slots.iter()) {
        by_node.entry(node).or_default().push(fact);
    }
    for facts in by_node.values_mut() {
        facts.sort_unstable_by_key(|fact| fact.id);
    }
    by_node
}

fn series_facts_from_slots<'a>(
    state: &'a TournamentState,
    slots: impl Iterator<Item = &'a TournamentSlot> + 'a,
) -> impl Iterator<Item = (NativeNodeId, SeriesGameFact)> + 'a {
    let games_by_slot = state.games_by_slot();
    let players = state
        .memberships
        .iter()
        .enumerate()
        .map(|(index, membership)| (membership.user_id, PlayerId::new(index)))
        .collect::<HashMap<_, _>>();
    slots.filter_map(move |slot| {
        let SlotKey::Elimination { node, slot: game } = slot.key else {
            return None;
        };
        let white = players[&slot.white];
        let state = match slot.resolution {
            Some(SlotResolution::Result(outcome) | SlotResolution::Withdrawal(outcome)) => {
                SeriesGameState::Completed(outcome)
            }
            Some(SlotResolution::Clinched) => SeriesGameState::Skipped,
            None if games_by_slot.contains_key(&slot.id) => SeriesGameState::Released,
            None => SeriesGameState::Planned,
        };
        Some((
            node,
            SeriesGameFact {
                id: game,
                white,
                state,
            },
        ))
    })
}

async fn persist_node_patches(
    state: &mut TournamentState,
    patches: &[EliminationNodePatch],
    conn: &mut DbConn<'_>,
) -> Result<(), DbError> {
    for patch in patches {
        let fact = EliminationNodeFact {
            node: patch.node,
            state: patch.state,
        };
        let row = TournamentEliminationNode::upsert(state.tournament.id, &fact, conn).await?;
        if let Some(position) = state
            .elimination_nodes
            .iter()
            .position(|current| current.node_id == row.node_id)
        {
            state.elimination_nodes[position] = row;
        } else {
            state.elimination_nodes.push(row);
        }
    }
    Ok(())
}

fn apply_patches_in_memory(facts: &mut [EliminationNodeFact], patches: &[EliminationNodePatch]) {
    for fact in facts {
        if let Some(patch) = patches.iter().find(|patch| patch.node == fact.node) {
            fact.state = patch.state;
        }
    }
}

pub(crate) fn elimination_config(configuration: &Config) -> Result<&EliminationConfig, DbError> {
    let FormatConfig::Elimination(config) = &configuration.format else {
        return Err(invalid_input("elimination adapter received another format"));
    };
    Ok(config)
}

fn build_definition(
    config: &EliminationConfig,
    participant_count: usize,
    bracket_order: Option<&[usize]>,
    persisted: bool,
) -> Result<EliminationDefinition, DbError> {
    let indices =
        bracket_order.map_or_else(|| (0..participant_count).collect(), |order| order.to_vec());
    let mut sorted = indices.clone();
    sorted.sort_unstable();
    if !sorted.into_iter().eq(0..participant_count) {
        return Err(if persisted {
            invalid_persisted("bracket order must contain every entrant exactly once")
        } else {
            invalid_input("bracket order must contain every entrant exactly once")
        });
    }
    let seeds = indices.into_iter().map(PlayerId::new).collect::<Vec<_>>();
    elimination::topology(config.topology, &seeds).map_err(|error| {
        if persisted {
            invalid_persisted(&format!("invalid elimination definition: {error}"))
        } else {
            invalid_input_with("invalid elimination definition", error)
        }
    })
}

pub(crate) fn public_resolution(
    state: &TournamentState,
    resolution: EliminationNodeResolution,
) -> Resolution {
    match resolution {
        EliminationNodeResolution::MatchDecided { winner, loser } => Resolution::PlayedWinner {
            winner: required_user(state, winner),
            loser: required_user(state, loser),
        },
        EliminationNodeResolution::WithdrawalWalkover { winner, withdrawn } => {
            Resolution::Walkover {
                winner: required_user(state, winner),
                withdrawn: required_user(state, withdrawn),
            }
        }
        EliminationNodeResolution::AutomaticAdvance { player } => Resolution::AutomaticAdvance {
            player: required_user(state, player),
        },
        EliminationNodeResolution::WithdrawalVacancy { .. }
        | EliminationNodeResolution::MutualWithdrawalVacancy { .. }
        | EliminationNodeResolution::Vacant => Resolution::Vacancy,
    }
}

pub(crate) fn public_source(source: EliminationSource) -> Source {
    match source {
        EliminationSource::InitialSeed { seed_index } => Source::InitialSeed {
            seed_index: u32::try_from(seed_index)
                .expect("admitted elimination seed index fits u32"),
        },
        EliminationSource::WinnerOf(node) => Source::WinnerOf { node_id: node },
        EliminationSource::LoserOf(node) => Source::LoserOf { node_id: node },
        EliminationSource::Vacant => Source::Vacant,
    }
}

fn slot_coordinates(
    state: &TournamentState,
    slot_id: Uuid,
) -> Result<(NativeNodeId, SeriesGameId), DbError> {
    let slot = &state.slots[state.slot_index(slot_id)?];
    let SlotKey::Elimination { node, slot: game } = slot.key else {
        return Err(invalid_input(
            "the Slot is not in an elimination tournament",
        ));
    };
    Ok((node, game))
}

fn slot_id_for_game(state: &TournamentState, game_id: Uuid) -> Result<Uuid, DbError> {
    let slot_id = state
        .game(game_id)?
        .tournament_slot_id
        .expect("elimination games own tournament Slots");
    Ok(slot_id)
}

fn required_user(state: &TournamentState, player: PlayerId) -> Uuid {
    state.memberships[player.index()].user_id
}

#[cfg(test)]
mod tests {
    use shared_types::tournament_view::{EliminationNodeStateResponse, TournamentFormatResponse};

    use super::{progress_elimination, seal_elimination_terminal_for_batch};
    use crate::{
        db_error::DbError,
        get_conn,
        models::{Game, Tournament, TournamentEliminationNode, TournamentSlot, TournamentUser},
        schema::tournaments::dsl as tournament_columns,
        test_support::{
            db::test_db,
            tournament::{
                create_rr_tournament_with_configuration,
                create_user,
                fixed_instant,
                insert_unfrozen_memberships,
            },
        },
        tournaments::{
            fixed_field as fixed_field_db,
            public::load_by_id,
            state::{load_in_progress, BotUsers},
        },
        DbConn,
    };
    use chrono::Utc;
    use diesel::ExpressionMethods;
    use diesel_async::RunQueryDsl;
    use hive_lib::Color;
    use shared_types::{
        tournament::{
            elimination::{
                ClinchPolicy,
                Config as EliminationConfig,
                EntrantSide,
                SeriesPhase,
                SeriesPlan,
                SetLimit,
                Stage,
                Topology,
            },
            BotAdmission,
            Clock,
            Config,
            EliminationNodeId,
            FormatConfig,
            GameOutcome,
            RealtimeClock,
            Resolution as SlotResolution,
            SeriesGameId,
            SlotKey,
        },
        Conclusion,
        TournamentGameResult,
        TournamentStatus,
    };
    use std::num::NonZeroU32;
    use tournamint::elimination::EliminationNodeFactState;
    use uuid::Uuid;

    mod fixed_field {
        use super::*;

        pub async fn start_by_organizer(
            tournament_id: Uuid,
            organizer_id: Uuid,
            conn: &mut DbConn<'_>,
        ) -> Result<fixed_field_db::StartOutcome, DbError> {
            let expected = TournamentUser::find_by_tournament_id(tournament_id, conn)
                .await?
                .into_iter()
                .map(|membership| membership.user_id)
                .collect();
            let tournament = fixed_field_db::prepare_elimination_start(
                tournament_id,
                organizer_id,
                expected,
                conn,
            )
            .await?;
            let setup = tournament.start_setup().expect("setup prepared");
            fixed_field_db::confirm_elimination_start(
                tournament_id,
                organizer_id,
                setup.id,
                setup.seeded_players,
                conn,
            )
            .await
        }

        pub use fixed_field_db::withdraw_player;

        pub async fn adjudicate_slot_atomic(
            tournament_id: Uuid,
            slot_id: Uuid,
            result: TournamentGameResult,
            actor_id: Uuid,
            conn: &mut DbConn<'_>,
        ) -> Result<fixed_field_db::AdjudicationOutcome, DbError> {
            let expected = TournamentSlot::find(tournament_id, slot_id, conn)
                .await?
                .resolution;
            fixed_field_db::adjudicate_slot_atomic(
                tournament_id,
                slot_id,
                result,
                expected,
                actor_id,
                conn,
            )
            .await
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pending_earlier_branch_does_not_block_series_progression_or_suffix_correction() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get test connection");
        let organizer = create_user("cc_organizer", &mut conn).await;
        let mut players = Vec::new();
        for index in 0..4 {
            players.push(
                create_user(&format!("cc_player_{index}"), &mut conn)
                    .await
                    .id,
            );
        }
        let tournament = create_rr_tournament_with_configuration(
            organizer.id,
            "correction_context",
            TournamentStatus::NotStarted,
            None,
            Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::Elimination(EliminationConfig {
                    topology: Topology::Single { bronze: false },
                    default_plan: SeriesPlan {
                        phases: vec![SeriesPhase {
                            games_per_set: 3,
                            color_order: vec![
                                EntrantSide::First,
                                EntrantSide::Second,
                                EntrantSide::First,
                            ],
                            set_limit: SetLimit::UntilDecisive,
                            clinch: ClinchPolicy::EarlyClinch,
                            clock: realtime(300, 3),
                        }],
                    },
                    stage_overrides: Vec::new(),
                }),
            },
            &mut conn,
        )
        .await;
        insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
        let opening_games = fixed_field::start_by_organizer(tournament.id, organizer.id, &mut conn)
            .await
            .expect("start bracket")
            .games;
        let mut later = None;
        for game in opening_games {
            let slot = slot_for_game(tournament.id, game.id, &mut conn).await;
            let node = elimination_coordinates(&slot).0.value();
            if later.as_ref().is_none_or(|(current, _)| node > *current) {
                later = Some((node, game));
            }
        }
        let later = later.expect("later node game").1;
        let first_slot = slot_for_game(tournament.id, later.id, &mut conn).await;
        let (series_node, first_game_ordinal) = elimination_coordinates(&first_slot);
        assert_eq!(first_game_ordinal, SeriesGameId::new(0));
        let snapshot = load_by_id(tournament.id, &mut conn)
            .await
            .expect("project the planned elimination series suffix");
        let TournamentFormatResponse::Elimination {
            nodes: projection_nodes,
            ..
        } = snapshot.format
        else {
            panic!("single-elimination snapshot has an elimination projection");
        };
        let series = projection_nodes
            .iter()
            .find(|node| node.node_id == series_node)
            .and_then(|node| node.series.as_ref())
            .expect("project active series");
        let first = series.sets[0]
            .slots
            .iter()
            .find(|slot| matches!(slot.slot.key, SlotKey::Elimination { slot, .. } if slot == SeriesGameId::new(0)))
            .expect("project released opening game");
        let second_planned = series.sets[0]
            .slots
            .iter()
            .find(|slot| matches!(slot.slot.key, SlotKey::Elimination { slot, .. } if slot == SeriesGameId::new(1)))
            .expect("project planned second game");
        assert_eq!(first.slot.id, first_slot.id);
        assert!(first.slot.game.is_some());
        assert!(second_planned.slot.game.is_none());
        assert_eq!(second_planned.slot.waits_for, Some(first_slot.id));

        let winner = later.white_id;
        // Withdrawal normalization seals terminal games before progressing without a preferred node.
        let at = Utc::now();
        later
            .adjudicate_unstarted(
                &TournamentGameResult::Winner(Color::White),
                Conclusion::Committee,
                at,
                &mut conn,
            )
            .await
            .expect("persist the later branch's terminal game");
        let current = Tournament::find(tournament.id, &mut conn)
            .await
            .expect("reload tournament");
        let mut state = load_in_progress(current, BotUsers::ForGameRelease, &mut conn)
            .await
            .expect("load current branches");
        seal_elimination_terminal_for_batch(&mut state, later.id, at, at, &mut conn)
            .await
            .expect("seal terminal game");
        let second = progress_elimination(&mut state, at, &mut conn)
            .await
            .expect("progress the ready series past the unrelated pending branch")
            .released_games
            .into_iter()
            .next()
            .expect("second game");
        let released_snapshot = load_by_id(tournament.id, &mut conn)
            .await
            .expect("reproject the released elimination series suffix");
        let TournamentFormatResponse::Elimination {
            nodes: projection_nodes,
            ..
        } = released_snapshot.format
        else {
            panic!("single-elimination snapshot has an elimination projection");
        };
        let released_second = projection_nodes
            .iter()
            .find(|node| node.node_id == series_node)
            .and_then(|node| node.series.as_ref())
            .and_then(|series| {
                series.sets[0]
                    .slots
                    .iter()
                    .find(|slot| matches!(slot.slot.key, SlotKey::Elimination { slot, .. } if slot == SeriesGameId::new(1)))
            })
            .expect("project released second series game");
        assert!(released_second.slot.game.is_some());
        assert!(released_second.slot.waits_for.is_none());

        adjudicate(tournament.id, second.id, winner, organizer.id, &mut conn).await;
        let second_slot = slot_for_game(tournament.id, second.id, &mut conn).await;
        let (corrected_node, _) = elimination_coordinates(&second_slot);
        let clinched = TournamentSlot::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .expect("load clinched series")
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(
            series_slots(&clinched, corrected_node)[2].1.resolution,
            Some(SlotResolution::Clinched),
        );
        let corrected = fixed_field::adjudicate_slot_atomic(
            tournament.id,
            second_slot.id,
            TournamentGameResult::Draw,
            organizer.id,
            &mut conn,
        )
        .await
        .expect("correct later node");
        let effects = corrected
            .commit
            .expect("correction changed tournament state");
        assert_eq!(effects.released_game_ids.len(), 1);
        let released = Game::find_by_game_id(&effects.released_game_ids[0], &mut conn)
            .await
            .expect("load released game");
        let released_slot = slot_for_game(tournament.id, released.id, &mut conn).await;
        let (released_node, ordinal) = elimination_coordinates(&released_slot);
        assert_eq!(released_node, corrected_node);
        assert_eq!(ordinal, SeriesGameId::new(2));
        let reopened = TournamentEliminationNode::find_by_tournament_id(tournament.id, &mut conn)
            .await
            .expect("load reopened node")
            .into_iter()
            .find_map(|node| {
                let fact = node.fact().ok()?;
                (fact.node == corrected_node).then_some(fact)
            })
            .expect("reopened node is persisted");
        assert!(matches!(
            reopened.state,
            EliminationNodeFactState::Active { .. }
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn correction_is_rejected_after_a_dependent_match_materializes() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get test connection");
        let organizer = create_user("wf_organizer", &mut conn).await;
        let mut players = Vec::new();
        for index in 0..4 {
            players.push(
                create_user(&format!("wf_player_{index}"), &mut conn)
                    .await
                    .id,
            );
        }
        let tournament = create_rr_tournament_with_configuration(
            organizer.id,
            "dependency_frontier_correction",
            TournamentStatus::NotStarted,
            None,
            Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::Elimination(EliminationConfig {
                    topology: Topology::Single { bronze: false },
                    default_plan: one_game(realtime(300, 3), EntrantSide::First),
                    stage_overrides: Vec::new(),
                }),
            },
            &mut conn,
        )
        .await;
        insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
        let opening_games = fixed_field::start_by_organizer(tournament.id, organizer.id, &mut conn)
            .await
            .expect("start bracket")
            .games;
        assert_eq!(opening_games.len(), 2);
        let target = opening_games[0].clone();
        let target_slot = slot_for_game(tournament.id, target.id, &mut conn).await;
        let mut semifinal_games = Vec::new();
        for game in opening_games {
            semifinal_games.extend(
                adjudicate(
                    tournament.id,
                    game.id,
                    game.white_id,
                    organizer.id,
                    &mut conn,
                )
                .await,
            );
        }
        let original_resolution = slot_for_game(tournament.id, target.id, &mut conn)
            .await
            .resolution;
        assert!(matches!(
            original_resolution,
            Some(SlotResolution::Result(GameOutcome::Adjudicated(_)))
        ));
        assert_eq!(semifinal_games.len(), 1);

        let unauthorized_retry = fixed_field::adjudicate_slot_atomic(
            tournament.id,
            target_slot.id,
            TournamentGameResult::Winner(Color::White),
            target.white_id,
            &mut conn,
        )
        .await;
        assert!(matches!(unauthorized_retry, Err(DbError::Unauthorized)));
        let exact_retry = fixed_field::adjudicate_slot_atomic(
            tournament.id,
            target_slot.id,
            TournamentGameResult::Winner(Color::White),
            organizer.id,
            &mut conn,
        )
        .await
        .expect("retry the committed result beyond the correction frontier");
        assert!(!exact_retry.cleared);
        assert!(exact_retry.committed_game.is_none());
        assert!(exact_retry.commit.is_none());

        let correction = fixed_field::adjudicate_slot_atomic(
            tournament.id,
            target_slot.id,
            TournamentGameResult::Winner(Color::Black),
            organizer.id,
            &mut conn,
        )
        .await;
        assert!(
            matches!(
                correction,
                Err(DbError::InvalidAction { ref info }) if info.contains("no longer be corrected")
            ),
            "unexpected correction result: {correction:?}",
        );
        assert_eq!(
            slot_for_game(tournament.id, target.id, &mut conn)
                .await
                .resolution,
            original_resolution,
        );
        assert_eq!(
            Game::find_by_uuid(&target.id, &mut conn)
                .await
                .expect("reload unchanged game")
                .tournament_game_result,
            TournamentGameResult::Winner(Color::White).to_string(),
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ready_branches_advance_without_blocking_unrelated_corrections() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get test connection");
        let organizer = create_user("branch_organizer", &mut conn).await;
        let mut players = Vec::new();
        for index in 0..8 {
            players.push(
                create_user(&format!("branch_player_{index}"), &mut conn)
                    .await
                    .id,
            );
        }
        let tournament = create_rr_tournament_with_configuration(
            organizer.id,
            "independent_branches",
            TournamentStatus::NotStarted,
            None,
            Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::Elimination(EliminationConfig {
                    topology: Topology::Single { bronze: false },
                    default_plan: one_game(realtime(300, 3), EntrantSide::First),
                    stage_overrides: Vec::new(),
                }),
            },
            &mut conn,
        )
        .await;
        diesel::update(tournament_columns::tournaments)
            .filter(tournament_columns::id.eq(tournament.id))
            .set(tournament_columns::seats.eq(Some(8)))
            .execute(&mut conn)
            .await
            .expect("set eight-player field");
        insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
        let opening = fixed_field::start_by_organizer(tournament.id, organizer.id, &mut conn)
            .await
            .expect("start bracket")
            .games;
        let mut ordered = Vec::new();
        for game in opening {
            let slot = slot_for_game(tournament.id, game.id, &mut conn).await;
            ordered.push((elimination_coordinates(&slot).0, game));
        }
        ordered.sort_by_key(|(node, _)| *node);
        assert_eq!(ordered.len(), 4);
        assert!(adjudicate(
            tournament.id,
            ordered[0].1.id,
            ordered[0].1.white_id,
            organizer.id,
            &mut conn
        )
        .await
        .is_empty());
        let semifinal = adjudicate(
            tournament.id,
            ordered[1].1.id,
            ordered[1].1.white_id,
            organizer.id,
            &mut conn,
        )
        .await;
        assert_eq!(
            semifinal.len(),
            1,
            "the ready semifinal releases while the other quarterfinals are active"
        );
        let unrelated = &ordered[2].1;
        assert!(adjudicate(
            tournament.id,
            unrelated.id,
            unrelated.white_id,
            organizer.id,
            &mut conn
        )
        .await
        .is_empty());
        let slot = slot_for_game(tournament.id, unrelated.id, &mut conn).await;
        let correction = fixed_field::adjudicate_slot_atomic(
            tournament.id,
            slot.id,
            TournamentGameResult::Winner(Color::Black),
            organizer.id,
            &mut conn,
        )
        .await
        .expect("an unrelated semifinal does not freeze this result");
        assert!(correction.commit.is_some());
        let semifinal_slot = slot_for_game(tournament.id, semifinal[0].id, &mut conn).await;
        assert_ne!(elimination_coordinates(&semifinal_slot).0, ordered[2].0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn withdrawal_only_final_projects_without_a_series() {
        let db = test_db().await;
        let mut conn = get_conn(&db.pool).await.expect("get test connection");
        let organizer = create_user("wp_organizer", &mut conn).await;
        let mut players = Vec::new();
        for index in 0..4 {
            players.push(
                create_user(&format!("wp_player_{index}"), &mut conn)
                    .await
                    .id,
            );
        }
        let tournament = create_rr_tournament_with_configuration(
            organizer.id,
            "withdrawal_projection",
            TournamentStatus::NotStarted,
            None,
            Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::Elimination(EliminationConfig {
                    topology: Topology::Single { bronze: false },
                    default_plan: one_game(realtime(300, 3), EntrantSide::First),
                    stage_overrides: Vec::new(),
                }),
            },
            &mut conn,
        )
        .await;
        insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), &mut conn).await;
        let opening_games = fixed_field::start_by_organizer(tournament.id, organizer.id, &mut conn)
            .await
            .expect("start bracket")
            .games;
        assert_eq!(opening_games.len(), 2);

        let withdrawn_finalist = opening_games[0].white_id;
        adjudicate(
            tournament.id,
            opening_games[0].id,
            withdrawn_finalist,
            organizer.id,
            &mut conn,
        )
        .await;
        fixed_field::withdraw_player(
            tournament.id,
            withdrawn_finalist,
            withdrawn_finalist,
            &mut conn,
        )
        .await
        .expect("withdraw finalist before its match materializes");
        assert!(adjudicate(
            tournament.id,
            opening_games[1].id,
            opening_games[1].white_id,
            organizer.id,
            &mut conn,
        )
        .await
        .is_empty());

        let snapshot = load_by_id(tournament.id, &mut conn)
            .await
            .expect("load withdrawal-only final through the public projection");
        let TournamentFormatResponse::Elimination {
            nodes: projection_nodes,
            ..
        } = snapshot.format
        else {
            panic!("single-elimination snapshot has an elimination projection");
        };
        let final_node = projection_nodes
            .iter()
            .find(|node| node.stage == Stage::SingleFinal)
            .expect("project championship node");
        assert!(matches!(
            final_node.state,
            EliminationNodeStateResponse::Resolved(_)
        ));
        assert!(final_node.entrants.iter().all(Option::is_some));
        assert!(final_node.series.is_none());
    }

    fn realtime(base_seconds: u32, increment_seconds: u32) -> Clock {
        Clock::Realtime(RealtimeClock {
            base_seconds: NonZeroU32::new(base_seconds).unwrap(),
            increment_seconds,
        })
    }

    fn one_game(clock: Clock, white: EntrantSide) -> SeriesPlan {
        SeriesPlan {
            phases: vec![SeriesPhase {
                games_per_set: 1,
                color_order: vec![white],
                set_limit: SetLimit::UntilDecisive,
                clinch: ClinchPolicy::PlayAll,
                clock,
            }],
        }
    }

    fn series_slots(
        slots: &[TournamentSlot],
        node: EliminationNodeId,
    ) -> Vec<(SeriesGameId, &TournamentSlot)> {
        let mut found = slots
            .iter()
            .filter_map(|slot| {
                let SlotKey::Elimination {
                    node: slot_node,
                    slot: game,
                } = slot.key
                else {
                    return None;
                };
                (slot_node == node).then_some((game, slot))
            })
            .collect::<Vec<_>>();
        found.sort_unstable_by_key(|(ordinal, _)| *ordinal);
        found
    }

    fn elimination_coordinates(slot: &TournamentSlot) -> (EliminationNodeId, SeriesGameId) {
        let SlotKey::Elimination { node, slot: game } = slot.key else {
            panic!("elimination Slot has its native key");
        };
        (node, game)
    }

    async fn slot_for_game(
        tournament_id: Uuid,
        game_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> TournamentSlot {
        let game = Game::find_by_uuid(&game_id, conn).await.expect("load game");
        let slot_id = game
            .tournament_slot_id
            .expect("fixed-field game Slot binding");
        TournamentSlot::find(tournament_id, slot_id, conn)
            .await
            .expect("load slot")
    }

    async fn adjudicate(
        tournament_id: Uuid,
        game_id: Uuid,
        winner: Uuid,
        organizer_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Vec<Game> {
        let slot = slot_for_game(tournament_id, game_id, conn).await;
        let result = if slot.white == winner {
            TournamentGameResult::Winner(Color::White)
        } else {
            TournamentGameResult::Winner(Color::Black)
        };
        let outcome =
            fixed_field::adjudicate_slot_atomic(tournament_id, slot.id, result, organizer_id, conn)
                .await
                .expect("adjudicate slot");
        let released_game_ids = outcome
            .commit
            .map(|effects| effects.released_game_ids)
            .unwrap_or_default();
        Game::find_by_nanoids(&released_game_ids, conn)
            .await
            .expect("load released games")
    }
}
