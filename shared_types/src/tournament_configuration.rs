use crate::{Clock, RealtimeClock, TimeMode};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt::{Display, Formatter, Result as FmtResult},
    num::{NonZeroU16, NonZeroU32},
    str::FromStr,
};
use thiserror::Error;
pub use tournamint::{
    elimination::{EliminationKind as EliminationTopology, EliminationStage},
    round_robin::{
        KoyaOptions,
        RoundRobinCriterion,
        RoundRobinDirectEncounterOptions,
        RoundRobinPrimaryScore,
        RoundRobinProgressiveOptions,
        RoundRobinSonnebornBergerOptions,
    },
    series::{
        ClinchPolicy,
        EntrantSide,
        SeriesPhase as NativeSeriesPhase,
        SeriesPlan as NativeSeriesPlan,
        SetLimit,
    },
    standings::{
        DirectEncounterForfeitPolicy,
        DirectEncounterOptions as SwissDirectEncounterOptions,
        ProgressiveScoreOptions as SwissProgressiveOptions,
        RepeatedEncounterPolicy,
        SonnebornBergerOptions as SwissSonnebornBergerOptions,
        TournamentPairingNumberOrder,
    },
    swiss::{
        AccelerationPolicy as SwissAcceleration,
        BuchholzOptions as SwissBuchholzOptions,
        DoubleSwissByePoints,
        DoubleSwissConfig,
        DoubleSwissPrimaryScore,
        DutchConfig,
        FlatAcceleration,
        MatchPointSystem,
        ScoreBasis as SwissScoreBasis,
        SingleGameSwissByePoints,
        SwissCriterion as SwissStandingsCriterion,
        SwissSystemConfig as SwissSystem,
    },
    PointSystem,
    Score,
};

pub const MAX_ELIMINATION_PHASES: usize = 8;
pub const MAX_ELIMINATION_COLOR_ORDER: usize = 32;
pub const MAX_ELIMINATION_GAMES_PER_SET: u16 = 32;
pub const MAX_ELIMINATION_FINITE_SETS: u16 = 32;
pub const MAX_ELIMINATION_STAGE_OVERRIDES: usize = 16;
pub const MAX_ROUND_ROBIN_REPEATS: u32 = 6;
pub const MAX_SWISS_ROUNDS: u32 = 64;
pub const MAX_ROUND_ROBIN_SEATS: i32 = 16;
pub const MAX_SWISS_SEATS: i32 = 160;
pub const MAX_ELIMINATION_SEATS: i32 = 64;
pub const MIN_SWISS_START_SEATS: i32 = 5;
pub const MIN_SWISS_ROUNDS: u32 = 3;
pub const MIN_SWISS_EXTRA_ROUNDS: i32 = -2;
pub const MAX_SWISS_EXTRA_ROUNDS: i32 = 3;

pub const ARENA_MIN_DURATION_SECONDS: u32 = 60 * 60;
pub const ARENA_MAX_DURATION_SECONDS: u32 = 5 * 60 * 60;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    RoundRobin,
    Swiss,
    DoubleSwiss,
    SingleElimination,
    DoubleElimination,
    Arena,
}

impl Format {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RoundRobin => "round_robin",
            Self::Swiss => "swiss",
            Self::DoubleSwiss => "double_swiss",
            Self::SingleElimination => "single_elimination",
            Self::DoubleElimination => "double_elimination",
            Self::Arena => "arena",
        }
    }

    pub const fn family_tag(self) -> &'static str {
        match self {
            Self::RoundRobin => "round_robin",
            Self::Swiss | Self::DoubleSwiss => "swiss",
            Self::SingleElimination | Self::DoubleElimination => "elimination",
            Self::Arena => "arena",
        }
    }
}

