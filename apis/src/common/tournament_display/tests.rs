use super::*;
use crate::providers::{RoundRobinState, SwissState};
use hive_lib::{Color, GameResult};
use reactive_stores::Store;
use shared_types::{
    tournament::{
        round_robin::Config as RoundRobinConfig,
        swiss::Config as SwissConfig,
        GameOutcome,
        PlayedGameOutcome,
        Resolution,
        RoundRobinGameId,
        SwissGameId,
        SwissLeg,
    },
    tournament_view::CompactTournamentGameResponse,
    Clock,
    GameId,
    GameSpeed,
    GameStart,
    SwissProgress,
};
use std::{collections::HashSet, num::NonZeroU32};

fn slot(id: u128, finished_at: Option<i64>) -> SlotResponse {
    let outcome = finished_at.map(|_| GameOutcome::Played(PlayedGameOutcome::WhiteWin));
    SlotResponse {
        id: Uuid::from_u128(id),
        key: SlotKey::RoundRobin {
            slot: RoundRobinGameId::new(id as usize),
        },
        participants: [Uuid::from_u128(100), Uuid::from_u128(200)],
        clock: Clock::Realtime(RealtimeClock {
            base_seconds: NonZeroU32::new(300).unwrap(),
            increment_seconds: 0,
        }),
        resolution: outcome.map(Resolution::Result),
        outcome,
        awarded_game_points: None,
        resolved_at: finished_at.map(|seconds| DateTime::from_timestamp(seconds, 0).unwrap()),
        game: None,
        scheduled_at: None,
        deadline_at: None,
        waits_for: None,
        available_results: Vec::new(),
        available_admin_actions: HashSet::new(),
    }
}

fn round_robin(slots: &[SlotResponse], policy: ReleasePolicy) -> TournamentFormatState {
    let mut configuration = RoundRobinConfig::standard(NonZeroU32::new(6).unwrap(), slots[0].clock);
    configuration.release_policy = policy;
    TournamentFormatState::RoundRobin(Store::new(RoundRobinState {
        configuration,
        rounds: Vec::new(),
        matches: Vec::new(),
        withdrawable_entrants: HashSet::new(),
        closeout_eligible_slots: 0,
        slots: slots.iter().map(|slot| (slot.id, slot.clone())).collect(),
    }))
}

fn swiss(slots: &[SlotResponse], configuration: SwissConfig) -> TournamentFormatState {
    TournamentFormatState::Swiss(Store::new(SwissState {
        configuration,
        rounds: Vec::new(),
        progress: SwissProgress::AwaitingResults,
        withdrawable_entrants: HashSet::new(),
        closeout_eligible_slots: 0,
        slots: slots.iter().map(|slot| (slot.id, slot.clone())).collect(),
    }))
}

#[test]
fn unlocked_rr_numbers_completions_per_opponent_using_game_finish_time() {
    Owner::new().with(|| {
        let mut slots = vec![
            slot(1, None),
            slot(2, Some(30)),
            slot(3, Some(10)),
            slot(4, Some(20)),
        ];
        slots[1].participants.reverse();
        // A later tournament resolution must not replace the actual game finish time.
        slots[2].game = Some(CompactTournamentGameResponse {
            game_id: GameId(String::from("finished-game")),
            participants: slots[2].participants,
            status: GameStatus::Finished(GameResult::Winner(Color::White)),
            start: GameStart::Ready,
            finished: true,
            ratings: [None; 2],
            berserked: [false; 2],
            speed: GameSpeed::Blitz,
            finished_at: slots[2].resolved_at,
        });
        slots[2].resolved_at = DateTime::from_timestamp(99, 0);
        let mut other_opponent = slot(5, Some(1));
        other_opponent.participants[1] = Uuid::from_u128(300);
        slots.push(other_opponent);
        let format = round_robin(&slots, ReleasePolicy::FullyUnlocked);
        let numbers = || {
            slots
                .iter()
                .enumerate()
                .map(|(index, slot)| tournament_game_number(format, slot, index + 1))
                .collect::<Vec<_>>()
        };
        assert_eq!(numbers(), vec![None, Some(3), Some(1), Some(2), Some(1)]);

        let completed = slot(1, Some(40));
        let TournamentFormatState::RoundRobin(state) = format else {
            unreachable!()
        };
        state.slots().update(|slots| {
            slots.insert(completed.id, completed.clone());
        });
        assert_eq!(tournament_game_number(format, &completed, 1), Some(4));
        assert_eq!(numbers()[1..], [Some(3), Some(1), Some(2), Some(1)]);
        let mut displayed = slots[..4].to_vec();
        displayed.sort_by_key(tournament_completion_order);
        assert_eq!(
            displayed.iter().map(|slot| slot.id).collect::<Vec<_>>(),
            [3, 4, 2, 1].map(Uuid::from_u128)
        );
    });
}

#[test]
fn unlocked_double_swiss_numbers_each_encounter_independently_and_breaks_timestamp_ties() {
    Owner::new().with(|| {
        let mut slots = vec![
            slot(1, Some(20)),
            slot(2, Some(10)),
            slot(3, Some(10)),
            slot(4, Some(10)),
        ];
        for (index, slot) in slots.iter_mut().enumerate() {
            slot.key = SlotKey::Swiss {
                slot: SwissGameId {
                    round_index: (index / 2) as u32,
                    pairing_index: 0,
                    leg: if index % 2 == 0 {
                        SwissLeg::First
                    } else {
                        SwissLeg::Second
                    },
                },
            };
        }
        let mut other_pairing = slots[0].clone();
        other_pairing.id = Uuid::from_u128(5);
        other_pairing.resolved_at = DateTime::from_timestamp(1, 0);
        other_pairing.key = SlotKey::Swiss {
            slot: SwissGameId {
                round_index: 0,
                pairing_index: 1,
                leg: SwissLeg::First,
            },
        };
        slots.push(other_pairing);
        let configuration = SwissConfig::double_swiss(NonZeroU32::new(3).unwrap(), slots[0].clock);
        let format = swiss(&slots, configuration);
        assert_eq!(
            slots
                .iter()
                .map(|slot| tournament_game_number(format, slot, 1))
                .collect::<Vec<_>>(),
            vec![Some(2), Some(1), Some(1), Some(2), Some(1)]
        );
    });
}

#[test]
fn scheduled_numbering_is_preserved_for_sequential_and_single_swiss_games() {
    Owner::new().with(|| {
        let slots = [slot(1, None), slot(2, Some(10))];
        let mut double = SwissConfig::double_swiss(NonZeroU32::new(3).unwrap(), slots[0].clock);
        double.double_swiss_release_policy = ReleasePolicy::SequentialPerMatchup;
        for format in [
            round_robin(&slots, ReleasePolicy::SequentialPerMatchup),
            swiss(&slots, double),
            swiss(
                &slots,
                SwissConfig::swiss(NonZeroU32::new(3).unwrap(), slots[0].clock),
            ),
        ] {
            assert_eq!(tournament_game_number(format, &slots[0], 1), Some(1));
            assert_eq!(tournament_game_number(format, &slots[1], 2), Some(2));
        }
    });
}
