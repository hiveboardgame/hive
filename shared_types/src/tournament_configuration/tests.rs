use super::*;
use crate::CorrespondenceClock;
use serde_json::Value as JsonValue;
#[test]
fn format_family_tags_are_stable_across_subformats() {
    for (format, expected) in [
        (Format::RoundRobin, "round_robin"),
        (Format::Swiss, "swiss"),
        (Format::DoubleSwiss, "swiss"),
        (Format::SingleElimination, "elimination"),
        (Format::DoubleElimination, "elimination"),
        (Format::Arena, "arena"),
    ] {
        assert_eq!(format.family_tag(), expected);
    }
}

fn phase(limit: SetLimit, clock: Clock) -> EliminationSeriesPhase {
    EliminationSeriesPhase {
        games_per_set: 2,
        color_order: vec![EntrantSide::First, EntrantSide::Second],
        set_limit: limit,
        clinch: ClinchPolicy::PlayAll,
        clock,
    }
}

fn realtime() -> Clock {
    Clock::Realtime(RealtimeClock {
        base_seconds: NonZeroU32::new(300).unwrap(),
        increment_seconds: 3,
    })
}

fn elimination(plan: EliminationSeriesPlan) -> Config {
    Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::Elimination(EliminationConfig {
            topology: EliminationTopology::Single { bronze: false },
            default_plan: plan.clone(),
            stage_overrides: Vec::new(),
        }),
    }
}

#[test]
fn elimination_creation_accepts_user_configuration_without_frozen_topology() {
    let plan = EliminationSeriesPlan::balanced(realtime());
    let configuration = elimination(plan);
    assert!(configuration.validate_for_creation().is_ok());
}

#[test]
fn elimination_requires_a_balanced_color_inventory_for_each_even_set() {
    let mut invalid_phase = phase(SetLimit::UntilDecisive, realtime());
    invalid_phase.color_order = vec![EntrantSide::First, EntrantSide::First];
    let plan = EliminationSeriesPlan {
        phases: vec![invalid_phase],
    };
    assert!(matches!(
        elimination(plan).validate_for_creation(),
        Err(ConfigError::InvalidEliminationPlan { .. })
    ));
}

#[test]
fn only_final_elimination_phase_can_repeat_forever() {
    let plan = EliminationSeriesPlan {
        phases: vec![
            phase(SetLimit::UntilDecisive, realtime()),
            phase(SetLimit::UntilDecisive, realtime()),
        ],
    };
    assert!(elimination(plan).validate_for_creation().is_err());
}

#[test]
fn bronze_override_requires_a_bronze_match() {
    let plan = EliminationSeriesPlan::balanced(Clock::Realtime(RealtimeClock {
        base_seconds: NonZeroU32::new(300).unwrap(),
        increment_seconds: 3,
    }));
    let mut configuration = elimination(plan.clone());
    let FormatConfig::Elimination(elimination) = &mut configuration.format else {
        unreachable!();
    };
    elimination.stage_overrides.push(EliminationStageOverride {
        stage: EliminationStage::Bronze,
        plan,
    });
    assert!(matches!(
        configuration.validate_for_creation(),
        Err(ConfigError::IncompatibleEliminationStage {
            stage: EliminationStage::Bronze,
            ..
        })
    ));
}

#[test]
fn hive_workload_limits_reject_oversized_formats() {
    let cases = [
        ("round-robin repeats", {
            let mut config = RoundRobinConfig::standard(
                NonZeroU32::new(MAX_ROUND_ROBIN_REPEATS + 1).unwrap(),
                realtime(),
            );
            config.game_point_system = PointSystem::STANDARD;
            Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::RoundRobin(config),
            }
        }),
        (
            "Swiss rounds",
            Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::Swiss(SwissConfig::swiss(
                    NonZeroU32::new(MAX_SWISS_ROUNDS + 1).unwrap(),
                    realtime(),
                )),
            },
        ),
    ];
    for (field, config) in cases {
        assert!(matches!(
            config.validate_for_creation(),
            Err(ConfigError::NumericLimit { field: found, .. }) if found == field
        ));
    }
}

