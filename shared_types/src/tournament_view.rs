use crate::{
    tournament::{
        arena::Config as ArenaConfig,
        elimination::{
            Config as EliminationConfig,
            Resolution as EliminationNodeResolution,
            Source as EliminationSource,
            Stage as EliminationStage,
        },
        round_robin::Config as RoundRobinConfig,
        swiss::Config as SwissConfig,
        Clock,
        EliminationNodeId,
        Format,
        FormatConfig,
        GameOutcome,
        Resolution,
        Score,
        SlotKey,
    },
    GameId,
    GameSpeed,
    GameStart,
    SlotAdminAction,
    SwissProgress,
    TournamentGameResult,
};
use chrono::{DateTime, Utc};
use hive_lib::GameStatus;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tournamint::{round_robin::RoundRobinCompletedMatchProjection, MatchScore};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum TournamentFormatResponse {
    Arena(ArenaState),
    RoundRobin(RoundRobinState),
    Swiss(SwissState),
    Elimination(EliminationState),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "reactive", derive(reactive_stores::Store))]
pub struct ArenaState {
    pub configuration: ArenaConfig,
    #[cfg_attr(feature = "reactive", store(key: GameId = |(game_id, _)| game_id.clone()))]
    pub games: HashMap<GameId, ArenaGameResponse>,
    #[cfg_attr(feature = "reactive", store(key: Uuid = |(player_id, _)| *player_id))]
    pub player_stats: HashMap<Uuid, ArenaPlayerStatsResponse>,
    pub featured_game_id: Option<GameId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "reactive", derive(reactive_stores::Store))]
