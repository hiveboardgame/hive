use shared_types::tournament::{
    round_robin::{
        Config as RoundRobinConfig,
        Criterion as RoundRobinCriterion,
        PrimaryScore as RoundRobinPrimaryScore,
    },
    standings::{Row, Value},
    swiss::{
        Criterion as SwissCriterion,
        PrimaryScore as DoubleSwissPrimaryScore,
        ScoreBasis,
        System as SwissSystem,
    },
    FormatConfig,
    PointSystem,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScorePresentation {
    GamePoints { half_point_units: bool },
    ScoreProduct { half_point_units: bool },
    MatchPoints,
    ArenaPoints,
    Integer,
    Ratio,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StandingsCount {
    RoundRobinSitOuts,
    RoundRobinMatchWins,
    RoundRobinMatchDraws,
    RoundRobinMatchLosses,
    SwissPairingAllocatedByes,
    SwissRequestedByes,
    SwissMatchWins,
    SwissMatchDraws,
    SwissMatchLosses,
}

impl StandingsCount {
    const fn index(self) -> usize {
        match self {
            Self::RoundRobinSitOuts | Self::SwissPairingAllocatedByes => 0,
            Self::SwissRequestedByes | Self::RoundRobinMatchWins => 1,
            Self::SwissMatchWins | Self::RoundRobinMatchDraws => 2,
            Self::SwissMatchDraws | Self::RoundRobinMatchLosses => 3,
            Self::SwissMatchLosses => 4,
        }
    }
}

pub fn standings_count(row: &Row, count: StandingsCount) -> u32 {
    row.counts.get(count.index()).copied().unwrap_or_default()
}

pub fn game_point_presentation(point_system: PointSystem) -> ScorePresentation {
    ScorePresentation::GamePoints {
        half_point_units: point_system == PointSystem::STANDARD,
    }
}

pub fn primary_score_presentation(configuration: &FormatConfig) -> ScorePresentation {
    match configuration {
        FormatConfig::RoundRobin(configuration) => round_robin_primary_presentation(configuration),
        FormatConfig::Swiss(configuration) => match configuration.system {
            SwissSystem::DoubleSwiss(system)
                if system.primary_score == DoubleSwissPrimaryScore::MatchPoints =>
            {
                ScorePresentation::MatchPoints
            }
            _ => game_point_presentation(configuration.game_point_system),
        },
        FormatConfig::Arena(_) => ScorePresentation::ArenaPoints,
        FormatConfig::Elimination(_) => ScorePresentation::Integer,
    }
}

fn round_robin_primary_presentation(configuration: &RoundRobinConfig) -> ScorePresentation {
    match configuration.primary_score {
        RoundRobinPrimaryScore::GamePoints => {
            game_point_presentation(configuration.game_point_system)
        }
        RoundRobinPrimaryScore::MatchPoints => ScorePresentation::MatchPoints,
    }
}

pub fn round_robin_criterion_presentation(
    configuration: &RoundRobinConfig,
    criterion: RoundRobinCriterion,
) -> ScorePresentation {
    match criterion {
        RoundRobinCriterion::PrimaryScore
        | RoundRobinCriterion::Koya(_)
        | RoundRobinCriterion::Progressive(_) => round_robin_primary_presentation(configuration),
        RoundRobinCriterion::GamePoints => game_point_presentation(configuration.game_point_system),
        RoundRobinCriterion::MatchPoints => ScorePresentation::MatchPoints,
        RoundRobinCriterion::SonnebornBerger(_) => ScorePresentation::ScoreProduct {
            half_point_units: configuration.primary_score == RoundRobinPrimaryScore::GamePoints
                && configuration.game_point_system == PointSystem::STANDARD,
        },
        RoundRobinCriterion::DirectEncounter(_) => ScorePresentation::Ratio,
        RoundRobinCriterion::TournamentPairingNumber(_)
        | RoundRobinCriterion::NumberOfWins
        | RoundRobinCriterion::MatchesWon
        | RoundRobinCriterion::GamesWon
        | RoundRobinCriterion::GamesWonWithBlack => ScorePresentation::Integer,
    }
}

fn swiss_score_basis_presentation(
    configuration: &shared_types::tournament::swiss::Config,
    basis: ScoreBasis,
) -> ScorePresentation {
    match basis {
        ScoreBasis::Primary => {
            primary_score_presentation(&FormatConfig::Swiss(configuration.clone()))
        }
        ScoreBasis::GamePoints => game_point_presentation(configuration.game_point_system),
        ScoreBasis::MatchPoints => ScorePresentation::MatchPoints,
    }
}

fn swiss_score_product_presentation(
    configuration: &shared_types::tournament::swiss::Config,
    basis: ScoreBasis,
) -> ScorePresentation {
    let half_point_units = matches!(
        swiss_score_basis_presentation(configuration, basis),
        ScorePresentation::GamePoints {
            half_point_units: true,
        }
    );
    ScorePresentation::ScoreProduct { half_point_units }
}

pub fn swiss_criterion_presentation(
    configuration: &shared_types::tournament::swiss::Config,
    criterion: SwissCriterion,
) -> ScorePresentation {
    match criterion {
        SwissCriterion::PrimaryScore => {
            primary_score_presentation(&FormatConfig::Swiss(configuration.clone()))
        }
        SwissCriterion::GamePoints => game_point_presentation(configuration.game_point_system),
        SwissCriterion::MatchPoints => ScorePresentation::MatchPoints,
        SwissCriterion::Buchholz(_) => {
            swiss_score_basis_presentation(configuration, ScoreBasis::Primary)
        }
        SwissCriterion::SonnebornBerger(options) => {
            swiss_score_product_presentation(configuration, options.score_basis)
        }
        SwissCriterion::Progressive(options) => {
            swiss_score_basis_presentation(configuration, options.score_basis)
        }
        SwissCriterion::DirectEncounter(_) => ScorePresentation::Ratio,
        SwissCriterion::TournamentPairingNumber(_)
        | SwissCriterion::NumberOfWins
        | SwissCriterion::GamesWon
        | SwissCriterion::GamesWonWithBlack => ScorePresentation::Integer,
    }
}

fn scaled_integer_text(value: u64, divisor: u64) -> String {
    let whole = value / divisor;
    let remainder = value % divisor;
    let fraction = match (divisor, remainder) {
        (_, 0) => "",
        (2, 1) | (4, 2) => "½",
        (4, 1) => "¼",
        (4, 3) => "¾",
        _ => unreachable!("standings scores use half-point or quarter-point scaling"),
    };
    if remainder == 0 {
        whole.to_string()
    } else if whole == 0 {
        fraction.to_string()
    } else {
        format!("{whole}{fraction}")
    }
}

pub fn half_point_text(value: u64) -> String {
    scaled_integer_text(value, 2)
}

pub fn value_text(value: Value, presentation: ScorePresentation) -> String {
    let value = match value {
        Value::Score(score) => u64::from(score.value()),
        Value::Integer(value) => value,
        Value::SignedInteger(value) => return value.to_string(),
        Value::NotApplicable => return String::from("—"),
    };
    match presentation {
        ScorePresentation::GamePoints {
            half_point_units: true,
        } => scaled_integer_text(value, 2),
        ScorePresentation::ScoreProduct {
            half_point_units: true,
        } => scaled_integer_text(value, 4),
        ScorePresentation::GamePoints {
            half_point_units: false,
        }
        | ScorePresentation::ScoreProduct {
            half_point_units: false,
        }
        | ScorePresentation::MatchPoints
        | ScorePresentation::ArenaPoints
        | ScorePresentation::Integer
        | ScorePresentation::Ratio => value.to_string(),
    }
}

pub fn primary_value_text(value: Value, configuration: &FormatConfig) -> String {
    value_text(value, primary_score_presentation(configuration))
}

#[cfg(test)]
mod tests {
    use super::{
        half_point_text,
        round_robin_criterion_presentation,
        swiss_criterion_presentation,
        value_text,
        ScorePresentation,
    };
    use shared_types::{
        tournament::{
            round_robin::{
                Config as RoundRobinConfig,
                Criterion as RoundRobinCriterion,
                ProgressiveOptions,
            },
            standings::Value,
            swiss::{
                BuchholzOptions,
                Config as SwissConfig,
                Criterion as SwissCriterion,
                ProgressiveOptions as SwissProgressiveOptions,
                ScoreBasis,
            },
            PointSystem,
            Score,
        },
        Clock,
        RealtimeClock,
    };
    use std::num::NonZeroU32;

    fn rr_config(points: PointSystem) -> RoundRobinConfig {
        let mut config = RoundRobinConfig::standard(NonZeroU32::new(2).unwrap(), clock());
        config.game_point_system = points;
        config
    }

    fn clock() -> Clock {
        Clock::Realtime(RealtimeClock {
            base_seconds: NonZeroU32::new(60).unwrap(),
            increment_seconds: 0,
        })
    }

    #[test]
    fn half_point_units_render_as_chess_points() {
        assert_eq!(half_point_text(0), "0");
        assert_eq!(half_point_text(1), "½");
        assert_eq!(half_point_text(2), "1");
        assert_eq!(half_point_text(3), "1½");
        assert_eq!(half_point_text(8), "4");
    }

    #[test]
    fn score_units_are_only_halved_when_the_column_declares_half_points() {
        let value = Value::Score(Score::new(3));
        assert_eq!(
            value_text(
                value,
                ScorePresentation::GamePoints {
                    half_point_units: true,
                },
            ),
            "1½",
        );
        assert_eq!(value_text(value, ScorePresentation::ArenaPoints), "3");
        assert_eq!(value_text(value, ScorePresentation::MatchPoints), "3");
    }

    #[test]
    fn integer_backed_point_sums_use_the_declared_game_point_scale() {
        let progressive = RoundRobinCriterion::Progressive(ProgressiveOptions::default());
        assert_eq!(
            round_robin_criterion_presentation(&rr_config(PointSystem::STANDARD), progressive),
            ScorePresentation::GamePoints {
                half_point_units: true,
            },
        );
        assert_eq!(
            value_text(
                Value::Integer(3),
                round_robin_criterion_presentation(&rr_config(PointSystem::STANDARD), progressive),
            ),
            "1½",
        );

        let custom = PointSystem {
            win: Score::new(3),
            draw: Score::new(1),
            loss: Score::new(0),
            forfeit_loss: Score::new(0),
        };
        assert_eq!(
            value_text(
                Value::Integer(3),
                round_robin_criterion_presentation(&rr_config(custom), progressive),
            ),
            "3",
        );
    }

    #[test]
    fn integer_backed_products_use_quarter_points_only_for_standard_game_scoring() {
        let criterion = RoundRobinCriterion::SonnebornBerger(Default::default());
        let standard =
            round_robin_criterion_presentation(&rr_config(PointSystem::STANDARD), criterion);
        assert_eq!(
            standard,
            ScorePresentation::ScoreProduct {
                half_point_units: true,
            },
        );
        assert_eq!(value_text(Value::Integer(7), standard), "1¾");

        let custom = PointSystem {
            win: Score::new(3),
            draw: Score::new(1),
            loss: Score::new(0),
            forfeit_loss: Score::new(0),
        };
        assert_eq!(
            value_text(
                Value::Integer(7),
                round_robin_criterion_presentation(&rr_config(custom), criterion),
            ),
            "7",
        );
    }

    #[test]
    fn swiss_integer_sums_follow_their_explicit_score_basis() {
        let configuration = SwissConfig::swiss(NonZeroU32::new(3).unwrap(), clock());
        assert_eq!(
            swiss_criterion_presentation(
                &configuration,
                SwissCriterion::Buchholz(BuchholzOptions::default()),
            ),
            ScorePresentation::GamePoints {
                half_point_units: true,
            },
        );
        assert_eq!(
            swiss_criterion_presentation(
                &configuration,
                SwissCriterion::Progressive(SwissProgressiveOptions {
                    score_basis: ScoreBasis::GamePoints,
                    cut_first_round: false,
                }),
            ),
            ScorePresentation::GamePoints {
                half_point_units: true,
            },
        );
        assert_eq!(
            swiss_criterion_presentation(
                &configuration,
                SwissCriterion::DirectEncounter(Default::default()),
            ),
            ScorePresentation::Ratio,
        );
    }

    #[test]
    fn swiss_products_normalize_standard_game_points_but_keep_custom_products_literal() {
        let mut configuration = SwissConfig::swiss(NonZeroU32::new(3).unwrap(), clock());
        let criterion = SwissCriterion::SonnebornBerger(Default::default());
        let standard = swiss_criterion_presentation(&configuration, criterion);
        assert_eq!(
            standard,
            ScorePresentation::ScoreProduct {
                half_point_units: true,
            },
        );
        assert_eq!(value_text(Value::Integer(7), standard), "1¾");

        configuration.game_point_system = PointSystem {
            win: Score::new(3),
            draw: Score::new(1),
            loss: Score::new(0),
            forfeit_loss: Score::new(0),
        };
        let custom = swiss_criterion_presentation(&configuration, criterion);
        assert_eq!(
            custom,
            ScorePresentation::ScoreProduct {
                half_point_units: false,
            },
        );
        assert_eq!(value_text(Value::Integer(7), custom), "7");
    }
}