#[test]
fn automatic_swiss_rounds_resolve_from_the_frozen_field_and_clamp_to_opponents() {
    for (players, extra_rounds, expected) in [
        (5, -2, 3),
        (5, 0, 3),
        (5, 3, 4),
        (8, -1, 3),
        (6, 2, 5),
        (8, 3, 6),
        (16, -2, 3),
        (16, 2, 6),
        (32, -2, 3),
        (32, -1, 4),
        (160, 2, 10),
    ] {
        let requested = serde_json::json!({"Automatic": {"extra_rounds": extra_rounds}});
        let configuration: SwissRoundConfiguration = serde_json::from_value(requested.clone())
            .expect("signed round adjustments can be loaded from saved configuration");
        assert_eq!(serde_json::to_value(configuration).unwrap(), requested);
        let resolved = configuration.resolve(players).unwrap();
        assert_eq!(
            resolved.resolved_rounds().map(NonZeroU32::get),
            Some(expected),
        );
        assert_eq!(
            serde_json::to_value(resolved).unwrap(),
            serde_json::json!({"Resolved": {"rounds": expected, "requested_extra": extra_rounds}}),
        );
        assert_eq!(resolved.resolve(players + 16), Some(resolved));
    }
}

#[test]
fn saved_swiss_round_adjustments_are_validated_before_and_after_resolution() {
    for extra in [-3, -2, -1, 0, 1, 2, 3, 4] {
        let mut config = SwissConfig::automatic_swiss(extra, realtime());
        let expected = if (-2..=3).contains(&extra) {
            Ok(())
        } else {
            Err(ConfigError::InvalidSwissRoundAdjustment { found: extra })
        };
        assert_eq!(validate_swiss(&config), expected);
        config.rounds = SwissRoundConfiguration::resolved(NonZeroU32::new(5).unwrap(), Some(extra));
        assert_eq!(validate_swiss(&config), expected);
    }
}

