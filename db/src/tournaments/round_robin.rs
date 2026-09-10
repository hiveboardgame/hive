use crate::{
    db_error::DbError,
    models::{Game, TournamentSlot, TournamentSlotInsert, TournamentUser},
    DbConn,
};
use chrono::{DateTime, Utc};
use shared_types::{
    tournament::{
        round_robin::Config as RoundRobinConfig,
        FormatConfig,
        ReleasePolicy,
        Resolution,
        SlotKey,
    },
    Clock,
};
use std::collections::{HashMap, HashSet};
use tournamint::{
    round_robin::{
        project as project_round_robin,
        RoundRobinGameFact,
        RoundRobinProjection,
        RoundRobinSchedule,
    },
    GameOutcome,
    PlayerId,
};
use uuid::Uuid;

use super::{
    build_round_robin_definition,
    configuration::build_round_robin_config,
    finish_fixed_field,
    release_slot,
    seal_slot_with_cleanup_at,
    state::{invalid_input, invalid_input_with, invalid_persisted},
    terminal_game_outcome,
    InitialArtifacts,
    ProgressionEffects,
    SlotSealedEvent,
    TournamentState,
};

mod finished;

pub(crate) use finished::round_robin_finished_snapshot;

pub(crate) struct RoundRobinFactsProjection {
    pub(crate) projection: RoundRobinProjection,
}

impl RoundRobinFactsProjection {
    pub(crate) fn is_complete(&self) -> bool {
        self.projection.is_complete()
    }
}

pub(crate) struct PreparedRoundRobinStart {
    schedule: RoundRobinSchedule,
    clock: Clock,
    release_policy: ReleasePolicy,
}

impl PreparedRoundRobinStart {
    pub(crate) const fn rating_clock(&self) -> Clock {
        self.clock
    }
}

pub(crate) fn prepare_initial_start(
    config: &RoundRobinConfig,
    participant_count: usize,
) -> Result<PreparedRoundRobinStart, DbError> {
    let schedule = build_round_robin_definition(config, participant_count)
        .map_err(|error| invalid_input_with("Invalid Round Robin schedule", error))?;
    Ok(PreparedRoundRobinStart {
        schedule,
        clock: config.clock,
        release_policy: config.release_policy,
    })
}

pub(crate) fn initial_artifacts(
    prepared: PreparedRoundRobinStart,
    memberships: &[TournamentUser],
) -> InitialArtifacts {
    let slots = prepared
        .schedule
        .rounds
        .iter()
        .flat_map(|round| &round.games)
        .map(|game| {
            let white = &memberships[game.white.index()];
            let black = &memberships[game.black.index()];
            TournamentSlotInsert {
                key: SlotKey::RoundRobin { slot: game.id },
                white: white.user_id,
                black: black.user_id,
                clock: prepared.clock,
                resolution: None,
            }
        })
        .collect::<Vec<_>>();
    let release = match prepared.release_policy {
        ReleasePolicy::FullyUnlocked => slots.iter().map(|slot| slot.key).collect(),
        ReleasePolicy::SequentialPerMatchup => prepared
            .schedule
            .rounds
            .iter()
            .flat_map(|round| &round.games)
            .filter(|game| game.previous_repeat.is_none())
            .map(|game| SlotKey::RoundRobin { slot: game.id })
            .collect(),
    };
    InitialArtifacts::new(slots, release)
}

pub(crate) fn project_round_robin_facts(
    state: &TournamentState,
) -> Result<RoundRobinFactsProjection, DbError> {
    let FormatConfig::RoundRobin(config) = &state.configuration.format else {
        return Err(invalid_input(
            "The Round Robin fact adapter received another tournament format",
        ));
    };
    let config = build_round_robin_config(config);
    let players = (0..state.memberships.len())
        .map(PlayerId::new)
        .collect::<Vec<_>>();
    let mut results = Vec::new();
    for slot in &state.slots {
        let SlotKey::RoundRobin { slot: game } = slot.key else {
            continue;
        };
        let Some(outcome) = resolved_game_outcome(slot) else {
            continue;
        };
        results.push(RoundRobinGameFact { id: game, outcome });
    }
    let projection = project_round_robin(&config, &players, &results)
        .map_err(|error| invalid_persisted(&format!("invalid Round Robin standings: {error}")))?;
    Ok(RoundRobinFactsProjection { projection })
}

