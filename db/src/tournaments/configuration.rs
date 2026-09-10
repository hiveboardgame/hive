use shared_types::{
    tournament::{
        elimination::{Config as EliminationConfig, Topology as EliminationTopology},
        round_robin::Config as RoundRobinConfig,
        swiss::Config as SwissConfig,
        Clock,
        Config,
        ConfigError,
        FormatConfig,
    },
    CorrespondenceClock,
    TimeMode,
};
use thiserror::Error;
use tournamint::{
    elimination::{BracketSetupError, EliminationDefinition, EliminationKind, EliminationStage},
    round_robin::{
        RoundRobinConfig as TournamintRoundRobinConfig,
        RoundRobinError,
        RoundRobinSchedule,
        RoundRobinStandingsPlan,
    },
    swiss::{SwissConfig as TournamintSwissConfig, SwissError, SwissFacts, SwissStandingsPlan},
    Color,
    PlayerId as TournamintPlayerId,
};

#[derive(Debug, Error)]
pub enum DefinitionError {
    #[error("{field} does not fit the Tournamint numeric type")]
    ValueOutOfRange { field: &'static str },
    #[error(transparent)]
    Configuration(#[from] ConfigError),
    #[error(transparent)]
    RoundRobin(#[from] RoundRobinError),
    #[error(transparent)]
    Swiss(#[from] SwissError),
    #[error(transparent)]
    Elimination(#[from] BracketSetupError),
    #[error("the elimination builder received the wrong topology")]
    WrongEliminationTopology,
    #[error("Swiss rounds do not resolve for a {participant_count}-player field")]
    SwissRoundsUnavailable { participant_count: usize },
    #[error("elimination stage {stage:?} is absent from the {participant_count}-player topology")]
    EliminationStageAbsent {
        stage: EliminationStage,
        participant_count: usize,
    },
}

pub fn validate_creation_configuration(
    configuration: &Config,
    participant_count: usize,
) -> Result<(), DefinitionError> {
    configuration.validate_for_creation()?;
    validate_game_clock_storage(&configuration.format)?;
    match &configuration.format {
        FormatConfig::RoundRobin(config) => {
            build_round_robin_definition(config, participant_count)?;
        }
        FormatConfig::Swiss(config) => {
            build_swiss_tournament(config, participant_count)?;
        }
        FormatConfig::Elimination(config) => {
            let definition = match config.topology {
                EliminationTopology::Single { .. } => {
                    build_single_elimination_bracket(config, participant_count)?
                }
                EliminationTopology::Double => {
                    build_double_elimination_bracket(config, participant_count)?
                }
            };
            for stage_override in &config.stage_overrides {
                if !definition
                    .nodes()
                    .iter()
                    .any(|node| node.stage == stage_override.stage)
                {
                    return Err(DefinitionError::EliminationStageAbsent {
                        stage: stage_override.stage,
                        participant_count,
                    });
                }
            }
        }
        FormatConfig::Arena(_) => {}
    }
    Ok(())
}

/// Checks that the canonical clock fits the scalar columns used by Hive
/// games before a tournament configuration is persisted.
pub(crate) fn game_time_parts(
    clock: Clock,
) -> Result<(TimeMode, Option<i32>, Option<i32>), DefinitionError> {
    let seconds = |value: u32, field| {
        i32::try_from(value).map_err(|_| DefinitionError::ValueOutOfRange { field })
    };
    match clock {
        Clock::Realtime(clock) => Ok((
            TimeMode::RealTime,
            Some(seconds(clock.base_seconds.get(), "clock base")?),
            Some(seconds(clock.increment_seconds, "clock increment")?),
        )),
        Clock::Correspondence(CorrespondenceClock::TotalTimeEach { seconds_each }) => Ok((
            TimeMode::Correspondence,
            Some(seconds(seconds_each.get(), "correspondence total time")?),
            None,
        )),
        Clock::Correspondence(CorrespondenceClock::DaysPerMove { seconds_per_move }) => Ok((
            TimeMode::Correspondence,
            None,
            Some(seconds(seconds_per_move.get(), "correspondence move time")?),
        )),
    }
}

fn validate_game_clock_storage(format: &FormatConfig) -> Result<(), DefinitionError> {
    match format {
        FormatConfig::RoundRobin(config) => {
            game_time_parts(config.clock)?;
        }
        FormatConfig::Swiss(config) => {
            game_time_parts(config.clock)?;
        }
        FormatConfig::Elimination(config) => {
            for phase in &config.default_plan.phases {
                game_time_parts(phase.clock)?;
            }
            for stage_override in &config.stage_overrides {
                for phase in &stage_override.plan.phases {
                    game_time_parts(phase.clock)?;
                }
            }
        }
        FormatConfig::Arena(config) => {
            game_time_parts(Clock::Realtime(config.game_clock))?;
        }
    }
    Ok(())
}

fn player_ids(participant_count: usize) -> Vec<TournamintPlayerId> {
    (0..participant_count)
        .map(TournamintPlayerId::new)
        .collect()
}

pub fn build_round_robin_definition(
    config: &RoundRobinConfig,
    participant_count: usize,
) -> Result<RoundRobinSchedule, DefinitionError> {
    let initial_order = player_ids(participant_count);
    let config = build_round_robin_config(config);
    Ok(tournamint::round_robin::schedule(&config, &initial_order)?)
}

pub fn build_swiss_tournament(
    config: &SwissConfig,
    participant_count: usize,
) -> Result<SwissFacts, DefinitionError> {
    let initial_order = player_ids(participant_count);
    let mut config = config.clone();
    config.rounds = config
        .rounds
        .resolve(participant_count)
        .ok_or(DefinitionError::SwissRoundsUnavailable { participant_count })?;
    let config = build_swiss_config(&config);
    let facts = SwissFacts {
        config,
        initial_ranking: initial_order,
        rounds: Vec::new(),
    };
    tournamint::swiss::project(&facts)?;
    Ok(facts)
}

pub fn build_single_elimination_bracket(
    config: &EliminationConfig,
    participant_count: usize,
) -> Result<EliminationDefinition, DefinitionError> {
    let EliminationTopology::Single { bronze } = config.topology else {
        return Err(DefinitionError::WrongEliminationTopology);
    };
    let initial_order = player_ids(participant_count);
    Ok(tournamint::elimination::topology(
        EliminationKind::Single { bronze },
        &initial_order,
    )?)
}

pub fn build_double_elimination_bracket(
    config: &EliminationConfig,
    participant_count: usize,
) -> Result<EliminationDefinition, DefinitionError> {
    if !matches!(config.topology, EliminationTopology::Double) {
        return Err(DefinitionError::WrongEliminationTopology);
    }
    let initial_order = player_ids(participant_count);
    Ok(tournamint::elimination::topology(
        EliminationKind::Double,
        &initial_order,
    )?)
}

pub fn build_round_robin_config(config: &RoundRobinConfig) -> TournamintRoundRobinConfig {
    TournamintRoundRobinConfig {
        primary_score: config.primary_score,
        match_point_system: config.match_point_system,
        repeats: config.repeats.get(),
        initial_color: Color::White,
        point_system: config.game_point_system,
        standings: RoundRobinStandingsPlan::new(config.standings.clone()),
    }
}

pub fn build_swiss_config(config: &SwissConfig) -> TournamintSwissConfig {
    TournamintSwissConfig {
        system: config.system,
        expected_rounds: config
            .rounds
            .resolved_rounds()
            .map_or(0, |rounds| rounds.get()),
        initial_color: Color::White,
        game_point_system: config.game_point_system,
        acceleration: config.acceleration,
        standings: SwissStandingsPlan::new(config.standings.clone()),
    }
}

#[cfg(test)]
#[path = "configuration/tests.rs"]
mod tests;
