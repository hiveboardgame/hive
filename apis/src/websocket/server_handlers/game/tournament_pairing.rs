use shared_types::{
    tournament::elimination::Stage,
    tournament_view::{
        ArenaState,
        EliminationNodeResponse,
        EliminationState,
        RoundRobinState,
        SlotResponse,
        SwissState,
        TournamentFormatResponse,
    },
    GameId,
};
use std::collections::{HashMap, HashSet};

// Phase labels describe round/set openers. Other releases get a generic pairing notice.
pub(super) fn released_pairing_labels(
    format: &TournamentFormatResponse,
    released: &[GameId],
) -> HashMap<GameId, String> {
    let mut pairings = HashMap::new();
    if released.is_empty() {
        return pairings;
    }
    let released = released.iter().collect::<HashSet<_>>();
    let mut add = |slot: &SlotResponse, label: String| {
        if let Some(id) = slot.game_id().filter(|id| released.contains(id)) {
            if slot.resolution.is_none() {
                pairings.insert(id.clone(), label);
            }
        }
    };
    // TODO: i18n once copy is approved.
    match format {
        TournamentFormatResponse::Swiss(SwissState { rounds, slots, .. }) => {
            for round in rounds {
                for encounter in &round.encounters {
                    if let Some(slot) = encounter.slot_ids.first().and_then(|id| slots.get(id)) {
                        add(slot, format!("Round {} starts", round.round_index + 1));
                    }
                }
            }
        }
        TournamentFormatResponse::Elimination(EliminationState { nodes, slots, .. }) => {
            for node in nodes {
                let stage = stage_label(node.stage, nodes);
                let Some(series) = &node.series else { continue };
                for (index, set) in series.sets.iter().enumerate() {
                    if let Some(slot) = set
                        .slots
                        .first()
                        .and_then(|entry| slots.get(&entry.slot_id))
                    {
                        let label = if index == 0 {
                            format!("New match · {stage}")
                        } else {
                            format!("Tiebreak set {} · {stage}", index + 1)
                        };
                        add(slot, label);
                    }
                }
            }
        }
        TournamentFormatResponse::RoundRobin(RoundRobinState { .. })
        | TournamentFormatResponse::Arena(ArenaState { .. }) => {}
    }
    pairings
}