/// Reports whether a still-Planned sequential repeat has reached its release
/// dependency. Accepted dispositions remain the dependency record even when a
/// later correction reopens their effective result, because Round Robin
/// corrections stay available after later repeats have materialized.
pub(crate) fn sequential_slot_is_release_eligible(
    state: &TournamentState,
    slot_id: Uuid,
) -> Result<bool, DbError> {
    Ok(sequential_release_eligible_slots(state)?.contains(&slot_id))
}

pub(crate) fn sequential_release_eligible_slots(
    state: &TournamentState,
) -> Result<HashSet<Uuid>, DbError> {
    let release_policy = match &state.configuration.format {
        FormatConfig::RoundRobin(config) => config.release_policy,
        _ => {
            return Err(invalid_input(
                "The Round Robin release adapter received another tournament format",
            ))
        }
    };
    if release_policy != ReleasePolicy::SequentialPerMatchup {
        return Ok(HashSet::new());
    }

    let definition = round_robin_definition(state)?;
    let scheduled_games = definition
        .rounds
        .iter()
        .flat_map(|round| &round.games)
        .collect::<Vec<_>>();
    let slots_by_game = state
        .slots
        .iter()
        .filter_map(|slot| match slot.key {
            SlotKey::RoundRobin { slot: game } => Some((game, slot)),
            SlotKey::Swiss { .. } | SlotKey::Elimination { .. } => None,
        })
        .collect::<HashMap<_, _>>();
    let mut eligible = HashSet::new();
    for &scheduled in &scheduled_games {
        let slot = slots_by_game[&scheduled.id];
        let mut scheduled = scheduled;
        let mut ready = true;
        while let Some(previous_repeat) = scheduled.previous_repeat {
            let previous_slot = slots_by_game[&previous_repeat];
            if previous_slot.resolution.is_none() {
                ready = false;
                break;
            }
            scheduled = scheduled_games[previous_repeat.value()];
        }
        if ready {
            eligible.insert(slot.id);
        }
    }
    Ok(eligible)
}

