use super::schedules::ScheduleMap;
use crate::responses::{
    TournamentLifecycleDetails,
    TournamentMemberships,
    TournamentPatch,
    TournamentResponse,
    TournamentStandings,
};
use hive_lib::Color;
use leptos::{prelude::*, reactive::effect::batch};
use reactive_stores::{ArcField, Store};
use shared_types::{
    tournament::{
        arena::Config as ArenaConfig,
        elimination::{
            Config as EliminationConfig,
            Source as EliminationSource,
            Stage as EliminationStage,
        },
        round_robin::Config as RoundRobinConfig,
        swiss::Config as SwissConfig,
        BotAdmission,
        EliminationNodeId,
        Format,
        FormatConfig,
        Score,
        SlotKey,
    },
    tournament_view::{
        ArenaGameResponse,
        ArenaPlayerStatsResponse,
        EliminationNodeStateResponse,
        EliminationPlayerResultResponse,
        RoundRobinMatchResponse,
        SlotResponse,
        SwissByeResponse,
        SwissMatchCompletionResponse,
        TournamentFormatResponse,
    },
    GameId,
    SwissProgress,
    TournamentId,
};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
};
use uuid::Uuid;

#[derive(Clone, Debug, Store)]
pub struct TournamentCommon {
    pub(crate) lifecycle: TournamentLifecycleDetails,
    pub(crate) memberships: TournamentMemberships,
    pub(crate) standings: TournamentStandings,
    pub(crate) bot_admission: BotAdmission,
}

#[derive(Clone, Debug, Store)]
pub struct ArenaState {
    pub(crate) configuration: ArenaConfig,
    #[store(key: GameId = |(game_id, _)| game_id.clone())]
    pub(crate) games: HashMap<GameId, ArenaGameResponse>,
    #[store(key: Uuid = |(player_id, _)| *player_id)]
    pub(crate) player_stats: HashMap<Uuid, ArenaPlayerStatsResponse>,
    pub(crate) featured_game_id: Option<GameId>,
}

#[derive(Clone, Debug, Store)]
pub struct RoundRobinState {
    pub(crate) configuration: RoundRobinConfig,
    pub(crate) rounds: Vec<RoundRobinRound>,
    pub(crate) matches: Vec<RoundRobinMatchResponse>,
    pub(crate) withdrawable_entrants: HashSet<Uuid>,
    pub(crate) closeout_eligible_slots: u32,
    #[store(key: Uuid = |(slot_id, _)| *slot_id)]
    pub(crate) slots: HashMap<Uuid, SlotResponse>,
}

#[derive(Clone, Debug)]
pub(crate) struct RoundRobinRound {
    pub(crate) round_index: usize,
    pub(crate) pass_index: usize,
    pub(crate) resting: Option<Uuid>,
    pub(crate) slots: Vec<RoundRobinSlot>,
}

#[derive(Clone, Debug)]
pub(crate) struct RoundRobinSlot {
    pub(crate) board_index: usize,
    pub(crate) slot_id: Uuid,
}

#[derive(Clone, Debug, Store)]
pub struct SwissState {
    pub(crate) configuration: SwissConfig,
    pub(crate) rounds: Vec<SwissRound>,
    pub(crate) progress: SwissProgress,
    pub(crate) withdrawable_entrants: HashSet<Uuid>,
    pub(crate) closeout_eligible_slots: u32,
    #[store(key: Uuid = |(slot_id, _)| *slot_id)]
    pub(crate) slots: HashMap<Uuid, SlotResponse>,
}

#[derive(Clone, Debug)]
pub(crate) struct SwissRound {
    pub(crate) round_index: u32,
    pub(crate) encounters: Vec<SwissEncounter>,
    pub(crate) byes: Vec<SwissByeResponse>,
}

#[derive(Clone, Debug)]
pub(crate) struct SwissEncounter {
    pub(crate) pairing_index: u32,
    pub(crate) participants: [Uuid; 2],
    pub(crate) pre_round_primary_scores: [Score; 2],
    pub(crate) rating_snapshots: [Option<u32>; 2],
    pub(crate) slot_ids: Vec<Uuid>,
    pub(crate) completion: Option<SwissMatchCompletionResponse>,
}

#[derive(Clone, Debug, Store)]
pub struct EliminationState {
    pub(crate) configuration: EliminationConfig,
    pub(crate) nodes: Vec<EliminationNode>,
    pub(crate) complete: bool,
    pub(crate) player_results: Vec<EliminationPlayerResultResponse>,
    pub(crate) withdrawable_entrants: HashSet<Uuid>,
    #[store(key: Uuid = |(slot_id, _)| *slot_id)]
    pub(crate) slots: HashMap<Uuid, SlotResponse>,
}

#[derive(Clone, Debug)]
pub(crate) struct EliminationNode {
    pub(crate) node_id: EliminationNodeId,
    pub(crate) wave_index: usize,
    pub(crate) stage: EliminationStage,
    pub(crate) stage_ordinal: usize,
    pub(crate) sources: [EliminationSource; 2],
    pub(crate) entrants: [Option<Uuid>; 2],
    pub(crate) possible_entrants: [Vec<Uuid>; 2],
    pub(crate) state: EliminationNodeStateResponse,
    pub(crate) series: Option<EliminationSeries>,
}