#[test]
fn format_machine_text_matches_serde_and_round_trips_strictly() {
    let cases = [
        (Format::RoundRobin, "round_robin"),
        (Format::Swiss, "swiss"),
        (Format::DoubleSwiss, "double_swiss"),
        (Format::SingleElimination, "single_elimination"),
        (Format::DoubleElimination, "double_elimination"),
        (Format::Arena, "arena"),
    ];

    for (value, token) in cases {
        assert_eq!(value.as_str(), token);
        assert_eq!(value.to_string(), token);
        assert_eq!(token.parse::<Format>().unwrap(), value);
        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            format!(r#""{token}""#)
        );
        assert_eq!(
            serde_json::from_str::<Format>(&format!(r#""{token}""#)).unwrap(),
            value
        );
    }

    for unknown in ["", "RoundRobin", "round-robin", "dutch", "burstein"] {
        assert_eq!(
            unknown.parse::<Format>(),
            Err(FormatParseError::Invalid {
                found: unknown.to_string(),
            })
        );
        assert!(serde_json::from_str::<Format>(&format!(r#""{unknown}""#)).is_err());
    }
}

#[test]
fn server_creation_rejects_burstein() {
    let configuration = Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::Swiss(SwissConfig::swiss(NonZeroU32::new(3).unwrap(), realtime())),
    };
    let mut request = serde_json::to_value(configuration).unwrap();
    let system = request
        .pointer_mut("/format/configuration/system")
        .and_then(JsonValue::as_object_mut)
        .unwrap();
    let native_configuration = system.remove("dutch").unwrap();
    system.insert(String::from("burstein"), native_configuration);
    let request: Config = serde_json::from_value(request).unwrap();

    assert_eq!(
        request.validate_for_creation(),
        Err(ConfigError::UnsupportedSwissSystem)
    );
}

#[test]
fn hive_rejects_round_robin_progressive_but_accepts_swiss_progressive() {
    let mut round_robin = RoundRobinConfig::standard(NonZeroU32::new(1).unwrap(), realtime());
    round_robin.standings.push(RoundRobinCriterion::Progressive(
        RoundRobinProgressiveOptions {
            cut_first_round: false,
        },
    ));
    assert_eq!(
        validate_round_robin(&round_robin),
        Err(ConfigError::RoundRobinProgressive)
    );
    for primary in [
        DoubleSwissPrimaryScore::GamePoints,
        DoubleSwissPrimaryScore::MatchPoints,
    ] {
        let config = SwissConfig::double_swiss_with_primary_score(
            NonZeroU32::new(3).unwrap(),
            realtime(),
            primary,
        );
        assert!(validate_swiss(&config).is_ok());
    }
    assert!(validate_swiss(&SwissConfig::swiss(NonZeroU32::new(3).unwrap(), realtime())).is_ok());
}

#[test]
fn seed_tiebreak_directions_are_mutually_exclusive_at_creation() {
    let mut round_robin = RoundRobinConfig::standard(NonZeroU32::new(1).unwrap(), realtime());
    let mut swiss = SwissConfig::swiss(NonZeroU32::new(3).unwrap(), realtime());
    for order in [
        TournamentPairingNumberOrder::Ascending,
        TournamentPairingNumberOrder::Descending,
    ] {
        round_robin
            .standings
            .push(RoundRobinCriterion::TournamentPairingNumber(order));
        swiss
            .standings
            .push(SwissStandingsCriterion::TournamentPairingNumber(order));
        let expected = if order == TournamentPairingNumberOrder::Ascending {
            Ok(())
        } else {
            Err(ConfigError::MultipleSeedTiebreaks)
        };
        assert_eq!(validate_round_robin(&round_robin), expected);
        assert_eq!(validate_swiss(&swiss), expected);
    }
}

#[test]
fn explicit_score_tiebreaks_require_the_alternate_double_swiss_score() {
    for primary in [
        DoubleSwissPrimaryScore::GamePoints,
        DoubleSwissPrimaryScore::MatchPoints,
    ] {
        let mut config = SwissConfig::double_swiss_with_primary_score(
            NonZeroU32::new(3).unwrap(),
            realtime(),
            primary,
        );
        let (alternate, redundant) = match primary {
            DoubleSwissPrimaryScore::GamePoints => (
                SwissStandingsCriterion::MatchPoints,
                SwissStandingsCriterion::GamePoints,
            ),
            DoubleSwissPrimaryScore::MatchPoints => (
                SwissStandingsCriterion::GamePoints,
                SwissStandingsCriterion::MatchPoints,
            ),
        };
        config.standings.push(alternate);
        assert!(validate_swiss(&config).is_ok());
        config.standings.push(redundant);
        assert_eq!(
            validate_swiss(&config),
            Err(ConfigError::InvalidScoreTiebreak)
        );
    }
    for criterion in [
        SwissStandingsCriterion::GamePoints,
        SwissStandingsCriterion::MatchPoints,
    ] {
        let mut config = SwissConfig::swiss(NonZeroU32::new(3).unwrap(), realtime());
        config.standings.push(criterion);
        assert_eq!(
            validate_swiss(&config),
            Err(ConfigError::InvalidScoreTiebreak)
        );
    }
}

#[test]
fn tournament_configurations_round_trip_through_the_websocket_codec() {
    use codee::{binary::MsgpackSerdeCodec, Decoder, Encoder};

    let mut formats = vec![FormatConfig::Arena(ArenaConfig::new(
        NonZeroU32::new(3600).unwrap(),
        RealtimeClock {
            base_seconds: NonZeroU32::new(300).unwrap(),
            increment_seconds: 3,
        },
    ))];
    for clock in [
        realtime(),
        Clock::Correspondence(CorrespondenceClock::DaysPerMove {
            seconds_per_move: NonZeroU32::new(86400).unwrap(),
        }),
        Clock::Correspondence(CorrespondenceClock::TotalTimeEach {
            seconds_each: NonZeroU32::new(172800).unwrap(),
        }),
    ] {
        let mut round_robin = RoundRobinConfig::standard(NonZeroU32::new(2).unwrap(), clock);
        round_robin
            .standings
            .extend(round_robin_creation_tiebreakers(
                round_robin.repeats,
                round_robin.primary_score,
            ));
        formats.push(FormatConfig::RoundRobin(round_robin));
        for extra in [-2, 0, 3] {
            formats.push(FormatConfig::Swiss(SwissConfig::automatic_swiss(
                extra, clock,
            )));
            formats.push(FormatConfig::Swiss(SwissConfig::automatic_double_swiss(
                extra,
                clock,
                DoubleSwissPrimaryScore::MatchPoints,
            )));
        }
        for topology in [
            EliminationTopology::Single { bronze: true },
            EliminationTopology::Double,
        ] {
            let mut plan = EliminationSeriesPlan::balanced(clock);
            plan.phases.insert(
                0,
                phase(SetLimit::AtMost(NonZeroU16::new(2).unwrap()), clock),
            );
            formats.push(FormatConfig::Elimination(EliminationConfig {
                topology,
                default_plan: plan,
                stage_overrides: vec![],
            }));
        }
    }
    for format in formats {
        let configuration = Config {
            bot_admission: BotAdmission::HumansAndBots,
            format,
        };
        let bytes =
            MsgpackSerdeCodec::encode(&configuration).expect("encode tournament configuration");
        let decoded: Config = MsgpackSerdeCodec::decode(&bytes).unwrap_or_else(|error| {
            panic!(
                "{} configuration cannot be read from the WebSocket: {error}",
                configuration.format()
            )
        });
        assert_eq!(decoded, configuration);
        let json = serde_json::to_value(&configuration).unwrap();
        assert_eq!(
            serde_json::from_value::<Config>(json.clone()).unwrap(),
            configuration
        );
        let mut unknown = json;
        unknown["format"]["extra_field"] = JsonValue::Bool(true);
        assert!(serde_json::from_value::<Config>(unknown).is_err());
    }
}

#[test]
fn round_robin_score_configuration_round_trips_and_rejects_incompatible_tiebreaks() {
    for primary in [
        RoundRobinPrimaryScore::GamePoints,
        RoundRobinPrimaryScore::MatchPoints,
    ] {
        let mut round_robin = RoundRobinConfig::standard(NonZeroU32::new(4).unwrap(), realtime());
        round_robin.primary_score = primary;
        let alternate = match primary {
            RoundRobinPrimaryScore::GamePoints => RoundRobinCriterion::MatchPoints,
            RoundRobinPrimaryScore::MatchPoints => RoundRobinCriterion::GamePoints,
        };
        round_robin.standings.push(alternate);
        let configuration = Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::RoundRobin(round_robin.clone()),
        };
        let encoded = serde_json::to_value(&configuration).unwrap();
        let decoded: Config = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded, configuration);
        assert!(decoded.validate_for_creation().is_ok());
        round_robin.standings.push(match primary {
            RoundRobinPrimaryScore::GamePoints => RoundRobinCriterion::GamePoints,
            RoundRobinPrimaryScore::MatchPoints => RoundRobinCriterion::MatchPoints,
        });
        assert_eq!(
            validate_round_robin(&round_robin),
            Err(ConfigError::InvalidScoreTiebreak)
        );
        round_robin.standings.pop();
        round_robin.repeats = NonZeroU32::new(3).unwrap();
        assert_eq!(
            validate_round_robin(&round_robin),
            Err(ConfigError::RoundRobinMatchScoringRequiresEvenRepeats)
        );
    }
}