/// Releases a cleared no-game repeat once its predecessor disposition has made
/// it eligible. The common correction primitive deliberately leaves a no-game
/// Clear as Planned; this format adapter owns the subsequent release policy.
pub(crate) async fn release_cleared_sequential_slot_if_eligible(
    state: &mut TournamentState,
    slot_id: Uuid,
    released_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Game>, DbError> {
    let slot = &state.slots[state.slot_index(slot_id)?];
    if slot.resolution.is_some()
        || state.slot_game(slot_id).is_some()
        || !sequential_slot_is_release_eligible(state, slot_id)?
    {
        return Ok(Vec::new());
    }
    Ok(vec![release_slot(state, slot_id, released_at, conn).await?])
}

/// Advances one sequential matchup lane after a newly accepted disposition.
/// A future repeat may already be Sealed without a game (for example,
/// an organizer adjudicated it while an earlier repeat was still active), so
/// walk through terminal repeats and release the first remaining Planned one.
pub(crate) async fn release_sequential_successor_after_disposition(
    state: &mut TournamentState,
    disposed_slot_id: Uuid,
    released_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Game>, DbError> {
    let mut cursor = &state.slots[state.slot_index(disposed_slot_id)?];
    if cursor.resolution.is_none() {
        return Ok(Vec::new());
    }
    if !sequential_slot_is_release_eligible(state, disposed_slot_id)? {
        return Ok(Vec::new());
    }

    let definition = round_robin_definition(state)?;
    let scheduled_games = definition
        .rounds
        .iter()
        .flat_map(|round| &round.games)
        .collect::<Vec<_>>();
    let slots_by_game = state
        .slots
        .iter()
        .filter_map(|slot| match slot.key {
            SlotKey::RoundRobin { slot: game } => Some((game, slot)),
            SlotKey::Swiss { .. } | SlotKey::Elimination { .. } => None,
        })
        .collect::<HashMap<_, _>>();
    let games_by_slot = state.games_by_slot();
    loop {
        let SlotKey::RoundRobin { slot: game } = cursor.key else {
            return Ok(Vec::new());
        };
        let Some(next_repeat) = scheduled_games[game.value()].next_repeat else {
            return Ok(Vec::new());
        };
        let successor = slots_by_game[&next_repeat];
        match (
            successor.resolution,
            games_by_slot.contains_key(&successor.id),
        ) {
            (None, false) => {
                let successor_id = successor.id;
                return Ok(vec![
                    release_slot(state, successor_id, released_at, conn).await?,
                ]);
            }
            (None, true) => return Ok(Vec::new()),
            (Some(_), _) => cursor = successor,
        }
    }
}

pub(crate) async fn progress_round_robin_game(
    state: &mut TournamentState,
    game_id: Uuid,
    sealed_at: DateTime<Utc>,
    progressed_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<ProgressionEffects, DbError> {
    let release_policy = match &state.configuration.format {
        FormatConfig::RoundRobin(config) => config.release_policy,
        _ => {
            return Err(invalid_input(
                "The Round Robin progression adapter received another tournament format",
            ))
        }
    };
    let slot_id = slot_id_for_game(state, game_id)?;
    let slot_index = state.slot_index(slot_id)?;
    let sealing_slot = state.slots[slot_index].clone();
    let sealed =
        seal_round_robin_terminal_for_batch(state, game_id, sealed_at, progressed_at, conn).await?;

    let mut released_games = Vec::new();
    if sealed && release_policy == ReleasePolicy::SequentialPerMatchup {
        released_games = release_sequential_successor_after_disposition(
            state,
            sealing_slot.id,
            progressed_at,
            conn,
        )
        .await?;
    }

    let projection = project_round_robin_facts(state)?;
    let mut finished_now = false;
    if projection.is_complete() {
        finish_fixed_field(state, progressed_at, conn).await?;
        finished_now = true;
    }
    Ok(ProgressionEffects {
        sealed,
        advanced: false,
        released_games,
        finished_now,
    })
}

/// Seals one terminal Round Robin game inside a larger aggregate mutation.
///
/// This deliberately does not release a repeat successor. The caller must
/// finish every related batch mutation and project the current facts before
/// allowing its transaction to commit.
pub(crate) async fn seal_round_robin_terminal_for_batch(
    state: &mut TournamentState,
    game_id: Uuid,
    sealed_at: DateTime<Utc>,
    schedule_cleanup_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<bool, DbError> {
    if !matches!(&state.configuration.format, FormatConfig::RoundRobin(_)) {
        return Err(invalid_input(
            "The Round Robin terminal adapter received another tournament format",
        ));
    }
    let slot_id = slot_id_for_game(state, game_id)?;
    let slot_index = state.slot_index(slot_id)?;
    let sealing_slot = state.slots[slot_index].clone();
    let game = state.game(game_id)?;
    if !game.finished {
        return Ok(false);
    }
    let outcome = terminal_game_outcome(game)?;
    seal_slot_with_cleanup_at(
        state,
        SlotSealedEvent {
            slot_id: sealing_slot.id,
            outcome,
            sealed_at,
        },
        schedule_cleanup_at,
        conn,
    )
    .await
}

fn resolved_game_outcome(slot: &TournamentSlot) -> Option<GameOutcome> {
    match slot.resolution {
        Some(Resolution::Result(outcome) | Resolution::Withdrawal(outcome)) => Some(outcome),
        None | Some(Resolution::Clinched) => None,
    }
}

fn slot_id_for_game(state: &TournamentState, game_id: Uuid) -> Result<Uuid, DbError> {
    let slot_id = state
        .game(game_id)?
        .tournament_slot_id
        .expect("fixed-field games own tournament Slots");
    Ok(slot_id)
}

fn round_robin_definition(state: &TournamentState) -> Result<RoundRobinSchedule, DbError> {
    let FormatConfig::RoundRobin(config) = &state.configuration.format else {
        return Err(invalid_input(
            "The Round Robin topology adapter received another tournament format",
        ));
    };
    build_round_robin_definition(config, state.memberships.len())
        .map_err(|error| invalid_persisted(&format!("invalid Round Robin definition: {error}")))
}