fn stage_label(stage: Stage, nodes: &[EliminationNodeResponse]) -> String {
    let rounds = nodes
        .iter()
        .filter_map(|node| match node.stage {
            Stage::SingleRound { round_index } => Some(round_index + 2),
            Stage::WinnersRound { round_index } => Some(round_index + 1),
            _ => None,
        })
        .max()
        .unwrap_or(1);
    // TODO: i18n once copy is approved.
    match stage {
        Stage::SingleFinal => String::from("Final"),
        Stage::GrandFinal => String::from("Grand final"),
        Stage::Reset => String::from("Reset final"),
        Stage::Bronze => String::from("Third place"),
        Stage::WinnersRound { round_index } if round_index + 1 == rounds => {
            String::from("Upper final")
        }
        Stage::SingleRound { round_index } | Stage::WinnersRound { round_index } => {
            if round_index + 2 == rounds {
                String::from("Semifinals")
            } else {
                format!("Round of {}", 1usize << (rounds - round_index))
            }
        }
        Stage::LosersMajor { round_index } if round_index + 2 == rounds => {
            String::from("Lower final")
        }
        Stage::LosersMinor { round_index } if round_index + 2 == rounds => {
            String::from("Lower semifinal")
        }
        Stage::LosersMinor { .. } | Stage::LosersMajor { .. } => format!(
            "Lower round {}",
            stage.lower_round_ordinal().unwrap_or_default() + 1
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hive_lib::GameStatus;
    use shared_types::{
        tournament::{
            elimination::{Config as EliminationConfig, SeriesPlan, Source, Topology},
            swiss::Config as SwissConfig,
            Clock,
            EliminationNodeId,
            RealtimeClock,
            Score,
            SeriesGameId,
            SlotKey,
            SwissGameId,
            SwissLeg,
        },
        tournament_view::{
            CompactTournamentGameResponse,
            EliminationNodeStateResponse,
            EliminationSeriesResponse,
            EliminationSeriesSetResponse,
            EliminationSeriesSlotResponse,
            SwissEncounterResponse,
            SwissRoundResponse,
        },
        GameSpeed,
        GameStart,
        SwissProgress,
    };
    use std::num::NonZeroU32;
    use uuid::Uuid;

    fn clock() -> Clock {
        Clock::Realtime(RealtimeClock {
            base_seconds: NonZeroU32::new(300).unwrap(),
            increment_seconds: 0,
        })
    }

    fn slot(index: u32, key: SlotKey) -> SlotResponse {
        let participants = [Uuid::from_u128(1), Uuid::from_u128(2)];
        SlotResponse {
            id: Uuid::from_u128(u128::from(index) + 10),
            key,
            participants,
            clock: clock(),
            resolution: None,
            outcome: None,
            awarded_game_points: None,
            resolved_at: None,
            game: Some(CompactTournamentGameResponse {
                game_id: GameId(format!("game-{index}")),
                participants,
                status: GameStatus::NotStarted,
                start: GameStart::Ready,
                finished: false,
                ratings: [None; 2],
                berserked: [false; 2],
                speed: GameSpeed::Blitz,
                finished_at: None,
            }),
            scheduled_at: None,
            deadline_at: None,
            waits_for: None,
            available_results: Vec::new(),
            available_admin_actions: HashSet::new(),
        }
    }

    #[test]
    fn elimination_announces_only_released_set_openers() {
        let node_id = EliminationNodeId::new(0);
        let mut slots = HashMap::new();
        let sets = (0..2)
            .map(|set| EliminationSeriesSetResponse {
                phase_index: 0,
                score: [0, 0],
                slots: (0..2)
                    .map(|game| {
                        let index = set * 2 + game;
                        let game_slot = slot(
                            index,
                            SlotKey::Elimination {
                                node: node_id,
                                slot: SeriesGameId::new(index),
                            },
                        );
                        let slot_id = game_slot.id;
                        slots.insert(slot_id, game_slot);
                        EliminationSeriesSlotResponse { slot_id }
                    })
                    .collect(),
            })
            .collect();
        let format = TournamentFormatResponse::Elimination(EliminationState {
            slots,
            configuration: EliminationConfig {
                topology: Topology::Single { bronze: false },
                default_plan: SeriesPlan::balanced(clock()),
                stage_overrides: Vec::new(),
            },
            nodes: vec![EliminationNodeResponse {
                node_id,
                wave_index: 0,
                stage: Stage::SingleFinal,
                stage_ordinal: 0,
                sources: [
                    Source::InitialSeed { seed_index: 0 },
                    Source::InitialSeed { seed_index: 1 },
                ],
                entrants: [Some(Uuid::from_u128(1)), Some(Uuid::from_u128(2))],
                possible_entrants: [Vec::new(), Vec::new()],
                state: EliminationNodeStateResponse::Active,
                series: Some(EliminationSeriesResponse {
                    score: [0; 2],
                    sets,
                }),
            }],
            complete: false,
            player_results: Vec::new(),
            withdrawable_entrants: HashSet::new(),
        });
        assert!(released_pairing_labels(&format, &[]).is_empty());
        assert!(released_pairing_labels(&format, &[GameId("game-1".into())]).is_empty());
        for opener in [0, 2] {
            let notices = released_pairing_labels(
                &format,
                &[
                    GameId(format!("game-{opener}")),
                    GameId(format!("game-{}", opener + 1)),
                ],
            );
            assert_eq!(notices.len(), 1);
            assert!(notices.contains_key(&GameId(format!("game-{opener}"))));
        }
    }

    #[test]
    fn double_swiss_announces_new_pairing_without_repeating_for_second_leg() {
        let slots: Vec<_> = [SwissLeg::First, SwissLeg::Second]
            .into_iter()
            .enumerate()
            .map(|(index, leg)| {
                slot(
                    index as u32,
                    SlotKey::Swiss {
                        slot: SwissGameId {
                            round_index: 1,
                            pairing_index: 0,
                            leg,
                        },
                    },
                )
            })
            .collect();
        let format = TournamentFormatResponse::Swiss(SwissState {
            slots: slots.iter().map(|slot| (slot.id, slot.clone())).collect(),
            configuration: SwissConfig::double_swiss(NonZeroU32::new(3).unwrap(), clock()),
            rounds: vec![SwissRoundResponse {
                round_index: 1,
                byes: Vec::new(),
                encounters: vec![SwissEncounterResponse {
                    pairing_index: 0,
                    participants: [Uuid::from_u128(1), Uuid::from_u128(2)],
                    pre_round_primary_scores: [Score::new(0); 2],
                    rating_snapshots: [None; 2],
                    slot_ids: slots.iter().map(|slot| slot.id).collect(),
                    completion: None,
                }],
            }],
            progress: SwissProgress::AwaitingResults,
            withdrawable_entrants: HashSet::new(),
            closeout_eligible_slots: 0,
        });
        assert_eq!(
            released_pairing_labels(&format, &[GameId("game-0".into())]).len(),
            1
        );
        assert!(released_pairing_labels(&format, &[GameId("game-1".into())]).is_empty());
    }
}
