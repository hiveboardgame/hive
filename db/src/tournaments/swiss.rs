use crate::{
    db_error::DbError,
    models::{Game, Rating, TournamentSlot, TournamentSlotInsert, TournamentSwissRound},
    tournaments::rating,
    DbConn,
};
use chrono::{DateTime, Utc};
use shared_types::{
    tournament::{
        swiss::System as SwissSystem,
        FormatConfig,
        GameOutcome,
        ReleasePolicy,
        Resolution,
        SlotKey,
        SwissGameId,
        SwissLeg,
    },
    GameSpeed,
    SwissProgress,
};
use std::collections::HashMap;
use tournamint::{
    swiss::{
        self,
        RoundPairings,
        SwissEncounterFact,
        SwissFacts,
        SwissHistoryStatus,
        SwissNextRound,
        SwissNextRoundInput,
        SwissProjection,
        SwissRoundFact,
        SwissSystemConfig as NativeSwissSystemConfig,
    },
    PlayerId,
};
use uuid::Uuid;

use super::{
    configuration::build_swiss_config,
    finish_fixed_field,
    materialize_slots,
    persist_finished_with_outcome,
    release_slot,
    seal_slot_with_cleanup_at,
    state::{invalid_input, invalid_input_with, invalid_persisted},
    terminal_game_outcome,
    ProgressionEffects,
    SlotSealedEvent,
    TournamentState,
};

mod finished;

pub(crate) use finished::swiss_finished_snapshot;

pub(crate) struct SwissFactsProjection {
    pub(crate) facts: SwissFacts,
    pub(crate) projection: SwissProjection,
}

impl SwissFactsProjection {
    pub(crate) fn round_complete(&self) -> bool {
        self.projection.history_status != SwissHistoryStatus::AwaitingResults
    }

    pub(crate) fn tournament_complete(&self) -> bool {
        self.projection.history_status == SwissHistoryStatus::ConfiguredRoundsComplete
    }
}

pub(crate) fn project_swiss_facts(
    state: &TournamentState,
) -> Result<SwissFactsProjection, DbError> {
    let FormatConfig::Swiss(config) = &state.configuration.format else {
        return Err(invalid_input("The Swiss adapter received another format"));
    };
    let config = build_swiss_config(config);
    let slots_by_key = state
        .slots
        .iter()
        .map(|slot| (slot.key, slot))
        .collect::<HashMap<_, _>>();
    let mut rounds = Vec::with_capacity(state.swiss_rounds.len());
    for row in &state.swiss_rounds {
        let pairings = row.pairings();
        let round_id = row.native_round_id();
        let encounters = pairings
            .games
            .iter()
            .enumerate()
            .map(|(pairing_ordinal, pairing)| {
                let mut outcomes = Vec::new();
                let pairing_ordinal = u32::try_from(pairing_ordinal)
                    .expect("admitted Swiss pairing ordinal fits u32");
                let legs = if matches!(config.system, NativeSwissSystemConfig::DoubleSwiss(_)) {
                    &[SwissLeg::First, SwissLeg::Second][..]
                } else {
                    &[SwissLeg::Single][..]
                };
                for &leg in legs {
                    let key = swiss_key(round_id, pairing_ordinal, leg);
                    let slot = slots_by_key[&key];
                    outcomes.push(resolved_game_outcome(slot));
                }
                Ok(SwissEncounterFact {
                    pairing: *pairing,
                    outcomes,
                })
            })
            .collect::<Result<Vec<_>, DbError>>()?;
        rounds.push(SwissRoundFact {
            eligible: eligible_players(pairings),
            pairings: pairings.clone(),
            encounters,
        });
    }
    let facts = SwissFacts {
        config,
        initial_ranking: (0..state.memberships.len()).map(PlayerId::new).collect(),
        rounds,
    };
    let projection = swiss::project(&facts)
        .map_err(|error| invalid_persisted(&format!("invalid Swiss facts: {error}")))?;
    Ok(SwissFactsProjection { facts, projection })
}