#[derive(Clone, Debug)]
pub(crate) struct EliminationSeries {
    pub(crate) score: [u64; 2],
    pub(crate) sets: Vec<EliminationSeriesSet>,
}

#[derive(Clone, Debug)]
pub(crate) struct EliminationSeriesSet {
    pub(crate) slots: Vec<EliminationSeriesSlot>,
}

#[derive(Clone, Debug)]
pub(crate) struct EliminationSeriesSlot {
    pub(crate) slot_id: Uuid,
}

#[derive(Clone, Debug)]
enum TournamentFormatState {
    Arena(ArenaState),
    RoundRobin(RoundRobinState),
    Swiss(SwissState),
    Elimination(EliminationState),
}

impl TournamentFormatState {
    fn format(&self) -> Format {
        match self {
            Self::Arena(_) => Format::Arena,
            Self::RoundRobin(_) => Format::RoundRobin,
            Self::Swiss(state) => FormatConfig::Swiss(state.configuration.clone()).kind(),
            Self::Elimination(state) => {
                FormatConfig::Elimination(state.configuration.clone()).kind()
            }
        }
    }

    fn mount(self) -> TournamentFormatStore {
        match self {
            Self::Arena(state) => TournamentFormatStore::Arena(Store::new(state)),
            Self::RoundRobin(state) => TournamentFormatStore::RoundRobin(Store::new(state)),
            Self::Swiss(state) => TournamentFormatStore::Swiss(Store::new(state)),
            Self::Elimination(state) => TournamentFormatStore::Elimination(Store::new(state)),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TournamentFormatStore {
    Arena(Store<ArenaState>),
    RoundRobin(Store<RoundRobinState>),
    Swiss(Store<SwissState>),
    Elimination(Store<EliminationState>),
}

impl TournamentFormatStore {
    pub(crate) fn format(self) -> Format {
        match self {
            Self::Arena(_) => Format::Arena,
            Self::RoundRobin(_) => Format::RoundRobin,
            Self::Swiss(state) => FormatConfig::Swiss(state.configuration().get_untracked()).kind(),
            Self::Elimination(state) => {
                FormatConfig::Elimination(state.configuration().get_untracked()).kind()
            }
        }
    }

    pub(crate) fn slot(self, slot_id: Uuid) -> Option<ArcField<SlotResponse>> {
        match self {
            Self::Arena(_) => None,
            Self::RoundRobin(state) => state
                .slots()
                .with_untracked(|slots| slots.contains_key(&slot_id))
                .then(|| state.slots().at_key(slot_id).into()),
            Self::Swiss(state) => state
                .slots()
                .with_untracked(|slots| slots.contains_key(&slot_id))
                .then(|| state.slots().at_key(slot_id).into()),
            Self::Elimination(state) => state
                .slots()
                .with_untracked(|slots| slots.contains_key(&slot_id))
                .then(|| state.slots().at_key(slot_id).into()),
        }
    }
}

pub type TournamentScheduleState = Store<ScheduleMap>;

#[derive(Clone, Copy, Debug)]
pub struct TournamentState {
    pub(crate) common: Store<TournamentCommon>,
    pub(crate) format: TournamentFormatStore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TournamentStateError {
    MismatchedFormat,
}

impl fmt::Display for TournamentStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MismatchedFormat => {
                formatter.write_str("tournament format data does not match its configuration")
            }
        }
    }
}

impl Error for TournamentStateError {}

impl TournamentState {
    pub(crate) fn new(response: TournamentResponse) -> Result<Self, TournamentStateError> {
        let (common, format) = state_from_response(response)?;
        Ok(Self {
            common: Store::new(common),
            format: format.mount(),
        })
    }

    pub fn tournament_id(self) -> TournamentId {
        self.common
            .lifecycle()
            .with_untracked(|lifecycle| lifecycle.tournament_id.clone())
    }