impl Display for Format {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum FormatParseError {
    #[error("{found:?} is not a valid tournament format")]
    Invalid { found: String },
}

impl FromStr for Format {
    type Err = FormatParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "round_robin" => Ok(Self::RoundRobin),
            "swiss" => Ok(Self::Swiss),
            "double_swiss" => Ok(Self::DoubleSwiss),
            "single_elimination" => Ok(Self::SingleElimination),
            "double_elimination" => Ok(Self::DoubleElimination),
            "arena" => Ok(Self::Arena),
            found => Err(FormatParseError::Invalid {
                found: found.to_string(),
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BotAdmission {
    HumansOnly,
    HumansAndBots,
    BotsOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairingIntent {
    Enabled,
    Paused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleasePolicy {
    FullyUnlocked,
    SequentialPerMatchup,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoundRobinConfig {
    pub primary_score: RoundRobinPrimaryScore,
    pub match_point_system: MatchPointSystem,
    pub repeats: NonZeroU32,
    pub release_policy: ReleasePolicy,
    pub clock: Clock,
    pub game_point_system: PointSystem,
    pub standings: Vec<RoundRobinCriterion>,
}

impl RoundRobinConfig {
    pub const fn has_matches(&self) -> bool {
        self.repeats.get().is_multiple_of(2)
    }

    pub fn standard(repeats: NonZeroU32, clock: Clock) -> Self {
        Self {
            repeats,
            primary_score: RoundRobinPrimaryScore::GamePoints,
            match_point_system: MatchPointSystem::STANDARD,
            release_policy: ReleasePolicy::FullyUnlocked,
            clock,
            game_point_system: PointSystem::STANDARD,
            standings: vec![RoundRobinCriterion::PrimaryScore],
        }
    }
}

pub fn round_robin_creation_tiebreakers(
    repeats: NonZeroU32,
    primary_score: RoundRobinPrimaryScore,
) -> Vec<RoundRobinCriterion> {
    let mut criteria = vec![
        RoundRobinCriterion::DirectEncounter(RoundRobinDirectEncounterOptions {
            forfeits: DirectEncounterForfeitPolicy::Exclude,
            repeated_encounters: RepeatedEncounterPolicy::Average,
        }),
        RoundRobinCriterion::SonnebornBerger(RoundRobinSonnebornBergerOptions { cut_lowest: 0 }),
        RoundRobinCriterion::Koya(KoyaOptions {
            threshold_numerator: 1,
            threshold_denominator: NonZeroU32::new(2).expect("two is nonzero"),
            include_adjudicated: false,
        }),
        RoundRobinCriterion::NumberOfWins,
        RoundRobinCriterion::GamesWonWithBlack,
        RoundRobinCriterion::TournamentPairingNumber(TournamentPairingNumberOrder::Ascending),
        RoundRobinCriterion::TournamentPairingNumber(TournamentPairingNumberOrder::Descending),
    ];
    if repeats.get().is_multiple_of(2) {
        criteria.push(RoundRobinCriterion::MatchesWon);
        criteria.push(match primary_score {
            RoundRobinPrimaryScore::GamePoints => RoundRobinCriterion::MatchPoints,
            RoundRobinPrimaryScore::MatchPoints => RoundRobinCriterion::GamePoints,
        });
    }
    criteria
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SwissRoundConfiguration {
    Automatic {
        extra_rounds: i32,
    },
    Resolved {
        rounds: NonZeroU32,
        requested_extra: Option<i32>,
    },
}

impl SwissRoundConfiguration {
    pub const fn automatic(extra_rounds: i32) -> Self {
        Self::Automatic { extra_rounds }
    }

    pub const fn resolved(rounds: NonZeroU32, requested_extra: Option<i32>) -> Self {
        Self::Resolved {
            rounds,
            requested_extra,
        }
    }

    pub const fn resolved_rounds(self) -> Option<NonZeroU32> {
        match self {
            Self::Automatic { .. } => None,
            Self::Resolved { rounds, .. } => Some(rounds),
        }
    }

    pub fn resolve(self, participant_count: usize) -> Option<Self> {
        let Self::Automatic { extra_rounds } = self else {
            return Some(self);
        };
        let participant_count = u32::try_from(participant_count).ok()?;
        let distinct_opponents = participant_count.checked_sub(1)?;
        let ceil_log_two = u32::BITS - participant_count.saturating_sub(1).leading_zeros();
        let base = ceil_log_two.max(MIN_SWISS_ROUNDS);
        let rounds = base
            .saturating_add_signed(extra_rounds)
            .max(MIN_SWISS_ROUNDS)
            .min(distinct_opponents);
        let rounds = NonZeroU32::new(rounds)?;
        Some(Self::Resolved {
            rounds,
            requested_extra: Some(extra_rounds),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwissConfig {
    pub system: SwissSystem,
    pub double_swiss_release_policy: ReleasePolicy,
    pub rounds: SwissRoundConfiguration,
    pub clock: Clock,
    pub game_point_system: PointSystem,
    pub acceleration: SwissAcceleration,
    pub standings: Vec<SwissStandingsCriterion>,
}

impl SwissConfig {
    pub fn swiss(rounds: NonZeroU32, clock: Clock) -> Self {
        Self::standard(
            dutch_swiss_system(),
            SwissRoundConfiguration::resolved(rounds, None),
            clock,
        )
    }

    pub fn automatic_swiss(extra_rounds: i32, clock: Clock) -> Self {
        Self::standard(
            dutch_swiss_system(),
            SwissRoundConfiguration::automatic(extra_rounds),
            clock,
        )
    }

    pub fn double_swiss(rounds: NonZeroU32, clock: Clock) -> Self {
        Self::double_swiss_with_primary_score(rounds, clock, DoubleSwissPrimaryScore::GamePoints)
    }

    pub fn double_swiss_with_primary_score(
        rounds: NonZeroU32,
        clock: Clock,
        primary_score: DoubleSwissPrimaryScore,
    ) -> Self {
        Self::standard(
            double_swiss_system(primary_score),
            SwissRoundConfiguration::resolved(rounds, None),
            clock,
        )
    }

    pub fn automatic_double_swiss(
        extra_rounds: i32,
        clock: Clock,
        primary_score: DoubleSwissPrimaryScore,
    ) -> Self {
        Self::standard(
            double_swiss_system(primary_score),
            SwissRoundConfiguration::automatic(extra_rounds),
            clock,
        )
    }

    fn standard(system: SwissSystem, rounds: SwissRoundConfiguration, clock: Clock) -> Self {
        Self {
            system,
            double_swiss_release_policy: ReleasePolicy::FullyUnlocked,
            rounds,
            clock,
            game_point_system: PointSystem::STANDARD,
            acceleration: SwissAcceleration::None,
            standings: vec![
                SwissStandingsCriterion::PrimaryScore,
                SwissStandingsCriterion::Buchholz(SwissBuchholzOptions::default()),
                SwissStandingsCriterion::SonnebornBerger(SwissSonnebornBergerOptions::default()),
                SwissStandingsCriterion::Progressive(SwissProgressiveOptions::default()),
                SwissStandingsCriterion::DirectEncounter(SwissDirectEncounterOptions::default()),
                SwissStandingsCriterion::NumberOfWins,
            ],
        }
    }
}

fn dutch_swiss_system() -> SwissSystem {
    let points = PointSystem::STANDARD;
    SwissSystem::Dutch(DutchConfig {
        bye_points: SingleGameSwissByePoints::game_win(points),
    })
}

fn double_swiss_system(primary_score: DoubleSwissPrimaryScore) -> SwissSystem {
    let points = PointSystem::STANDARD;
    let match_points = MatchPointSystem::STANDARD;
    SwissSystem::DoubleSwiss(DoubleSwissConfig {
        primary_score,
        match_point_system: match_points,
        bye_points: DoubleSwissByePoints {
            game_points: Score::new(points.win.value() + points.draw.value()),
            match_points: match_points.win,
            counts_as_win: true,
        },
    })
}

pub fn swiss_creation_tiebreakers(
    double_swiss_primary: Option<DoubleSwissPrimaryScore>,
) -> Vec<SwissStandingsCriterion> {
    let mut criteria = vec![
        SwissStandingsCriterion::DirectEncounter(SwissDirectEncounterOptions {
            score_basis: SwissScoreBasis::Primary,
            forfeits: DirectEncounterForfeitPolicy::Exclude,
            repeated_encounters: RepeatedEncounterPolicy::Average,
        }),
        SwissStandingsCriterion::SonnebornBerger(SwissSonnebornBergerOptions {
            score_basis: SwissScoreBasis::Primary,
            cut_lowest: 0,
            ..SwissSonnebornBergerOptions::default()
        }),
        SwissStandingsCriterion::Buchholz(SwissBuchholzOptions {
            cut_lowest: 0,
            cut_highest: 0,
            ..SwissBuchholzOptions::default()
        }),
        SwissStandingsCriterion::Buchholz(SwissBuchholzOptions {
            cut_lowest: 1,
            cut_highest: 0,
            ..SwissBuchholzOptions::default()
        }),
        SwissStandingsCriterion::Buchholz(SwissBuchholzOptions {
            cut_lowest: 2,
            cut_highest: 0,
            ..SwissBuchholzOptions::default()
        }),
        SwissStandingsCriterion::Buchholz(SwissBuchholzOptions {
            cut_lowest: 1,
            cut_highest: 1,
            ..SwissBuchholzOptions::default()
        }),
        SwissStandingsCriterion::Progressive(SwissProgressiveOptions {
            score_basis: SwissScoreBasis::Primary,
            cut_first_round: false,
        }),
        SwissStandingsCriterion::NumberOfWins,
        SwissStandingsCriterion::GamesWonWithBlack,
        SwissStandingsCriterion::TournamentPairingNumber(TournamentPairingNumberOrder::Ascending),
        SwissStandingsCriterion::TournamentPairingNumber(TournamentPairingNumberOrder::Descending),
    ];
    if let Some(primary) = double_swiss_primary {
        criteria.push(match primary {
            DoubleSwissPrimaryScore::GamePoints => SwissStandingsCriterion::MatchPoints,
            DoubleSwissPrimaryScore::MatchPoints => SwissStandingsCriterion::GamePoints,
        });
    }
    criteria
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EliminationSeriesPhase {
    pub games_per_set: u16,
    pub color_order: Vec<EntrantSide>,
    pub set_limit: SetLimit,
    pub clinch: ClinchPolicy,
    pub clock: Clock,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EliminationSeriesPlan {
    pub phases: Vec<EliminationSeriesPhase>,
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum PlanError {
    /// The host requested opening metadata from a plan with no phase.
    #[error("elimination series plan has no phase")]
    EmptyPhases,
    /// A persisted phase cannot be represented by the native nonzero game count.
    #[error("elimination phase {phase_index} has zero games per set")]
    ZeroGamesPerSet { phase_index: usize },
}

impl EliminationSeriesPlan {
    pub fn balanced(clock: Clock) -> Self {
        Self {
            phases: vec![EliminationSeriesPhase {
                games_per_set: 2,
                color_order: vec![EntrantSide::First, EntrantSide::Second],
                set_limit: SetLimit::UntilDecisive,
                clinch: ClinchPolicy::PlayAll,
                clock,
            }],
        }
    }

    pub fn to_native(&self) -> Result<NativeSeriesPlan, PlanError> {
        let phases = self
            .phases
            .iter()
            .enumerate()
            .map(|(phase_index, phase)| {
                Ok(NativeSeriesPhase {
                    games_per_set: NonZeroU16::new(phase.games_per_set)
                        .ok_or(PlanError::ZeroGamesPerSet { phase_index })?,
                    color_order: phase.color_order.clone(),
                    set_limit: phase.set_limit,
                    clinch: phase.clinch,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(NativeSeriesPlan { phases })
    }

    /// Converts the plan while carrying its opening clock across the boundary.
    pub fn to_native_with_initial_clock(&self) -> Result<(NativeSeriesPlan, Clock), PlanError> {
        let initial_clock = self.phases.first().ok_or(PlanError::EmptyPhases)?.clock;
        Ok((self.to_native()?, initial_clock))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EliminationStageOverride {
    pub stage: EliminationStage,
    pub plan: EliminationSeriesPlan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EliminationConfig {
    pub topology: EliminationTopology,
    pub default_plan: EliminationSeriesPlan,
    pub stage_overrides: Vec<EliminationStageOverride>,
}

impl EliminationConfig {
    pub fn time_mode(&self) -> Option<TimeMode> {
        self.default_plan
            .phases
            .first()
            .map(|phase| phase.clock.mode())
    }

    pub fn validate_for_creation(&self) -> Result<(), ConfigError> {
        validate_elimination(self)
    }

    pub fn effective_plan(&self, stage: EliminationStage) -> &EliminationSeriesPlan {
        self.stage_overrides
            .iter()
            .find(|override_| override_.stage == stage)
            .map_or(&self.default_plan, |override_| &override_.plan)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArenaConfig {
    pub duration_seconds: NonZeroU32,
    pub game_clock: RealtimeClock,
}

impl ArenaConfig {
    pub const fn new(duration_seconds: NonZeroU32, game_clock: RealtimeClock) -> Self {
        Self {
            duration_seconds,
            game_clock,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "format",
    content = "configuration",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum FormatConfig {
    RoundRobin(RoundRobinConfig),
    Swiss(SwissConfig),
    Elimination(EliminationConfig),
    Arena(ArenaConfig),
}

impl FormatConfig {
    pub const fn kind(&self) -> Format {
        match self {
            Self::RoundRobin(_) => Format::RoundRobin,
            Self::Swiss(config) => swiss_format(config.system),
            Self::Elimination(config) => elimination_format(config.topology),
            Self::Arena(_) => Format::Arena,
        }
    }
}

const fn swiss_format(system: SwissSystem) -> Format {
    match system {
        SwissSystem::Dutch(_) | SwissSystem::Burstein(_) => Format::Swiss,
        SwissSystem::DoubleSwiss(_) => Format::DoubleSwiss,
    }
}

const fn elimination_format(topology: EliminationTopology) -> Format {
    match topology {
        EliminationTopology::Single { .. } => Format::SingleElimination,
        EliminationTopology::Double => Format::DoubleElimination,
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub bot_admission: BotAdmission,
    pub format: FormatConfig,
}

impl Config {
    pub const fn format(&self) -> Format {
        self.format.kind()
    }

    pub fn validate_for_creation(&self) -> Result<(), ConfigError> {
        validate_configuration(self)
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ConfigError {
    #[error("{field} exceeds Hive's supported maximum of {maximum} (found {found})")]
    NumericLimit {
        field: &'static str,
        maximum: u32,
        found: u32,
    },
    #[error("Arena duration must be between one and five hours inclusive")]
    InvalidArenaDuration,
    #[error("Swiss round adjustment must be between -2 and +3 (found {found})")]
    InvalidSwissRoundAdjustment { found: i32 },
    #[error("Swiss system is not supported by Hive")]
    UnsupportedSwissSystem,
    #[error("Progressive is only supported for Swiss tournaments")]
    RoundRobinProgressive,
    #[error("select only one seed tiebreak direction")]
    MultipleSeedTiebreaks,
    #[error(
        "round-robin match points and match wins require an even number of games per opponent"
    )]
    RoundRobinMatchScoringRequiresEvenRepeats,
    #[error("a score tiebreak must be the alternate game or match score")]
    InvalidScoreTiebreak,
    #[error("elimination stage override {stage:?} occurs more than once")]
    DuplicateEliminationStageOverride { stage: EliminationStage },
    #[error("elimination stage {stage:?} is incompatible with topology {topology:?}")]
    IncompatibleEliminationStage {
        topology: EliminationTopology,
        stage: EliminationStage,
    },
    #[error("elimination plan {location} is invalid: {reason}")]
    InvalidEliminationPlan { location: String, reason: String },
}

fn validate_configuration(config: &Config) -> Result<(), ConfigError> {
    match &config.format {
        FormatConfig::RoundRobin(round_robin) => validate_round_robin(round_robin),
        FormatConfig::Swiss(swiss) => validate_swiss(swiss),
        FormatConfig::Arena(arena) => {
            let duration = arena.duration_seconds.get();
            if !(ARENA_MIN_DURATION_SECONDS..=ARENA_MAX_DURATION_SECONDS).contains(&duration) {
                return Err(ConfigError::InvalidArenaDuration);
            }
            Ok(())
        }
        FormatConfig::Elimination(elimination) => elimination.validate_for_creation(),
    }
}

fn numeric_limit(field: &'static str, found: u32, maximum: u32) -> Result<(), ConfigError> {
    if found > maximum {
        Err(ConfigError::NumericLimit {
            field,
            maximum,
            found,
        })
    } else {
        Ok(())
    }
}

fn validate_round_robin(config: &RoundRobinConfig) -> Result<(), ConfigError> {
    if !config.has_matches()
        && (config.primary_score == RoundRobinPrimaryScore::MatchPoints
            || config.standings.iter().any(|criterion| {
                matches!(
                    criterion,
                    RoundRobinCriterion::MatchPoints | RoundRobinCriterion::MatchesWon
                )
            }))
    {
        return Err(ConfigError::RoundRobinMatchScoringRequiresEvenRepeats);
    }
    for criterion in &config.standings {
        let valid = match criterion {
            RoundRobinCriterion::GamePoints => {
                config.primary_score == RoundRobinPrimaryScore::MatchPoints
            }
            RoundRobinCriterion::MatchPoints => {
                config.primary_score == RoundRobinPrimaryScore::GamePoints
            }
            _ => true,
        };
        if !valid {
            return Err(ConfigError::InvalidScoreTiebreak);
        }
    }
    if config
        .standings
        .iter()
        .any(|criterion| matches!(criterion, RoundRobinCriterion::Progressive(_)))
    {
        return Err(ConfigError::RoundRobinProgressive);
    }
    validate_seed_tiebreaks(
        config
            .standings
            .iter()
            .filter(|criterion| {
                matches!(criterion, RoundRobinCriterion::TournamentPairingNumber(_))
            })
            .count(),
    )?;
    numeric_limit(
        "round-robin repeats",
        config.repeats.get(),
        MAX_ROUND_ROBIN_REPEATS,
    )?;
    Ok(())
}

fn validate_seed_tiebreaks(count: usize) -> Result<(), ConfigError> {
    if count > 1 {
        Err(ConfigError::MultipleSeedTiebreaks)
    } else {
        Ok(())
    }
}

fn validate_swiss_round_adjustment(found: i32) -> Result<(), ConfigError> {
    if (MIN_SWISS_EXTRA_ROUNDS..=MAX_SWISS_EXTRA_ROUNDS).contains(&found) {
        Ok(())
    } else {
        Err(ConfigError::InvalidSwissRoundAdjustment { found })
    }
}

fn validate_swiss(config: &SwissConfig) -> Result<(), ConfigError> {
    validate_seed_tiebreaks(
        config
            .standings
            .iter()
            .filter(|criterion| {
                matches!(
                    criterion,
                    SwissStandingsCriterion::TournamentPairingNumber(_)
                )
            })
            .count(),
    )?;
    for criterion in &config.standings {
        let valid = match criterion {
            SwissStandingsCriterion::GamePoints => {
                matches!(config.system, SwissSystem::DoubleSwiss(system)
                if system.primary_score == DoubleSwissPrimaryScore::MatchPoints)
            }
            SwissStandingsCriterion::MatchPoints => {
                matches!(config.system, SwissSystem::DoubleSwiss(system)
                if system.primary_score == DoubleSwissPrimaryScore::GamePoints)
            }
            _ => true,
        };
        if !valid {
            return Err(ConfigError::InvalidScoreTiebreak);
        }
    }
    match config.system {
        SwissSystem::Dutch(_) | SwissSystem::DoubleSwiss(_) => {}
        SwissSystem::Burstein(_) => return Err(ConfigError::UnsupportedSwissSystem),
    }
    match config.rounds {
        SwissRoundConfiguration::Automatic { extra_rounds } => {
            validate_swiss_round_adjustment(extra_rounds)?
        }
        SwissRoundConfiguration::Resolved {
            rounds,
            requested_extra,
        } => {
            numeric_limit("Swiss rounds", rounds.get(), MAX_SWISS_ROUNDS)?;
            if let Some(extra_rounds) = requested_extra {
                validate_swiss_round_adjustment(extra_rounds)?;
            }
        }
    }
    if let SwissAcceleration::Flat(options) = config.acceleration {
        numeric_limit("acceleration rounds", options.rounds, MAX_SWISS_ROUNDS)?;
    }
    Ok(())
}

fn validate_elimination(config: &EliminationConfig) -> Result<(), ConfigError> {
    numeric_limit(
        "elimination stage overrides",
        u32::try_from(config.stage_overrides.len()).unwrap_or(u32::MAX),
        u32::try_from(MAX_ELIMINATION_STAGE_OVERRIDES).expect("supported limit fits u32"),
    )?;
    let Some(time_mode) = config.time_mode() else {
        return Err(ConfigError::InvalidEliminationPlan {
            location: String::from("default plan"),
            reason: String::from("the phase list is empty"),
        });
    };
    validate_series_plan(&config.default_plan, time_mode, "default plan")?;

    let mut stages = HashSet::new();
    for override_ in &config.stage_overrides {
        validate_stage(config.topology, override_.stage)?;
        if !stages.insert(override_.stage) {
            return Err(ConfigError::DuplicateEliminationStageOverride {
                stage: override_.stage,
            });
        }
        validate_series_plan(
            &override_.plan,
            time_mode,
            &format!("stage override {:?}", override_.stage),
        )?;
    }

    Ok(())
}

#[cfg(test)]
#[path = "tournament_configuration/tests.rs"]
mod tests;

fn validate_stage(
    topology: EliminationTopology,
    stage: EliminationStage,
) -> Result<(), ConfigError> {
    let compatible = topology.supports_stage(stage);
    if compatible {
        Ok(())
    } else {
        Err(ConfigError::IncompatibleEliminationStage { topology, stage })
    }
}

fn validate_series_plan(
    plan: &EliminationSeriesPlan,
    time_mode: TimeMode,
    location: &str,
) -> Result<(), ConfigError> {
    let invalid = |reason: &str| ConfigError::InvalidEliminationPlan {
        location: location.to_owned(),
        reason: reason.to_owned(),
    };

    if plan.phases.len() > MAX_ELIMINATION_PHASES {
        return Err(invalid("the phase list exceeds the safety limit"));
    }

    for phase in &plan.phases {
        if phase.color_order.len() > MAX_ELIMINATION_COLOR_ORDER {
            return Err(invalid("a phase color order exceeds the safety limit"));
        }
        if phase.games_per_set > MAX_ELIMINATION_GAMES_PER_SET {
            return Err(invalid("games per set exceeds the safety limit"));
        }
        if phase.clock.mode() != time_mode {
            return Err(invalid("a phase time control changes tournament time mode"));
        }
        if let SetLimit::AtMost(limit) = phase.set_limit {
            if limit.get() > MAX_ELIMINATION_FINITE_SETS {
                return Err(invalid("finite set count exceeds the safety limit"));
            }
        }
    }

    let native = plan
        .to_native()
        .map_err(|error| invalid(&error.to_string()))?;
    native
        .validate()
        .map_err(|error| invalid(&error.to_string()))?;

    Ok(())
}