pub struct RoundRobinState {
    pub configuration: RoundRobinConfig,
    pub rounds: Vec<RoundRobinRoundResponse>,
    pub matches: Vec<RoundRobinMatchResponse>,
    pub withdrawable_entrants: HashSet<Uuid>,
    pub closeout_eligible_slots: u32,
    #[cfg_attr(feature = "reactive", store(key: Uuid = |(slot_id, _)| *slot_id))]
    pub slots: HashMap<Uuid, SlotResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct RoundRobinRoundResponse {
    pub round_index: usize,
    pub pass_index: usize,
    pub resting: Option<Uuid>,
    pub slots: Vec<RoundRobinSlotResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct RoundRobinSlotResponse {
    pub board_index: usize,
    pub slot_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "reactive", derive(reactive_stores::Store))]
pub struct SwissState {
    pub configuration: SwissConfig,
    pub rounds: Vec<SwissRoundResponse>,
    pub progress: SwissProgress,
    pub withdrawable_entrants: HashSet<Uuid>,
    pub closeout_eligible_slots: u32,
    #[cfg_attr(feature = "reactive", store(key: Uuid = |(slot_id, _)| *slot_id))]
    pub slots: HashMap<Uuid, SlotResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SwissRoundResponse {
    pub round_index: u32,
    pub encounters: Vec<SwissEncounterResponse>,
    pub byes: Vec<SwissByeResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SwissEncounterResponse {
    pub pairing_index: u32,
    pub participants: [Uuid; 2],
    pub pre_round_primary_scores: [Score; 2],
    pub rating_snapshots: [Option<u32>; 2],
    pub slot_ids: Vec<Uuid>,
    pub completion: Option<SwissMatchCompletionResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "reactive", derive(reactive_stores::Store))]
pub struct EliminationState {
    pub configuration: EliminationConfig,
    pub nodes: Vec<EliminationNodeResponse>,
    pub complete: bool,
    pub player_results: Vec<EliminationPlayerResultResponse>,
    pub withdrawable_entrants: HashSet<Uuid>,
    #[cfg_attr(feature = "reactive", store(key: Uuid = |(slot_id, _)| *slot_id))]
    pub slots: HashMap<Uuid, SlotResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct EliminationNodeResponse {
    pub node_id: EliminationNodeId,
    pub wave_index: usize,
    pub stage: EliminationStage,
    pub stage_ordinal: usize,
    pub sources: [EliminationSource; 2],
    pub entrants: [Option<Uuid>; 2],
    pub possible_entrants: [Vec<Uuid>; 2],
    pub state: EliminationNodeStateResponse,
    pub series: Option<EliminationSeriesResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct EliminationSeriesResponse {
    pub score: [u64; 2],
    pub sets: Vec<EliminationSeriesSetResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct EliminationSeriesSetResponse {
    pub phase_index: usize,
    pub score: [u64; 2],
    pub slots: Vec<EliminationSeriesSlotResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct EliminationSeriesSlotResponse {
    pub slot_id: Uuid,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct CompactTournamentGameResponse {
    pub game_id: GameId,
    pub participants: [Uuid; 2],
    pub status: GameStatus,
    pub start: GameStart,
    pub finished: bool,
    pub ratings: [Option<u32>; 2],
    pub berserked: [bool; 2],
    pub speed: GameSpeed,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct SlotResponse {
    pub id: Uuid,
    pub key: SlotKey,
    pub participants: [Uuid; 2],
    pub clock: Clock,
    pub resolution: Option<Resolution>,
    pub outcome: Option<GameOutcome>,
    pub awarded_game_points: Option<[Score; 2]>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub game: Option<CompactTournamentGameResponse>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub deadline_at: Option<DateTime<Utc>>,
    /// Unresolved predecessor that delays Game creation, not scheduling.
    pub waits_for: Option<Uuid>,
    pub available_admin_actions: HashSet<SlotAdminAction>,
    /// Concrete adjudicated results coherent with the current tournament facts.
    pub available_results: Vec<TournamentGameResult>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ArenaGameResponse {
    pub ordinal: i64,
    pub game: CompactTournamentGameResponse,
    pub outcome: Option<GameOutcome>,
    pub awarded_points: Option<[Score; 2]>,
    pub doubled: Option<[bool; 2]>,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ArenaPlayerStatsResponse {
    pub player: Uuid,
    pub performance_rating: Option<i32>,
    pub average_opponent_rating: Option<u32>,
    pub performance_games: u32,
    pub arena_rating: Option<i32>,
    pub games_scored: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub on_fire: bool,
    pub berserks: u32,
    pub paused: bool,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct PlayerStatsResponse {
    pub player: Uuid,
    pub performance_rating: Option<i32>,
    pub average_opponent_rating: Option<u32>,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct RoundRobinMatchResponse {
    pub participants: [Uuid; 2],
    pub completion: Option<RoundRobinCompletedMatchProjection>,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct SwissMatchCompletionResponse {
    pub aggregate: [MatchScore; 2],
    pub game_points: [Score; 2],
    pub match_points: [Score; 2],
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct SwissByeResponse {
    pub player: Uuid,
    pub pre_round_primary_score: Score,
    pub rating_snapshot: Option<u32>,
    pub game_points: Score,
    pub match_points: Score,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum EliminationNodeStateResponse {
    Planned { conditional: bool },
    Active,
    Resolved(EliminationNodeResolution),
    Skipped,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct EliminationPlayerResultResponse {
    pub player: Uuid,
    pub in_contention: bool,
    pub placement: Option<u32>,
    pub exit_stage: Option<EliminationStage>,
}

impl TournamentFormatResponse {
    pub fn slots(&self) -> Option<&HashMap<Uuid, SlotResponse>> {
        match self {
            Self::Arena(_) => None,
            Self::RoundRobin(state) => Some(&state.slots),
            Self::Swiss(state) => Some(&state.slots),
            Self::Elimination(state) => Some(&state.slots),
        }
    }

    pub fn kind(&self) -> Format {
        match self {
            Self::Arena(_) => Format::Arena,
            Self::RoundRobin(_) => Format::RoundRobin,
            Self::Swiss(state) => FormatConfig::Swiss(state.configuration.clone()).kind(),
            Self::Elimination(state) => {
                FormatConfig::Elimination(state.configuration.clone()).kind()
            }
        }
    }
}

impl SlotResponse {
    pub const fn white(&self) -> Uuid {
        self.participants[0]
    }

    pub const fn black(&self) -> Uuid {
        self.participants[1]
    }

    pub fn game_id(&self) -> Option<&GameId> {
        self.game.as_ref().map(|game| &game.game_id)
    }

    pub fn outcome(&self) -> Option<GameOutcome> {
        self.outcome
    }
}

#[cfg(feature = "reactive")]
mod reactive {
    use super::{ArenaGameResponse, ArenaPlayerStatsResponse, SlotResponse};
    use reactive_stores::{KeyMap, PatchField, StorePath};
    macro_rules! patch_response_leaf {
    ($($type:ty),+ $(,)?) => {
        $(
            impl PatchField for $type {
                fn patch_field(
                    &mut self,
                    new: Self,
                    path: &StorePath,
                    notify: &mut dyn FnMut(&StorePath),
                    _keys: Option<&KeyMap>,
                ) {
                    if *self != new {
                        *self = new;
                        notify(path);
                    }
                }
            }
        )+
    };
}

    patch_response_leaf!(SlotResponse, ArenaGameResponse, ArenaPlayerStatsResponse);
}