    pub fn apply_patch(self, patch: TournamentPatch) -> bool {
        batch(|| {
            match (patch, self.format) {
                (
                    TournamentPatch::ArenaBerserked { game_id, color },
                    TournamentFormatStore::Arena(state),
                ) => {
                    let current = state.games().at_key(game_id);
                    if let Some(mut game) = current.try_get_untracked() {
                        game.game.berserked[match color {
                            Color::White => 0,
                            Color::Black => 1,
                        }] = true;
                        current.set(game);
                    }
                }
                (TournamentPatch::ArenaGameUpsert(game), TournamentFormatStore::Arena(state)) => {
                    let game_id = game.game.game_id.clone();
                    if state
                        .games()
                        .with_untracked(|games| games.contains_key(&game_id))
                    {
                        state.games().at_key(game_id).set(game);
                    } else {
                        let mut games = state.games().get_untracked();
                        games.insert(game_id, game);
                        state.games().patch(games);
                    }
                }
                (
                    TournamentPatch::ArenaPlayerStatsUpsert(stats),
                    TournamentFormatStore::Arena(state),
                ) => {
                    if state.player_stats().with_untracked(|stats_by_player| {
                        stats_by_player.contains_key(&stats.player)
                    }) {
                        state.player_stats().at_key(stats.player).set(stats);
                    } else {
                        let mut stats_by_player = state.player_stats().get_untracked();
                        stats_by_player.insert(stats.player, stats);
                        state.player_stats().patch(stats_by_player);
                    }
                }
                (
                    TournamentPatch::ArenaFeaturedGameChanged(game_id),
                    TournamentFormatStore::Arena(state),
                ) => state.featured_game_id().set(game_id),
                (TournamentPatch::SlotUpsert(slot), format) => {
                    let Some(current) = format.slot(slot.id) else {
                        return false;
                    };
                    if current.with_untracked(|current| current.key != slot.key) {
                        return false;
                    }
                    current.set(slot);
                }
                (TournamentPatch::MembershipsReplace(memberships), _) => {
                    self.common.memberships().set(memberships)
                }
                (TournamentPatch::StandingsReplace(standings), _) => {
                    self.common.standings().set(standings)
                }
                (
                    TournamentPatch::RoundRobinAvailabilityReplace {
                        withdrawable_entrants,
                        closeout_eligible_slots,
                    },
                    TournamentFormatStore::RoundRobin(state),
                ) => {
                    state.withdrawable_entrants().set(withdrawable_entrants);
                    state.closeout_eligible_slots().set(closeout_eligible_slots);
                }
                (
                    TournamentPatch::SwissAvailabilityReplace {
                        withdrawable_entrants,
                        closeout_eligible_slots,
                    },
                    TournamentFormatStore::Swiss(state),
                ) => {
                    state.withdrawable_entrants().set(withdrawable_entrants);
                    state.closeout_eligible_slots().set(closeout_eligible_slots);
                }
                (
                    TournamentPatch::EliminationAvailabilityReplace {
                        withdrawable_entrants,
                    },
                    TournamentFormatStore::Elimination(state),
                ) => state.withdrawable_entrants().set(withdrawable_entrants),
                (TournamentPatch::FormatReplace(format), current) => {
                    let Ok(next) = format_state(format) else {
                        return false;
                    };
                    if next.format() != current.format() {
                        return false;
                    }
                    self.replace_format(next);
                }
                (TournamentPatch::LifecycleDetailsReplace(lifecycle), _) => {
                    if lifecycle.tournament_id != self.tournament_id() {
                        return false;
                    }
                    self.common.lifecycle().set(lifecycle);
                }
                (TournamentPatch::DescriptionChanged(description), _) => self
                    .common
                    .lifecycle()
                    .update(|lifecycle| lifecycle.description = description),
                _ => return false,
            }
            true
        })
    }

    pub fn apply_snapshot(self, response: TournamentResponse) -> bool {
        if response.tournament_id != self.tournament_id() {
            return false;
        }
        let Ok((common, format)) = state_from_response(response) else {
            return false;
        };
        if format.format() != self.format.format() {
            return false;
        }

        batch(|| {
            self.common.set(common);
            self.replace_format(format);
        });
        true
    }

