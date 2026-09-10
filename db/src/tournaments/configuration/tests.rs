use super::*;
use shared_types::tournament::{
    arena::{Config as ArenaConfig, MIN_DURATION_SECONDS},
    elimination::{
        Config as EliminationConfig,
        SeriesPlan as EliminationSeriesPlan,
        Stage as EliminationStage,
        StageOverride as EliminationStageOverride,
        Topology as EliminationTopology,
    },
    swiss::Criterion as SwissStandingsCriterion,
    BotAdmission,
    Clock,
    Config,
    FormatConfig,
    RealtimeClock,
};
use std::num::NonZeroU32;

fn realtime() -> Clock {
    Clock::Realtime(RealtimeClock {
        base_seconds: NonZeroU32::new(300).unwrap(),
        increment_seconds: 3,
    })
}

fn oversized_realtime() -> Clock {
    Clock::Realtime(RealtimeClock {
        base_seconds: NonZeroU32::new(i32::MAX as u32 + 1).unwrap(),
        increment_seconds: 3,
    })
}

#[test]
fn game_time_parts_reject_sql_numeric_overflow() {
    let oversized = NonZeroU32::new(i32::MAX as u32 + 1).unwrap();
    for clock in [
        oversized_realtime(),
        Clock::Realtime(RealtimeClock {
            base_seconds: NonZeroU32::new(300).unwrap(),
            increment_seconds: u32::MAX,
        }),
        Clock::Correspondence(CorrespondenceClock::TotalTimeEach {
            seconds_each: oversized,
        }),
        Clock::Correspondence(CorrespondenceClock::DaysPerMove {
            seconds_per_move: oversized,
        }),
    ] {
        assert!(matches!(
            game_time_parts(clock),
            Err(DefinitionError::ValueOutOfRange { .. })
        ));
    }
}

#[test]
fn creation_checks_every_format_clock_before_persisting() {
    let configurable = |format| Config {
        bot_admission: BotAdmission::HumansAndBots,
        format,
    };
    let elimination = |default_clock, stage_overrides| {
        configurable(FormatConfig::Elimination(EliminationConfig {
            topology: EliminationTopology::Single { bronze: false },
            default_plan: EliminationSeriesPlan::balanced(default_clock),
            stage_overrides,
        }))
    };
    let configurations = [
        configurable(FormatConfig::RoundRobin(RoundRobinConfig::standard(
            NonZeroU32::new(1).unwrap(),
            oversized_realtime(),
        ))),
        configurable(FormatConfig::Swiss(SwissConfig::swiss(
            NonZeroU32::new(3).unwrap(),
            oversized_realtime(),
        ))),
        elimination(oversized_realtime(), Vec::new()),
        elimination(
            realtime(),
            vec![EliminationStageOverride {
                stage: EliminationStage::SingleFinal,
                plan: EliminationSeriesPlan::balanced(oversized_realtime()),
            }],
        ),
        Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::Arena(ArenaConfig::new(
                NonZeroU32::new(MIN_DURATION_SECONDS).unwrap(),
                match oversized_realtime() {
                    Clock::Realtime(clock) => clock,
                    Clock::Correspondence(_) => unreachable!(),
                },
            )),
        },
    ];

    for configuration in configurations {
        assert!(matches!(
            validate_creation_configuration(&configuration, 4),
            Err(DefinitionError::ValueOutOfRange { .. })
        ));
    }
}

#[test]
fn swiss_config_construction_rejects_invalid_cross_options() {
    let mut swiss = SwissConfig::swiss(NonZeroU32::new(3).unwrap(), realtime());
    swiss.standings = vec![SwissStandingsCriterion::MatchPoints];
    assert!(matches!(
        build_swiss_tournament(&swiss, 4),
        Err(DefinitionError::Swiss(_))
    ));
}

#[test]
fn elimination_builders_use_native_topology() {
    let plan = EliminationSeriesPlan::balanced(realtime());
    let single = EliminationConfig {
        topology: EliminationTopology::Single { bronze: true },
        default_plan: plan.clone(),
        stage_overrides: Vec::new(),
    };
    let double = EliminationConfig {
        topology: EliminationTopology::Double,
        default_plan: plan,
        stage_overrides: Vec::new(),
    };

    let single = build_single_elimination_bracket(&single, 4).unwrap();
    let double = build_double_elimination_bracket(&double, 4).unwrap();
    assert!(single
        .nodes()
        .iter()
        .any(|node| node.stage == EliminationStage::Bronze));
    assert!(double
        .nodes()
        .iter()
        .any(|node| node.stage == EliminationStage::Reset));
}