pub(crate) async fn progress_swiss_game(
    state: &mut TournamentState,
    game_id: Uuid,
    sealed_at: DateTime<Utc>,
    progressed_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<ProgressionEffects, DbError> {
    let slot_id = slot_id_for_game(state, game_id)?;
    let sealed =
        seal_swiss_terminal_for_batch(state, game_id, sealed_at, progressed_at, conn).await?;
    let mut released_games = Vec::new();
    if sealed {
        released_games
            .extend(release_double_second_if_ready(state, slot_id, progressed_at, conn).await?);
    }
    let mut effects = ProgressionEffects {
        sealed,
        released_games,
        ..ProgressionEffects::default()
    };
    loop {
        let mut progressed = progress_swiss(state, progressed_at, conn).await?;
        let stable = progressed.finished_now
            || !progressed.advanced
            || !progressed.released_games.is_empty();
        effects.advanced |= progressed.advanced;
        effects.finished_now |= progressed.finished_now;
        effects
            .released_games
            .append(&mut progressed.released_games);
        if stable {
            return Ok(effects);
        }
    }
}

pub(crate) async fn progress_swiss(
    state: &mut TournamentState,
    progressed_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<ProgressionEffects, DbError> {
    let projection = project_swiss_facts(state)?;
    if !projection.round_complete() {
        return Ok(ProgressionEffects::default());
    }
    if projection.tournament_complete() {
        finish_fixed_field(state, progressed_at, conn).await?;
        return Ok(ProgressionEffects {
            finished_now: true,
            ..ProgressionEffects::default()
        });
    }
    let input = next_round_input(state);
    let pairings = match swiss::pair_next_round(&projection.facts, &input) {
        Ok(SwissNextRound::Pairings(pairings)) => pairings,
        Ok(SwissNextRound::PairingExhausted) => {
            let standings = swiss_finished_snapshot(state, &projection);
            persist_finished_with_outcome(&state.tournament, standings, progressed_at, conn)
                .await?;
            return Ok(ProgressionEffects {
                finished_now: true,
                ..ProgressionEffects::default()
            });
        }
        Err(error) => {
            return Err(invalid_input_with(
                "Swiss could not pair the next round",
                error,
            ))
        }
    };
    let round_id =
        u32::try_from(projection.facts.rounds.len()).expect("admitted Swiss round count fits u32");
    let rating_snapshots = accepted_rating_snapshots(state, &pairings, conn).await?;
    let (slots, release) = round_slots(state, round_id, &pairings)?;
    let row = TournamentSwissRound::insert(
        state.tournament.id,
        round_id,
        pairings,
        rating_snapshots,
        progressed_at,
        conn,
    )
    .await?;
    state.swiss_rounds.push(row);
    let released_games = materialize_slots(state, slots, release, progressed_at, conn).await?;
    Ok(ProgressionEffects {
        sealed: false,
        advanced: true,
        released_games,
        finished_now: false,
    })
}

async fn accepted_rating_snapshots(
    state: &TournamentState,
    pairings: &RoundPairings,
    conn: &mut DbConn<'_>,
) -> Result<HashMap<PlayerId, u32>, DbError> {
    let FormatConfig::Swiss(configuration) = &state.configuration.format else {
        return Err(invalid_input(
            "Swiss ratings requested for another tournament format",
        ));
    };
    let players = pairings
        .games
        .iter()
        .flat_map(|pairing| [pairing.white(), pairing.black()])
        .chain(pairings.byes.iter().map(|bye_| bye_.player))
        .collect::<Vec<_>>();
    let speed = GameSpeed::from(configuration.clock);
    let player_users = players
        .iter()
        .copied()
        .map(|player| (player, state.memberships[player.index()].user_id))
        .collect::<Vec<_>>();
    let user_ids = player_users
        .iter()
        .map(|(_, user_id)| *user_id)
        .collect::<Vec<_>>();
    let ratings = Rating::for_uuids_at_speed(&user_ids, &speed, conn).await?;
    let ratings_by_user = ratings
        .into_iter()
        .map(|rating| (rating.user_uid, rating.rating))
        .collect::<HashMap<_, _>>();
    let mut snapshots = HashMap::with_capacity(players.len());
    for (player, user_id) in player_users {
        let rating = rating::snapshot(ratings_by_user[&user_id])
            .ok_or_else(|| invalid_persisted("accepted Swiss player has an invalid rating"))?;
        snapshots.insert(player, rating);
    }
    Ok(snapshots)
}

fn next_round_input(state: &TournamentState) -> SwissNextRoundInput {
    SwissNextRoundInput {
        eligible: state
            .memberships
            .iter()
            .enumerate()
            .filter(|(_, membership)| membership.withdrawn_at.is_none())
            .map(|(index, _)| PlayerId::new(index))
            .collect(),
        requested_byes: Vec::new(),
        restrictions: Vec::new(),
    }
}

pub(crate) fn swiss_next_round_is_exhausted(
    state: &TournamentState,
    projection: &SwissFactsProjection,
) -> Result<bool, DbError> {
    if projection.tournament_complete() || !projection.round_complete() {
        return Ok(false);
    }
    match swiss::pair_next_round(&projection.facts, &next_round_input(state)) {
        Ok(SwissNextRound::Pairings(_)) => Ok(false),
        Ok(SwissNextRound::PairingExhausted) => Ok(true),
        Err(error) => Err(invalid_persisted(&format!(
            "Swiss next-round projection failed: {error}",
        ))),
    }
}

pub(crate) fn swiss_progress(
    state: &TournamentState,
    projected: &SwissFactsProjection,
) -> Result<SwissProgress, DbError> {
    Ok(match projected.projection.history_status {
        SwissHistoryStatus::AwaitingResults => SwissProgress::AwaitingResults,
        SwissHistoryStatus::ConfiguredRoundsComplete => SwissProgress::Complete,
        SwissHistoryStatus::ReadyToPair => {
            if swiss_next_round_is_exhausted(state, projected)? {
                SwissProgress::PairingExhausted
            } else {
                SwissProgress::ReadyForNextRound
            }
        }
    })
}

pub(crate) async fn seal_swiss_terminal_for_batch(
    state: &mut TournamentState,
    game_id: Uuid,
    sealed_at: DateTime<Utc>,
    schedule_cleanup_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<bool, DbError> {
    let slot_id = slot_id_for_game(state, game_id)?;
    let slot = state.slots[state.slot_index(slot_id)?].clone();
    swiss_coordinates(state, slot.id)?;
    let game = state.game(game_id)?;
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

pub(crate) fn ensure_planned_double_swiss_second_adjudication(
    state: &TournamentState,
    slot_id: Uuid,
) -> Result<(), DbError> {
    let slot = &state.slots[state.slot_index(slot_id)?];
    let (round_id, pairing_ordinal, leg) = swiss_coordinates(state, slot_id)?;
    if leg != SwissLeg::Second {
        return Err(invalid_input(
            "only a Double-Swiss second leg may be adjudicated while planned",
        ));
    }
    if slot.resolution.is_some() || state.slot_game(slot_id).is_some() {
        return Err(invalid_input(
            "only a planned Double-Swiss second leg may be adjudicated",
        ));
    }
    let first_key = swiss_key(round_id, pairing_ordinal, SwissLeg::First);
    let first = state
        .slots
        .iter()
        .find(|slot| slot.key == first_key)
        .expect("accepted Double-Swiss pairing has a first leg");
    if first.resolution.is_none() {
        return Err(invalid_input("the Double-Swiss first leg is not resolved"));
    }
    Ok(())
}

fn double_swiss_first_clear_allowed(state: &TournamentState, second_leg: &TournamentSlot) -> bool {
    second_leg.resolution.is_none() && state.slot_game(second_leg.id).is_none()
}

pub(crate) async fn release_cleared_double_swiss_second_if_eligible(
    state: &mut TournamentState,
    slot_id: Uuid,
    released_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Game>, DbError> {
    ensure_planned_double_swiss_second_adjudication(state, slot_id)?;
    Ok(vec![release_slot(state, slot_id, released_at, conn).await?])
}

async fn release_double_second_if_ready(
    state: &mut TournamentState,
    slot_id: Uuid,
    released_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Game>, DbError> {
    let (round_id, pairing_ordinal, leg) = swiss_coordinates(state, slot_id)?;
    if leg != SwissLeg::First {
        return Ok(Vec::new());
    }
    let second_key = swiss_key(round_id, pairing_ordinal, SwissLeg::Second);
    let second = state
        .slots
        .iter()
        .find(|slot| slot.key == second_key)
        .expect("accepted Double-Swiss pairing has a second leg");
    if double_swiss_first_clear_allowed(state, second) {
        return Ok(vec![
            release_slot(state, second.id, released_at, conn).await?,
        ]);
    }
    Ok(Vec::new())
}

fn round_slots(
    state: &TournamentState,
    round_id: u32,
    pairings: &RoundPairings,
) -> Result<(Vec<TournamentSlotInsert>, Vec<SlotKey>), DbError> {
    let FormatConfig::Swiss(config) = &state.configuration.format else {
        return Err(invalid_input(
            "The Swiss Slot adapter received another tournament format",
        ));
    };
    let double = matches!(config.system, SwissSystem::DoubleSwiss(_));
    let mut slots = Vec::new();
    let mut release = Vec::new();
    for (pairing_ordinal, pairing) in pairings.games.iter().copied().enumerate() {
        let pairing_ordinal =
            u32::try_from(pairing_ordinal).expect("admitted Swiss pairing ordinal fits u32");
        let legs = if double {
            vec![SwissLeg::First, SwissLeg::Second]
        } else {
            vec![SwissLeg::Single]
        };
        for leg in legs {
            let (white, black) = if leg == SwissLeg::Second {
                (pairing.black(), pairing.white())
            } else {
                (pairing.white(), pairing.black())
            };
            let key = swiss_key(round_id, pairing_ordinal, leg);
            let white = &state.memberships[white.index()];
            let black = &state.memberships[black.index()];
            let slot = TournamentSlotInsert {
                key,
                white: white.user_id,
                black: black.user_id,
                clock: config.clock,
                resolution: None,
            };
            if leg != SwissLeg::Second
                || config.double_swiss_release_policy == ReleasePolicy::FullyUnlocked
            {
                release.push(slot.key);
            }
            slots.push(slot);
        }
    }
    Ok((slots, release))
}

fn eligible_players(pairings: &RoundPairings) -> Vec<PlayerId> {
    let mut eligible = pairings
        .games
        .iter()
        .flat_map(|pairing| [pairing.white(), pairing.black()])
        .chain(pairings.byes.iter().map(|bye_| bye_.player))
        .collect::<Vec<_>>();
    eligible.sort_unstable_by_key(|player| player.index());
    eligible
}

fn swiss_coordinates(
    state: &TournamentState,
    slot_id: Uuid,
) -> Result<(u32, u32, SwissLeg), DbError> {
    let slot = &state.slots[state.slot_index(slot_id)?];
    let SlotKey::Swiss { slot: game } = slot.key else {
        return Err(invalid_input("the Slot is not in a Swiss tournament"));
    };
    Ok((game.round_index, game.pairing_index, game.leg))
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

fn swiss_key(round_id: u32, pairing_index: u32, leg: SwissLeg) -> SlotKey {
    SlotKey::Swiss {
        slot: SwissGameId {
            round_index: round_id,
            pairing_index,
            leg,
        },
    }
}