    fn replace_format(self, next: TournamentFormatState) {
        match (self.format, next) {
            (TournamentFormatStore::Arena(current), TournamentFormatState::Arena(next)) => {
                current.configuration().set(next.configuration);
                current.games().patch(next.games);
                current.player_stats().patch(next.player_stats);
                current.featured_game_id().set(next.featured_game_id);
            }
            (
                TournamentFormatStore::RoundRobin(current),
                TournamentFormatState::RoundRobin(next),
            ) => {
                current.configuration().set(next.configuration);
                current.rounds().set(next.rounds);
                current.matches().set(next.matches);
                current
                    .withdrawable_entrants()
                    .set(next.withdrawable_entrants);
                current
                    .closeout_eligible_slots()
                    .set(next.closeout_eligible_slots);
                current.slots().patch(next.slots);
            }
            (TournamentFormatStore::Swiss(current), TournamentFormatState::Swiss(next)) => {
                current.configuration().set(next.configuration);
                current.rounds().set(next.rounds);
                current.progress().set(next.progress);
                current
                    .withdrawable_entrants()
                    .set(next.withdrawable_entrants);
                current
                    .closeout_eligible_slots()
                    .set(next.closeout_eligible_slots);
                current.slots().patch(next.slots);
            }
            (
                TournamentFormatStore::Elimination(current),
                TournamentFormatState::Elimination(next),
            ) => {
                current.configuration().set(next.configuration);
                current.nodes().set(next.nodes);
                current.complete().set(next.complete);
                current.player_results().set(next.player_results);
                current
                    .withdrawable_entrants()
                    .set(next.withdrawable_entrants);
                current.slots().patch(next.slots);
            }
            _ => unreachable!("Format replacement was prevalidated"),
        }
    }
}

fn state_from_response(
    response: TournamentResponse,
) -> Result<(TournamentCommon, TournamentFormatState), TournamentStateError> {
    let TournamentResponse {
        tournament_id,
        name,
        description,
        invitees,
        declined_invitees,
        players,
        pairing_numbers,
        organizers,
        organizer_invitees,
        withdrawn,
        status,
        bot_admission,
        format,
        player_stats,
        standings,
        seats,
        min_seats,
        invite_only,
        band_upper,
        band_lower,
        starts_at,
        started_at,
        finished_at,
        created_at,
        start_setup,
    } = response;
    let format = format_state(format)?;
    let common = TournamentCommon {
        lifecycle: TournamentLifecycleDetails {
            tournament_id,
            name,
            description,
            status,
            seats,
            min_seats,
            invite_only,
            band_upper,
            band_lower,
            starts_at,
            started_at,
            finished_at,
            created_at,
            start_setup,
        },
        memberships: TournamentMemberships {
            invitees,
            declined_invitees,
            players,
            pairing_numbers,
            organizers,
            organizer_invitees,
            withdrawn,
        },
        standings: TournamentStandings {
            player_stats,
            snapshot: standings,
        },
        bot_admission,
    };
    Ok((common, format))
}

fn insert_slot(
    slots: &mut HashMap<Uuid, SlotResponse>,
    slot: SlotResponse,
    format: Format,
) -> Result<Uuid, TournamentStateError> {
    let valid_key = matches!(
        (slot.key, format),
        (SlotKey::RoundRobin { .. }, Format::RoundRobin)
            | (SlotKey::Swiss { .. }, Format::Swiss)
            | (SlotKey::Elimination { .. }, Format::SingleElimination)
    );
    let id = slot.id;
    if !valid_key || slots.insert(id, slot).is_some() {
        return Err(TournamentStateError::MismatchedFormat);
    }
    Ok(id)
}

fn format_state(
    format: TournamentFormatResponse,
) -> Result<TournamentFormatState, TournamentStateError> {
    match format {
        TournamentFormatResponse::Arena {
            configuration,
            games,
            player_stats,
            featured_game_id,
        } => {
            let mut games_by_id = HashMap::new();
            for game in games {
                if games_by_id
                    .insert(game.game.game_id.clone(), game)
                    .is_some()
                {
                    return Err(TournamentStateError::MismatchedFormat);
                }
            }
            let mut stats_by_player = HashMap::new();
            for stats in player_stats {
                if stats_by_player.insert(stats.player, stats).is_some() {
                    return Err(TournamentStateError::MismatchedFormat);
                }
            }
            Ok(TournamentFormatState::Arena(ArenaState {
                configuration,
                games: games_by_id,
                player_stats: stats_by_player,
                featured_game_id,
            }))
        }
        TournamentFormatResponse::RoundRobin {
            configuration,
            rounds,
            matches,
            withdrawable_entrants,
            closeout_eligible_slots,
        } => {
            let mut slots = HashMap::new();
            let mut converted = Vec::with_capacity(rounds.len());
            for round in rounds {
                let mut round_slots = Vec::with_capacity(round.slots.len());
                for entry in round.slots {
                    round_slots.push(RoundRobinSlot {
                        board_index: entry.board_index,
                        slot_id: insert_slot(&mut slots, entry.slot, Format::RoundRobin)?,
                    });
                }
                converted.push(RoundRobinRound {
                    round_index: round.round_index,
                    pass_index: round.pass_index,
                    resting: round.resting,
                    slots: round_slots,
                });
            }
            let rounds = converted;
            Ok(TournamentFormatState::RoundRobin(RoundRobinState {
                configuration,
                rounds,
                matches,
                withdrawable_entrants,
                closeout_eligible_slots,
                slots,
            }))
        }
        TournamentFormatResponse::Swiss {
            configuration,
            rounds,
            progress,
            withdrawable_entrants,
            closeout_eligible_slots,
        } => {
            let mut slots = HashMap::new();
            let mut converted = Vec::with_capacity(rounds.len());
            for round in rounds {
                let mut encounters = Vec::with_capacity(round.encounters.len());
                for encounter in round.encounters {
                    let slot_ids = encounter
                        .slots
                        .into_iter()
                        .map(|slot| insert_slot(&mut slots, slot, Format::Swiss))
                        .collect::<Result<_, _>>()?;
                    encounters.push(SwissEncounter {
                        pairing_index: encounter.pairing_index,
                        participants: encounter.participants,
                        pre_round_primary_scores: encounter.pre_round_primary_scores,
                        rating_snapshots: encounter.rating_snapshots,
                        slot_ids,
                        completion: encounter.completion,
                    });
                }
                converted.push(SwissRound {
                    round_index: round.round_index,
                    encounters,
                    byes: round.byes,
                });
            }
            let rounds = converted;
            Ok(TournamentFormatState::Swiss(SwissState {
                configuration,
                rounds,
                progress,
                withdrawable_entrants,
                closeout_eligible_slots,
                slots,
            }))
        }
        TournamentFormatResponse::Elimination {
            configuration,
            nodes,
            complete,
            player_results,
            withdrawable_entrants,
        } => {
            let mut slots = HashMap::new();
            let mut converted = Vec::with_capacity(nodes.len());
            for node in nodes {
                let series = if let Some(series) = node.series {
                    let mut sets = Vec::with_capacity(series.sets.len());
                    for set in series.sets {
                        let set_slots = set
                            .slots
                            .into_iter()
                            .map(|entry| {
                                insert_slot(&mut slots, entry.slot, Format::SingleElimination)
                                    .map(|slot_id| EliminationSeriesSlot { slot_id })
                            })
                            .collect::<Result<_, _>>()?;
                        sets.push(EliminationSeriesSet { slots: set_slots });
                    }
                    Some(EliminationSeries {
                        score: series.score,
                        sets,
                    })
                } else {
                    None
                };
                converted.push(EliminationNode {
                    node_id: node.node_id,
                    wave_index: node.wave_index,
                    stage: node.stage,
                    stage_ordinal: node.stage_ordinal,
                    sources: node.sources,
                    entrants: node.entrants,
                    possible_entrants: node.possible_entrants,
                    state: node.state,
                    series,
                });
            }
            let nodes = converted;
            Ok(TournamentFormatState::Elimination(EliminationState {
                configuration,
                nodes,
                complete,
                player_results,
                withdrawable_entrants,
                slots,
            }))
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ActiveTournamentState {
    active: StoredValue<Option<RegisteredTournamentState>>,
    next_registration: StoredValue<u64>,
}

#[derive(Clone, Copy, Debug)]
struct RegisteredTournamentState {
    registration: ActiveTournamentRegistration,
    state: TournamentState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActiveTournamentRegistration(u64);

impl ActiveTournamentState {
    pub fn new() -> Self {
        Self {
            active: StoredValue::new(None),
            next_registration: StoredValue::new(0),
        }
    }

    pub fn register(self, state: TournamentState) -> ActiveTournamentRegistration {
        let mut registration = ActiveTournamentRegistration(0);
        self.next_registration.update_value(|next| {
            *next = next.wrapping_add(1);
            registration = ActiveTournamentRegistration(*next);
        });
        self.active.set_value(Some(RegisteredTournamentState {
            registration,
            state,
        }));
        registration
    }

    pub fn cleanup_should_unwatch(
        self,
        registration: ActiveTournamentRegistration,
        tournament_id: &TournamentId,
    ) -> bool {
        let mut should_unwatch = true;
        self.active.update_value(|active| {
            if active
                .as_ref()
                .is_some_and(|active| active.registration == registration)
            {
                *active = None;
            } else if active
                .as_ref()
                .is_some_and(|active| active.state.tournament_id() == *tournament_id)
            {
                should_unwatch = false;
            }
        });
        should_unwatch
    }

    pub fn state_for(self, tournament_id: &TournamentId) -> Option<TournamentState> {
        self.active.with_value(|active| {
            active
                .filter(|active| active.state.tournament_id() == *tournament_id)
                .map(|active| active.state)
        })
    }

    pub fn apply_patch(self, tournament_id: &TournamentId, patch: TournamentPatch) -> bool {
        self.state_for(tournament_id)
            .is_some_and(|state| state.apply_patch(patch))
    }
}

impl Default for ActiveTournamentState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_active_tournament_state() {
    provide_context(ActiveTournamentState::new());
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::tournament_view::{
        CompactTournamentGameResponse,
        RoundRobinRoundResponse,
        RoundRobinSlotResponse,
    };
    use tournamint::{
        round_robin::RoundRobinCompletedMatchProjection,
        GameOutcome,
        MatchDisposition,
        MatchScore,
        PlayedGameOutcome,
    };

    use chrono::{DateTime, Utc};
    use hive_lib::{GameResult, GameStatus};
    use leptos::prelude::Owner;
    use shared_types::{
        tournament::{arena::Config as ArenaConfig, BotAdmission, RoundRobinGameId},
        GameSpeed,
        GameStart,
        RealtimeClock,
        SlotAdminAction,
        TournamentStatus,
    };
    use std::{
        collections::HashSet,
        num::NonZeroU32,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };

    fn timestamp(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).unwrap()
    }

    fn compact_game(id: &str, status: GameStatus) -> CompactTournamentGameResponse {
        CompactTournamentGameResponse {
            game_id: GameId(id.to_string()),
            participants: [Uuid::from_u128(1), Uuid::from_u128(2)],
            status,
            start: GameStart::Ready,
            finished: false,
            ratings: [Some(1500), Some(1500)],
            berserked: [false, false],
            speed: GameSpeed::Blitz,
            finished_at: None,
        }
    }

    fn slot(id: u128, index: usize) -> SlotResponse {
        SlotResponse {
            id: Uuid::from_u128(id),
            key: SlotKey::RoundRobin {
                slot: RoundRobinGameId::new(index),
            },
            participants: [Uuid::from_u128(1), Uuid::from_u128(2)],
            clock: realtime_clock(),
            resolution: None,
            outcome: None,
            awarded_game_points: None,
            resolved_at: None,
            game: None,
            scheduled_at: None,
            deadline_at: None,
            waits_for: None,
            available_admin_actions: HashSet::from([SlotAdminAction::SetDeadline]),
        }
    }

    fn realtime_clock() -> shared_types::Clock {
        shared_types::Clock::Realtime(RealtimeClock {
            base_seconds: NonZeroU32::new(300).unwrap(),
            increment_seconds: 3,
        })
    }

    fn response() -> TournamentResponse {
        let first = slot(3, 0);
        let second = slot(4, 1);
        TournamentResponse {
            tournament_id: TournamentId(String::from("store-test")),
            name: String::from("Store test"),
            description: Some(String::from("snapshot")),
            invitees: Vec::new(),
            declined_invitees: Vec::new(),
            players: HashMap::new(),
            pairing_numbers: HashMap::new(),
            organizers: Vec::new(),
            organizer_invitees: Vec::new(),
            start_setup: None,
            withdrawn: HashSet::new(),
            status: TournamentStatus::InProgress,
            bot_admission: BotAdmission::HumansAndBots,
            format: TournamentFormatResponse::RoundRobin {
                matches: Vec::new(),
                configuration: RoundRobinConfig::standard(NonZeroU32::new(1).unwrap(), first.clock),
                rounds: vec![RoundRobinRoundResponse {
                    round_index: 0,
                    pass_index: 0,
                    resting: None,
                    slots: vec![
                        RoundRobinSlotResponse {
                            board_index: 0,
                            slot: first,
                        },
                        RoundRobinSlotResponse {
                            board_index: 1,
                            slot: second,
                        },
                    ],
                }],
                withdrawable_entrants: HashSet::new(),
                closeout_eligible_slots: 2,
            },
            player_stats: Vec::new(),
            standings: None,
            seats: Some(2),
            min_seats: 2,
            invite_only: false,
            band_upper: None,
            band_lower: None,
            starts_at: None,
            started_at: Some(timestamp(0)),
            finished_at: None,
            created_at: timestamp(0),
        }
    }

    fn arena_response() -> TournamentResponse {
        let mut response = response();
        response.format = TournamentFormatResponse::Arena {
            configuration: ArenaConfig::new(
                NonZeroU32::new(3600).unwrap(),
                match realtime_clock() {
                    shared_types::Clock::Realtime(clock) => clock,
                    _ => unreachable!(),
                },
            ),
            games: vec![arena_game("first", 1), arena_game("second", 2)],
            player_stats: Vec::new(),
            featured_game_id: None,
        };
        response
    }

    fn arena_game(id: &str, ordinal: i64) -> ArenaGameResponse {
        ArenaGameResponse {
            ordinal,
            game: compact_game(id, GameStatus::InProgress),
            outcome: None,
            awarded_points: None,
            doubled: None,
        }
    }

    #[test]
    fn stale_same_tournament_cleanup_keeps_the_replacement_watch() {
        let owner = Owner::new();
        owner.with(|| {
            let active = ActiveTournamentState::new();
            let first = TournamentState::new(response()).unwrap();
            let first_registration = active.register(first);
            let second = TournamentState::new(response()).unwrap();
            let second_registration = active.register(second);
            let tournament_id = TournamentId(String::from("store-test"));

            assert!(!active.cleanup_should_unwatch(first_registration, &tournament_id));

            assert!(active.state_for(&tournament_id).is_some());
            assert_eq!(
                second_registration,
                active.active.get_value().unwrap().registration
            );
        });
    }

    #[test]
    fn stale_different_tournament_cleanup_unwatches_only_the_old_tournament() {
        let owner = Owner::new();
        owner.with(|| {
            let active = ActiveTournamentState::new();
            let first = TournamentState::new(response()).unwrap();
            let first_registration = active.register(first);
            let mut second_response = response();
            second_response.tournament_id = TournamentId(String::from("second"));
            let second = TournamentState::new(second_response).unwrap();
            let second_registration = active.register(second);
            let first_id = TournamentId(String::from("store-test"));
            let second_id = TournamentId(String::from("second"));

            assert!(active.cleanup_should_unwatch(first_registration, &first_id));
            assert!(active.state_for(&second_id).is_some());
            assert_eq!(
                second_registration,
                active.active.get_value().unwrap().registration
            );
        });
    }

    #[test]
    fn exact_cleanup_removes_the_route_and_later_stale_cleanup_still_unwatches() {
        let owner = Owner::new();
        owner.with(|| {
            let active = ActiveTournamentState::new();
            let state = TournamentState::new(response()).unwrap();
            let registration = active.register(state);
            let tournament_id = TournamentId(String::from("store-test"));

            assert!(active.cleanup_should_unwatch(registration, &tournament_id));
            assert!(active.state_for(&tournament_id).is_none());
            assert!(active
                .cleanup_should_unwatch(ActiveTournamentRegistration(u64::MAX), &tournament_id,));
        });
    }

    #[test]
    fn standings_patch_does_not_notify_lifecycle_consumers() {
        let owner = Owner::new();
        owner.with(|| {
            let state = TournamentState::new(response()).unwrap();
            let runs = Arc::new(AtomicUsize::new(0));
            let lifecycle_name = Memo::new({
                let runs = Arc::clone(&runs);
                move |_| {
                    runs.fetch_add(1, Ordering::Relaxed);
                    state.common.lifecycle().get().name
                }
            });

            assert_eq!(lifecycle_name.get(), "Store test");
            assert_eq!(runs.load(Ordering::Relaxed), 1);
            assert!(state.apply_patch(TournamentPatch::StandingsReplace(
                TournamentStandings::default(),
            )));
            assert_eq!(lifecycle_name.get(), "Store test");
            assert_eq!(runs.load(Ordering::Relaxed), 1);
        });
    }

    #[test]
    fn mismatched_format_inputs_are_rejected_without_mutating_arena_state() {
        let owner = Owner::new();
        owner.with(|| {
            let state = TournamentState::new(arena_response()).unwrap();
            let TournamentFormatStore::Arena(arena) = state.format else {
                unreachable!()
            };
            let before_configuration = arena.configuration().get_untracked();
            let before_games = arena.games().get_untracked();
            let before_stats = arena.player_stats().get_untracked();
            let before_featured = arena.featured_game_id().get_untracked();

            assert!(
                !state.apply_patch(TournamentPatch::RoundRobinAvailabilityReplace {
                    withdrawable_entrants: HashSet::new(),
                    closeout_eligible_slots: 1,
                })
            );
            assert!(!state.apply_patch(TournamentPatch::SlotUpsert(slot(9, 9))));
            assert!(!state.apply_patch(TournamentPatch::FormatReplace(response().format)));
            assert!(!state.apply_snapshot(response()));

            assert_eq!(arena.configuration().get_untracked(), before_configuration);
            assert_eq!(arena.games().get_untracked(), before_games);
            assert_eq!(arena.player_stats().get_untracked(), before_stats);
            assert_eq!(arena.featured_game_id().get_untracked(), before_featured);
        });
    }

    #[test]
    fn slot_patch_cannot_insert_an_orphan_or_change_structural_identity() {
        let owner = Owner::new();
        owner.with(|| {
            let state = TournamentState::new(response()).unwrap();
            let TournamentFormatStore::RoundRobin(format) = state.format else {
                unreachable!()
            };
            let initial_slots = format.slots().get_untracked();

            assert!(!state.apply_patch(TournamentPatch::SlotUpsert(slot(9, 9))));

            let mut changed_key = initial_slots[&Uuid::from_u128(3)].clone();
            changed_key.key = SlotKey::RoundRobin {
                slot: RoundRobinGameId::new(99),
            };
            assert!(!state.apply_patch(TournamentPatch::SlotUpsert(changed_key)));
            assert_eq!(format.slots().get_untracked(), initial_slots);
        });
    }

    #[test]
    fn keyed_slot_update_does_not_notify_sibling_slot() {
        let owner = Owner::new();
        owner.with(|| {
            let state = TournamentState::new(response()).unwrap();
            let TournamentFormatStore::RoundRobin(format) = state.format else {
                unreachable!()
            };
            let first: ArcField<SlotResponse> = format.slots().at_key(Uuid::from_u128(3)).into();
            let second: ArcField<SlotResponse> = format.slots().at_key(Uuid::from_u128(4)).into();
            let first_runs = Arc::new(AtomicUsize::new(0));
            let second_runs = Arc::new(AtomicUsize::new(0));
            let first_deadline = Memo::new({
                let runs = Arc::clone(&first_runs);
                let first = first.clone();
                move |_| {
                    runs.fetch_add(1, Ordering::Relaxed);
                    first.get().deadline_at
                }
            });
            let second_deadline = Memo::new({
                let runs = Arc::clone(&second_runs);
                let second = second.clone();
                move |_| {
                    runs.fetch_add(1, Ordering::Relaxed);
                    second.get().deadline_at
                }
            });
            assert_eq!(first_deadline.get(), None);
            assert_eq!(second_deadline.get(), None);

            let mut update = first.get_untracked();
            update.deadline_at = Some(timestamp(30));
            assert!(state.apply_patch(TournamentPatch::SlotUpsert(update)));

            assert_eq!(first_deadline.get(), Some(timestamp(30)));
            assert_eq!(second_deadline.get(), None);
            assert_eq!(first_runs.load(Ordering::Relaxed), 2);
            assert_eq!(second_runs.load(Ordering::Relaxed), 1);
        });
    }

    #[test]
    fn delayed_berserk_patch_preserves_awards_and_never_reinserts_excluded_games() {
        let owner = Owner::new();
        owner.with(|| {
            let game_id = GameId(String::from("first"));
            let mut terminal = arena_game("first", 1);
            terminal.game.finished = true;
            terminal.game.status = GameStatus::Finished(GameResult::Draw);
            terminal.outcome = Some(GameOutcome::Played(PlayedGameOutcome::Draw));
            terminal.awarded_points = Some([Score::new(1), Score::new(2)]);
            terminal.doubled = Some([false, true]);
            let mut response = arena_response();
            let TournamentFormatResponse::Arena { games, .. } = &mut response.format else {
                unreachable!()
            };
            games[0] = terminal.clone();
            let state = TournamentState::new(response).unwrap();
            let TournamentFormatStore::Arena(format) = state.format else {
                unreachable!()
            };
            for color in [Color::White, Color::White, Color::Black] {
                assert!(state.apply_patch(TournamentPatch::ArenaBerserked {
                    game_id: game_id.clone(),
                    color
                }));
            }
            terminal.game.berserked = [true, true];
            assert_eq!(
                format.games().at_key(game_id.clone()).get_untracked(),
                terminal
            );
            let mut included = format.games().get_untracked();
            included.remove(&game_id);
            format.games().patch(included);
            assert!(state.apply_patch(TournamentPatch::ArenaBerserked {
                game_id: game_id.clone(),
                color: Color::White
            }));
            assert!(!format
                .games()
                .with_untracked(|games| games.contains_key(&game_id)));
        });
    }

    #[test]
    fn keyed_arena_game_update_does_not_notify_sibling_game() {
        let owner = Owner::new();
        owner.with(|| {
            let state = TournamentState::new(arena_response()).unwrap();
            let TournamentFormatStore::Arena(format) = state.format else {
                unreachable!()
            };
            let first_id = GameId(String::from("first"));
            let second_id = GameId(String::from("second"));
            let first: ArcField<ArenaGameResponse> = format.games().at_key(first_id.clone()).into();
            let second: ArcField<ArenaGameResponse> = format.games().at_key(second_id).into();
            let first_runs = Arc::new(AtomicUsize::new(0));
            let second_runs = Arc::new(AtomicUsize::new(0));
            let first_ordinal = Memo::new({
                let runs = Arc::clone(&first_runs);
                let first = first.clone();
                move |_| {
                    runs.fetch_add(1, Ordering::Relaxed);
                    first.get().ordinal
                }
            });
            let second_ordinal = Memo::new({
                let runs = Arc::clone(&second_runs);
                let second = second.clone();
                move |_| {
                    runs.fetch_add(1, Ordering::Relaxed);
                    second.get().ordinal
                }
            });
            assert_eq!(first_ordinal.get(), 1);
            assert_eq!(second_ordinal.get(), 2);

            assert!(state.apply_patch(TournamentPatch::ArenaGameUpsert(arena_game("first", 3,))));

            assert_eq!(first_ordinal.get(), 3);
            assert_eq!(second_ordinal.get(), 2);
            assert_eq!(first_runs.load(Ordering::Relaxed), 2);
            assert_eq!(second_runs.load(Ordering::Relaxed), 1);
        });
    }

    #[test]
    fn full_snapshot_replaces_round_robin_match_results_after_correction() {
        let owner = Owner::new();
        owner.with(|| {
            let state = TournamentState::new(response()).unwrap();
            let TournamentFormatStore::RoundRobin(format) = state.format else {
                unreachable!()
            };
            let completed = RoundRobinCompletedMatchProjection {
                dispositions: [MatchDisposition::Regular; 2],
                aggregate: [MatchScore::Win, MatchScore::Loss],
                game_points: [Score::new(4), Score::new(0)],
                match_points: [Score::new(2), Score::new(0)],
            };
            for completion in [Some(completed), None] {
                let mut replacement = response();
                let TournamentFormatResponse::RoundRobin { matches, .. } = &mut replacement.format
                else {
                    unreachable!()
                };
                matches.push(RoundRobinMatchResponse {
                    participants: [Uuid::from_u128(1), Uuid::from_u128(2)],
                    completion,
                });
                assert!(state.apply_snapshot(replacement));
                assert_eq!(format.matches().get()[0].completion, completion);
            }
        });
    }

    #[test]
    fn full_snapshot_refreshes_keys_for_new_fixed_field_slot() {
        let owner = Owner::new();
        owner.with(|| {
            let state = TournamentState::new(response()).unwrap();
            let TournamentFormatStore::RoundRobin(format) = state.format else {
                unreachable!()
            };
            let mut replacement = response();
            let TournamentFormatResponse::RoundRobin { rounds, .. } = &mut replacement.format
            else {
                unreachable!()
            };
            rounds[0].slots.push(RoundRobinSlotResponse {
                board_index: 2,
                slot: slot(5, 2),
            });

            assert!(state.apply_snapshot(replacement));
            let inserted: ArcField<SlotResponse> = format.slots().at_key(Uuid::from_u128(5)).into();
            assert_eq!(inserted.get().id, Uuid::from_u128(5));
        });
    }

    #[test]
    fn format_patch_refreshes_keys_for_new_arena_game() {
        let owner = Owner::new();
        owner.with(|| {
            let state = TournamentState::new(arena_response()).unwrap();
            let TournamentFormatStore::Arena(format) = state.format else {
                unreachable!()
            };
            let existing_id = GameId(String::from("first"));
            let existing: ArcField<ArenaGameResponse> =
                format.games().at_key(existing_id.clone()).into();
            let existing_runs = Arc::new(AtomicUsize::new(0));
            let existing_game_id = Memo::new({
                let existing = existing.clone();
                let existing_runs = Arc::clone(&existing_runs);
                move |_| {
                    existing_runs.fetch_add(1, Ordering::Relaxed);
                    existing.get().game.game_id
                }
            });
            assert_eq!(existing_game_id.get(), existing_id);

            let mut replacement = arena_response().format;
            let TournamentFormatResponse::Arena { games, .. } = &mut replacement else {
                unreachable!()
            };
            games.push(arena_game("third", 3));

            assert!(state.apply_patch(TournamentPatch::FormatReplace(replacement)));
            let inserted_id = GameId(String::from("third"));
            let inserted: ArcField<ArenaGameResponse> =
                format.games().at_key(inserted_id.clone()).into();
            assert_eq!(inserted.get().game.game_id, inserted_id);
            assert_eq!(existing_game_id.get(), existing_id);
            assert_eq!(existing_runs.load(Ordering::Relaxed), 1);
        });
    }
}
