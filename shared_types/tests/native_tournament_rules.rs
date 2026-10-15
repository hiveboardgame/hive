use codee::{binary::MsgpackSerdeCodec, Decoder, Encoder};
use shared_types::{
    tournament::{
        arena::Config as ArenaConfig,
        elimination::Source,
        round_robin::Config as RoundRobinConfig,
        BotAdmission,
        Config,
        FormatConfig,
    },
    Clock,
    RealtimeClock,
};
use std::num::NonZeroU32;
use tournamint::{arena::ArenaConfig as NativeArenaConfig, elimination::EliminationNodeId};

fn clock() -> RealtimeClock {
    RealtimeClock {
        base_seconds: NonZeroU32::new(180).unwrap(),
        increment_seconds: 1,
    }
}

#[test]
fn arena_rules_survive_storage_and_websocket_transport() {
    let json = r#"{
        "rules":{"scoring":{"fire":false,"berserk":null,"quick_draw":null,"repeated_draw":null},
            "pairing_policy":"grouped","rematch_cooldown":1,"rematch_wait_secs":15,"max_color_strike":3,"initial_color":"white"},
        "duration_seconds":3600,"game_clock":{"base_seconds":180,"increment_seconds":1}
    }"#;
    let mut configuration = ArenaConfig::new(NonZeroU32::new(3600).unwrap(), clock());
    configuration.rules = NativeArenaConfig::plain();
    assert_eq!(
        serde_json::to_value(&configuration).unwrap(),
        serde_json::from_str::<serde_json::Value>(json).unwrap()
    );
    assert_eq!(
        serde_json::from_str::<ArenaConfig>(json).unwrap(),
        configuration
    );
    let bytes = MsgpackSerdeCodec::encode(&configuration).unwrap();
    let decoded: ArenaConfig = MsgpackSerdeCodec::decode(&bytes).unwrap();
    assert_eq!(decoded, configuration);
}

#[test]
fn native_round_robin_rules_preserve_the_storage_shape_and_nonzero_validation() {
    let json = r#"{
        "rules":{"repeats":2,"initial_color":"white",
            "point_system":{"win":2,"draw":1,"loss":0,"forfeit_loss":0},
            "primary_score":"game_points","match_point_system":{"win":2,"draw":1,"loss":0,"forfeit_loss":0},
            "standings":{"criteria":["primary_score"]}},
        "release_policy":"fully_unlocked","clock":{"mode":"realtime","control":{"base_seconds":180,"increment_seconds":1}}
    }"#;
    let mut configuration =
        RoundRobinConfig::standard(NonZeroU32::new(2).unwrap(), Clock::Realtime(clock()));
    assert_eq!(
        serde_json::to_value(&configuration).unwrap(),
        serde_json::from_str::<serde_json::Value>(json).unwrap()
    );
    assert_eq!(
        serde_json::from_str::<RoundRobinConfig>(json).unwrap(),
        configuration
    );
    let bytes = MsgpackSerdeCodec::encode(&configuration).unwrap();
    let decoded: RoundRobinConfig = MsgpackSerdeCodec::decode(&bytes).unwrap();
    assert_eq!(decoded, configuration);
    configuration.rules.repeats = 0;
    assert!(Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::RoundRobin(configuration)
    }
    .validate_for_creation()
    .is_err());
}

#[test]
fn native_elimination_sources_work_with_the_websocket_codec() {
    for (source, json) in [
        (
            Source::InitialSeed { seed_index: 3 },
            r#"{"initial_seed":{"seed_index":3}}"#,
        ),
        (
            Source::WinnerOf(EliminationNodeId::new(7)),
            r#"{"winner_of":7}"#,
        ),
        (
            Source::LoserOf(EliminationNodeId::new(8)),
            r#"{"loser_of":8}"#,
        ),
        (Source::Vacant, r#""vacant""#),
    ] {
        assert_eq!(serde_json::to_string(&source).unwrap(), json);
        let bytes = MsgpackSerdeCodec::encode(&source).unwrap();
        let decoded: Source = MsgpackSerdeCodec::decode(&bytes).unwrap();
        assert_eq!(decoded, source);
    }
}

#[test]
fn arena_creation_derives_the_bounded_wait_from_the_clock() {
    for (base, increment, expected) in [
        (1, 0, 8),
        (29, 0, 8),
        (30, 0, 10),
        (59, 0, 10),
        (60, 0, 15),
        (60, 2, 20),
        (180, 0, 24),
        (180, 2, 32),
        (300, 0, 36),
        (300, 3, 48),
        (600, 0, 50),
        (u32::MAX, u32::MAX, 50),
    ] {
        let configuration = ArenaConfig::new(
            NonZeroU32::new(3600).unwrap(),
            RealtimeClock {
                base_seconds: NonZeroU32::new(base).unwrap(),
                increment_seconds: increment,
            },
        );
        assert_eq!(configuration.rules.rematch_wait_secs, expected);
        assert!(Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::Arena(configuration),
        }
        .validate_for_creation()
        .is_ok());
    }
}

#[test]
fn arena_creation_rejects_forged_rules_even_when_native_validation_accepts_them() {
    let expected = ArenaConfig::new(NonZeroU32::new(3600).unwrap(), clock());
    let mut huge_bonus = expected.rules;
    huge_bonus.scoring.berserk.as_mut().unwrap().win_bonus = tournamint::Score::new(u32::MAX);
    let mut different_wait = expected.rules;
    different_wait.rematch_wait_secs = 8;
    let mut different_pairing = expected.rules;
    different_pairing.rematch_cooldown = 0;
    for rules in [
        huge_bonus,
        different_wait,
        different_pairing,
        NativeArenaConfig::plain(),
    ] {
        assert!(rules.validate().is_ok());
        let mut forged = expected.clone();
        forged.rules = rules;
        // Exercise the untrusted serialized configuration received by the server.
        let request = Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::Arena(forged),
        };
        let decoded: Config =
            serde_json::from_value(serde_json::to_value(request).unwrap()).unwrap();
        assert!(decoded.validate_for_creation().is_err());
    }
}
