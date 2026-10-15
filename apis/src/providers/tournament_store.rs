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
    tournament::{BotAdmission, Format, FormatConfig, SlotKey},
    tournament_view::{SlotResponse, TournamentFormatResponse},
    Clock,
    TournamentId,
};
use std::{error::Error, fmt};
use uuid::Uuid;

#[derive(Clone, Debug, Store)]
pub struct TournamentCommon {
    pub(crate) lifecycle: TournamentLifecycleDetails,
    pub(crate) memberships: TournamentMemberships,
    pub(crate) standings: TournamentStandings,
    pub(crate) bot_admission: BotAdmission,
}

pub use shared_types::tournament_view::{
    ArenaState,
    ArenaStateStoreFields,
    EliminationNodeResponse as EliminationNode,
    EliminationState,
    EliminationStateStoreFields,
    RoundRobinRoundResponse as RoundRobinRound,
    RoundRobinState,
    RoundRobinStateStoreFields,
    SwissRoundResponse as SwissRound,
    SwissState,
    SwissStateStoreFields,
};

fn mount_format(format: TournamentFormatResponse) -> TournamentFormatState {
    // reactive_stores 0.4.3 does not fully initialize keys on first access.
    // Register them before exposing the store so initial patches and keyed writes work.
    match format {
        TournamentFormatResponse::Arena(state) => {
            let store = Store::new(state);
            store.games().update_keys();
            store.player_stats().update_keys();
            TournamentFormatState::Arena(store)
        }
        TournamentFormatResponse::RoundRobin(state) => {
            let store = Store::new(state);
            store.slots().update_keys();
            TournamentFormatState::RoundRobin(store)
        }
        TournamentFormatResponse::Swiss(state) => {
            let store = Store::new(state);
            store.slots().update_keys();
            TournamentFormatState::Swiss(store)
        }
        TournamentFormatResponse::Elimination(state) => {
            let store = Store::new(state);
            store.slots().update_keys();
            TournamentFormatState::Elimination(store)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TournamentFormatState {
    Arena(Store<ArenaState>),
    RoundRobin(Store<RoundRobinState>),
    Swiss(Store<SwissState>),
    Elimination(Store<EliminationState>),
}

impl TournamentFormatState {
    pub fn display_clock(self) -> Option<Clock> {
        match self {
            Self::Arena(state) => Some(Clock::Realtime(
                state.configuration().get_untracked().game_clock,
            )),
            Self::RoundRobin(state) => Some(state.configuration().get_untracked().clock),
            Self::Swiss(state) => Some(state.configuration().get_untracked().clock),
            Self::Elimination(state) => state
                .configuration()
                .get_untracked()
                .default_plan
                .phases
                .first()
                .map(|phase| phase.clock),
        }
    }

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
    pub(crate) format: TournamentFormatState,
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
            format: mount_format(format),
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
                    TournamentFormatState::Arena(state),
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
                (TournamentPatch::ArenaGameUpsert(game), TournamentFormatState::Arena(state)) => {
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
                    TournamentFormatState::Arena(state),
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
                    TournamentFormatState::Arena(state),
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
                    TournamentFormatState::RoundRobin(state),
                ) => {
                    state.withdrawable_entrants().set(withdrawable_entrants);
                    state.closeout_eligible_slots().set(closeout_eligible_slots);
                }
                (
                    TournamentPatch::SwissAvailabilityReplace {
                        withdrawable_entrants,
                        closeout_eligible_slots,
                    },
                    TournamentFormatState::Swiss(state),
                ) => {
                    state.withdrawable_entrants().set(withdrawable_entrants);
                    state.closeout_eligible_slots().set(closeout_eligible_slots);
                }
                (
                    TournamentPatch::EliminationAvailabilityReplace {
                        withdrawable_entrants,
                    },
                    TournamentFormatState::Elimination(state),
                ) => state.withdrawable_entrants().set(withdrawable_entrants),
                (TournamentPatch::FormatReplace(format), current) => {
                    let Ok(next) = validate_format(format) else {
                        return false;
                    };
                    if next.kind() != current.format() {
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
        if response.lifecycle.tournament_id != self.tournament_id() {
            return false;
        }
        let Ok((common, format)) = state_from_response(response) else {
            return false;
        };
        if format.kind() != self.format.format() {
            return false;
        }

        batch(|| {
            self.common.set(common);
            self.replace_format(format);
        });
        true
    }

    fn replace_format(self, next: TournamentFormatResponse) {
        match (self.format, next) {
            (TournamentFormatState::Arena(current), TournamentFormatResponse::Arena(next)) => {
                current.configuration().set(next.configuration);
                current.games().patch(next.games);
                current.player_stats().patch(next.player_stats);
                current.featured_game_id().set(next.featured_game_id);
            }
            (
                TournamentFormatState::RoundRobin(current),
                TournamentFormatResponse::RoundRobin(next),
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
            (TournamentFormatState::Swiss(current), TournamentFormatResponse::Swiss(next)) => {
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
                TournamentFormatState::Elimination(current),
                TournamentFormatResponse::Elimination(next),
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
) -> Result<(TournamentCommon, TournamentFormatResponse), TournamentStateError> {
    let TournamentResponse {
        lifecycle,
        memberships,
        standings,
        bot_admission,
        format,
    } = response;
    let format = validate_format(format)?;
    Ok((
        TournamentCommon {
            lifecycle,
            memberships,
            standings,
            bot_admission,
        },
        format,
    ))
}

fn validate_format(
    format: TournamentFormatResponse,
) -> Result<TournamentFormatResponse, TournamentStateError> {
    let valid = match &format {
        TournamentFormatResponse::Arena(state) => {
            state
                .games
                .iter()
                .all(|(id, game)| id == &game.game.game_id)
                && state
                    .player_stats
                    .iter()
                    .all(|(id, stats)| id == &stats.player)
        }
        TournamentFormatResponse::RoundRobin(state) => state
            .slots
            .iter()
            .all(|(id, slot)| *id == slot.id && matches!(slot.key, SlotKey::RoundRobin { .. })),
        TournamentFormatResponse::Swiss(state) => state
            .slots
            .iter()
            .all(|(id, slot)| *id == slot.id && matches!(slot.key, SlotKey::Swiss { .. })),
        TournamentFormatResponse::Elimination(state) => state
            .slots
            .iter()
            .all(|(id, slot)| *id == slot.id && matches!(slot.key, SlotKey::Elimination { .. })),
    };
    valid
        .then_some(format)
        .ok_or(TournamentStateError::MismatchedFormat)
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
        ArenaGameResponse,
        CompactTournamentGameResponse,
        RoundRobinMatchResponse,
        RoundRobinRoundResponse,
        RoundRobinSlotResponse,
        SwissEncounterResponse,
        SwissRoundResponse,
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
        tournament::{
            arena::Config as ArenaConfig,
            round_robin::Config as RoundRobinConfig,
            swiss::Config as SwissConfig,
            BotAdmission,
            RoundRobinGameId,
            Score,
            SwissGameId,
            SwissLeg,
        },
        GameId,
        GameSpeed,
        GameStart,
        RealtimeClock,
        SlotAdminAction,
        SwissProgress,
        TournamentStatus,
    };
    use std::{
        collections::{HashMap, HashSet},
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
            available_results: Vec::new(),
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
            lifecycle: TournamentLifecycleDetails {
                tournament_id: TournamentId(String::from("store-test")),
                name: String::from("Store test"),
                description: Some(String::from("snapshot")),
                start_setup: None,
                status: TournamentStatus::InProgress,
                seats: Some(2),
                min_seats: 2,
                invite_only: false,
                band_upper: None,
                band_lower: None,
                starts_at: None,
                started_at: Some(timestamp(0)),
                finished_at: None,
                created_at: timestamp(0),
            },
            memberships: TournamentMemberships::default(),
            standings: TournamentStandings::default(),
            bot_admission: BotAdmission::HumansAndBots,
            format: TournamentFormatResponse::RoundRobin(RoundRobinState {
                configuration: RoundRobinConfig::standard(NonZeroU32::new(1).unwrap(), first.clock),
                matches: Vec::new(),
                rounds: vec![RoundRobinRoundResponse {
                    round_index: 0,
                    pass_index: 0,
                    resting: None,
                    slots: vec![
                        RoundRobinSlotResponse {
                            board_index: 0,
                            slot_id: first.id,
                        },
                        RoundRobinSlotResponse {
                            board_index: 1,
                            slot_id: second.id,
                        },
                    ],
                }],
                slots: HashMap::from([(first.id, first), (second.id, second)]),
                withdrawable_entrants: HashSet::new(),
                closeout_eligible_slots: 2,
            }),
        }
    }

    fn arena_response() -> TournamentResponse {
        let mut response = response();
        response.format = TournamentFormatResponse::Arena(ArenaState {
            configuration: ArenaConfig::new(
                NonZeroU32::new(3600).unwrap(),
                match realtime_clock() {
                    shared_types::Clock::Realtime(clock) => clock,
                    _ => unreachable!(),
                },
            ),
            games: [arena_game("first", 1), arena_game("second", 2)]
                .into_iter()
                .map(|game| (game.game.game_id.clone(), game))
                .collect(),
            player_stats: HashMap::new(),
            featured_game_id: None,
        });
        response
    }

    fn swiss_response(deadline_at: Option<DateTime<Utc>>) -> TournamentResponse {
        let mut response = response();
        let mut slot = slot(3, 0);
        slot.key = SlotKey::Swiss {
            slot: SwissGameId {
                round_index: 0,
                pairing_index: 0,
                leg: SwissLeg::First,
            },
        };
        slot.deadline_at = deadline_at;
        response.format = TournamentFormatResponse::Swiss(SwissState {
            slots: HashMap::from([(slot.id, slot.clone())]),
            configuration: SwissConfig::swiss(NonZeroU32::new(3).unwrap(), slot.clock),
            rounds: vec![SwissRoundResponse {
                round_index: 0,
                encounters: vec![SwissEncounterResponse {
                    pairing_index: 0,
                    participants: slot.participants,
                    pre_round_primary_scores: [Score::new(0); 2],
                    rating_snapshots: [Some(1500); 2],
                    slot_ids: vec![slot.id],
                    completion: None,
                }],
                byes: Vec::new(),
            }],
            progress: SwissProgress::AwaitingResults,
            withdrawable_entrants: HashSet::new(),
            closeout_eligible_slots: 1,
        });
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
            second_response.lifecycle.tournament_id = TournamentId(String::from("second"));
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
            let TournamentFormatState::Arena(arena) = state.format else {
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
            let TournamentFormatState::RoundRobin(format) = state.format else {
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
            let TournamentFormatState::RoundRobin(format) = state.format else {
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
            let TournamentFormatResponse::Arena(ArenaState { games, .. }) = &mut response.format
            else {
                unreachable!()
            };
            games.insert(game_id.clone(), terminal.clone());
            let state = TournamentState::new(response).unwrap();
            let TournamentFormatState::Arena(format) = state.format else {
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
            let TournamentFormatState::Arena(format) = state.format else {
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
    fn swiss_format_patch_updates_previously_read_slots() {
        let owner = Owner::new();
        owner.with(|| {
            let state = TournamentState::new(swiss_response(None)).unwrap();
            let slot = state.format.slot(Uuid::from_u128(3)).unwrap();
            let deadline = Memo::new(move |_| slot.get().deadline_at);
            assert_eq!(deadline.get(), None);

            let replacement = swiss_response(Some(timestamp(30))).format;
            assert!(state.apply_patch(TournamentPatch::FormatReplace(replacement)));

            assert_eq!(deadline.get(), Some(timestamp(30)));
        });
    }

    #[test]
    fn full_snapshot_replaces_round_robin_match_results_after_correction() {
        let owner = Owner::new();
        owner.with(|| {
            let state = TournamentState::new(response()).unwrap();
            let TournamentFormatState::RoundRobin(format) = state.format else {
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
                let TournamentFormatResponse::RoundRobin(RoundRobinState { matches, .. }) =
                    &mut replacement.format
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
            let TournamentFormatState::RoundRobin(format) = state.format else {
                unreachable!()
            };
            let mut replacement = response();
            let TournamentFormatResponse::RoundRobin(RoundRobinState { rounds, slots, .. }) =
                &mut replacement.format
            else {
                unreachable!()
            };
            rounds[0].slots.push(RoundRobinSlotResponse {
                board_index: 2,
                slot_id: Uuid::from_u128(5),
            });

            slots.insert(Uuid::from_u128(5), slot(5, 2));
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
            let TournamentFormatState::Arena(format) = state.format else {
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
            let TournamentFormatResponse::Arena(ArenaState { games, .. }) = &mut replacement else {
                unreachable!()
            };
            games.insert(GameId(String::from("third")), arena_game("third", 3));

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
