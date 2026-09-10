use super::model::LegacyRankGroup;
use anyhow::{bail, Result};
use hive_lib::Color;
use shared_types::{Tiebreaker, TournamentGameResult};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    iter,
};
use uuid::Uuid;

#[derive(Clone, Debug)]
struct Pairing {
    white: Uuid,
    black: Uuid,
    result: TournamentGameResult,
}

#[derive(Clone, Debug, Default)]
pub struct LegacyStandings {
    players: BTreeSet<Uuid>,
    pairings: BTreeMap<Uuid, Vec<Pairing>>,
    scores: BTreeMap<Uuid, HashMap<Tiebreaker, f32>>,
}

impl LegacyStandings {
    pub fn add_result(
        &mut self,
        white: Uuid,
        black: Uuid,
        result: TournamentGameResult,
    ) -> Result<()> {
        if white == black {
            bail!("a legacy standings pairing contains the same player twice");
        }
        self.players.insert(white);
        self.players.insert(black);
        let pairing = Pairing {
            white,
            black,
            result,
        };
        self.pairings
            .entry(white)
            .or_default()
            .push(pairing.clone());
        self.pairings.entry(black).or_default().push(pairing);
        Ok(())
    }

    pub fn rank_groups(&mut self, configured: &[Tiebreaker]) -> Result<Vec<LegacyRankGroup>> {
        let mut seen = HashSet::new();
        let plan = iter::once(Tiebreaker::RawPoints)
            .chain(configured.iter().copied())
            .filter(|criterion| seen.insert(*criterion))
            .collect::<Vec<_>>();
        for criterion in plan.iter().copied() {
            match criterion {
                Tiebreaker::RawPoints => self.compute_raw_points(),
                Tiebreaker::HeadToHead => self.compute_head_to_head(&plan),
                Tiebreaker::WinsAsBlack => self.compute_wins_as_black(),
                Tiebreaker::SonnebornBerger => self.compute_sonneborn_berger(),
                unsupported => {
                    bail!("legacy standings contain unsupported criterion {unsupported}")
                }
            }
        }

        let groups = self.grouped_by(&plan);
        let mut display_ordinal = 0_u32;
        groups
            .into_iter()
            .map(|mut user_ids| {
                user_ids.sort_unstable();
                let competition_rank = display_ordinal
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("legacy standings are too large"))?;
                display_ordinal = display_ordinal
                    .checked_add(u32::try_from(user_ids.len())?)
                    .ok_or_else(|| anyhow::anyhow!("legacy standings are too large"))?;
                Ok(LegacyRankGroup {
                    competition_rank,
                    user_ids,
                })
            })
            .collect()
    }

    fn compute_raw_points(&mut self) {
        let players = self.players.iter().copied().collect::<Vec<_>>();
        for player in players {
            let score = self
                .pairings
                .get(&player)
                .into_iter()
                .flatten()
                .map(|pairing| match pairing.result {
                    TournamentGameResult::Draw => 0.5,
                    TournamentGameResult::Winner(Color::White) if pairing.white == player => 1.0,
                    TournamentGameResult::Winner(Color::Black) if pairing.black == player => 1.0,
                    TournamentGameResult::Unknown
                    | TournamentGameResult::DoubleForfeit
                    | TournamentGameResult::Winner(_) => 0.0,
                })
                .sum();
            self.set_score(player, Tiebreaker::RawPoints, score);
        }
    }

    fn compute_wins_as_black(&mut self) {
        let players = self.players.iter().copied().collect::<Vec<_>>();
        for player in players {
            let score = self
                .pairings
                .get(&player)
                .into_iter()
                .flatten()
                .filter(|pairing| {
                    pairing.black == player
                        && pairing.result == TournamentGameResult::Winner(Color::Black)
                })
                .count() as f32;
            self.set_score(player, Tiebreaker::WinsAsBlack, score);
        }
    }

    fn compute_sonneborn_berger(&mut self) {
        let players = self.players.iter().copied().collect::<Vec<_>>();
        for player in players.iter().copied() {
            let mut score = 0.0_f32;
            for opponent in players.iter().copied().filter(|other| *other != player) {
                let opponent_points = self.score(opponent, Tiebreaker::RawPoints);
                for pairing in self.pairings_between(player, opponent) {
                    score += match pairing.result {
                        TournamentGameResult::Draw => 0.5 * opponent_points,
                        TournamentGameResult::Winner(Color::White) if pairing.white == player => {
                            opponent_points
                        }
                        TournamentGameResult::Winner(Color::Black) if pairing.black == player => {
                            opponent_points
                        }
                        TournamentGameResult::Unknown
                        | TournamentGameResult::DoubleForfeit
                        | TournamentGameResult::Winner(_) => 0.0,
                    };
                }
            }
            self.set_score(player, Tiebreaker::SonnebornBerger, score);
        }
    }

    fn compute_head_to_head(&mut self, plan: &[Tiebreaker]) {
        let position = plan
            .iter()
            .position(|criterion| *criterion == Tiebreaker::HeadToHead)
            .unwrap_or(0);
        let prior = &plan[..position];
        let mut scores = BTreeMap::<Uuid, f32>::new();
        for group in self.grouped_by(prior) {
            for first_index in 0..group.len() {
                for second_index in first_index.saturating_add(1)..group.len() {
                    let first = group[first_index];
                    let second = group[second_index];
                    let (first_score, second_score) = self.head_to_head_pair(first, second);
                    *scores.entry(first).or_default() += first_score;
                    *scores.entry(second).or_default() += second_score;
                }
            }
            for player in group {
                self.set_score(
                    player,
                    Tiebreaker::HeadToHead,
                    scores.get(&player).copied().unwrap_or_default(),
                );
            }
        }
    }

    fn head_to_head_pair(&self, first: Uuid, second: Uuid) -> (f32, f32) {
        let mut first_score = 0.0;
        let mut second_score = 0.0;
        for pairing in self.pairings_between(first, second) {
            match pairing.result {
                TournamentGameResult::Draw => {
                    first_score += 0.5;
                    second_score += 0.5;
                }
                TournamentGameResult::Winner(Color::White) if pairing.white == first => {
                    first_score += 1.0;
                }
                TournamentGameResult::Winner(Color::White) if pairing.white == second => {
                    second_score += 1.0;
                }
                TournamentGameResult::Winner(Color::Black) if pairing.black == first => {
                    first_score += 1.0;
                }
                TournamentGameResult::Winner(Color::Black) if pairing.black == second => {
                    second_score += 1.0;
                }
                TournamentGameResult::Unknown
                | TournamentGameResult::DoubleForfeit
                | TournamentGameResult::Winner(_) => {}
            }
        }
        (first_score, second_score)
    }

    fn pairings_between(&self, first: Uuid, second: Uuid) -> impl Iterator<Item = &Pairing> {
        self.pairings
            .get(&first)
            .into_iter()
            .flatten()
            .filter(move |pairing| pairing.white == second || pairing.black == second)
    }

    fn grouped_by(&self, criteria: &[Tiebreaker]) -> Vec<Vec<Uuid>> {
        let mut scored = self
            .players
            .iter()
            .copied()
            .map(|player| {
                let values = criteria
                    .iter()
                    .map(|criterion| self.score(player, *criterion))
                    .collect::<Vec<_>>();
                (player, values)
            })
            .collect::<Vec<_>>();
        scored.sort_by(|(first_player, first), (second_player, second)| {
            second
                .partial_cmp(first)
                .expect("legacy standings scores are always finite")
                .then_with(|| first_player.cmp(second_player))
        });

        let mut groups = Vec::<Vec<Uuid>>::new();
        let mut previous = None::<Vec<f32>>;
        for (player, values) in scored {
            if previous.as_ref() != Some(&values) {
                groups.push(Vec::new());
                previous = Some(values);
            }
            groups
                .last_mut()
                .expect("the current legacy standings group exists")
                .push(player);
        }
        groups
    }

    fn score(&self, player: Uuid, criterion: Tiebreaker) -> f32 {
        self.scores
            .get(&player)
            .and_then(|scores| scores.get(&criterion))
            .copied()
            .unwrap_or_default()
    }

    fn set_score(&mut self, player: Uuid, criterion: Tiebreaker, score: f32) {
        self.scores
            .entry(player)
            .or_default()
            .insert(criterion, score);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    #[test]
    fn repeated_direct_encounters_are_summed_like_the_old_application() {
        let first = player(1);
        let second = player(2);
        let third = player(3);
        let mut standings = LegacyStandings::default();
        standings
            .add_result(first, second, TournamentGameResult::Winner(Color::White))
            .unwrap();
        standings
            .add_result(second, first, TournamentGameResult::Draw)
            .unwrap();
        standings
            .add_result(first, third, TournamentGameResult::Winner(Color::Black))
            .unwrap();
        standings
            .add_result(second, third, TournamentGameResult::Winner(Color::White))
            .unwrap();

        let groups = standings
            .rank_groups(&[Tiebreaker::RawPoints, Tiebreaker::HeadToHead])
            .unwrap();
        assert_eq!(groups[0].user_ids, vec![first]);
        assert_eq!(groups[1].user_ids, vec![second]);
        assert_eq!(groups[2].user_ids, vec![third]);
    }

    #[test]
    fn three_player_points_group_is_split_by_summed_head_to_head() {
        let first = player(31);
        let second = player(32);
        let third = player(33);
        let fourth = player(34);
        let fifth = player(35);
        let sixth = player(36);
        let mut standings = LegacyStandings::default();
        standings
            .add_result(first, second, TournamentGameResult::Winner(Color::White))
            .unwrap();
        standings
            .add_result(first, third, TournamentGameResult::Winner(Color::White))
            .unwrap();
        standings
            .add_result(second, third, TournamentGameResult::Winner(Color::White))
            .unwrap();
        standings
            .add_result(second, fourth, TournamentGameResult::Winner(Color::White))
            .unwrap();
        standings
            .add_result(third, fifth, TournamentGameResult::Winner(Color::White))
            .unwrap();
        standings
            .add_result(third, sixth, TournamentGameResult::Winner(Color::White))
            .unwrap();

        let groups = standings
            .rank_groups(&[Tiebreaker::RawPoints, Tiebreaker::HeadToHead])
            .unwrap();
        assert_eq!(groups[0].user_ids, vec![first]);
        assert_eq!(groups[1].user_ids, vec![second]);
        assert_eq!(groups[2].user_ids, vec![third]);
    }

    #[test]
    fn head_to_head_precedes_conflicting_later_wins_as_black() {
        let first = player(41);
        let second = player(42);
        let third = player(43);
        let mut standings = LegacyStandings::default();
        standings
            .add_result(first, second, TournamentGameResult::Winner(Color::White))
            .unwrap();
        standings
            .add_result(third, second, TournamentGameResult::Winner(Color::Black))
            .unwrap();

        let groups = standings
            .rank_groups(&[Tiebreaker::HeadToHead, Tiebreaker::WinsAsBlack])
            .unwrap();
        assert_eq!(groups[0].user_ids, vec![first]);
        assert_eq!(groups[1].user_ids, vec![second]);
        assert_eq!(groups[2].user_ids, vec![third]);
    }

    #[test]
    fn equal_scores_use_competition_ranks_and_ignore_display_order() {
        let first = player(11);
        let second = player(12);
        let third = player(13);
        let mut standings = LegacyStandings::default();
        standings
            .add_result(first, second, TournamentGameResult::Draw)
            .unwrap();
        standings
            .add_result(first, third, TournamentGameResult::Winner(Color::White))
            .unwrap();
        standings
            .add_result(second, third, TournamentGameResult::Winner(Color::White))
            .unwrap();

        let groups = standings.rank_groups(&[Tiebreaker::RawPoints]).unwrap();
        assert_eq!(groups[0].competition_rank, 1);
        assert_eq!(groups[0].user_ids, vec![first, second]);
        assert_eq!(groups[1].competition_rank, 3);
        assert_eq!(groups[1].user_ids, vec![third]);
    }

    #[test]
    fn unsupported_legacy_criterion_fails_closed() {
        let mut standings = LegacyStandings::default();
        standings
            .add_result(player(21), player(22), TournamentGameResult::Draw)
            .unwrap();
        assert!(standings.rank_groups(&[Tiebreaker::Buchholz]).is_err());
    }
}
