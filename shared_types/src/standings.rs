use crate::{Tiebreaker, TournamentGameResult};
use hive_lib::Color;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    default::Default,
};
use uuid::Uuid;

pub type PlayerScores = HashMap<Tiebreaker, f32>;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Pairing {
    white_uuid: Uuid,
    black_uuid: Uuid,
    white_elo: f64,
    black_elo: f64,
    result: TournamentGameResult,
}

impl Pairing {
    pub fn other(&self, player: Uuid) -> Option<Uuid> {
        if self.white_uuid == player {
            return Some(self.black_uuid);
        }
        if self.black_uuid == player {
            return Some(self.white_uuid);
        }
        None
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Standings {
    pub players: HashSet<Uuid>,
    pub players_scores: HashMap<Uuid, PlayerScores>,
    pub pairings: HashMap<Uuid, Vec<Pairing>>,
    pub tiebreakers: Vec<Tiebreaker>,
    pub players_standings: Vec<Vec<Uuid>>,
}

impl Standings {
    pub fn new() -> Self {
        Self {
            players: HashSet::new(),
            players_scores: HashMap::new(),
            pairings: HashMap::new(),
            tiebreakers: vec![Tiebreaker::RawPoints],
            players_standings: Vec::new(),
        }
    }

    pub fn add_tiebreaker(&mut self, tiebreaker: Tiebreaker) {
        self.tiebreakers.push(tiebreaker);
    }

    pub fn update_standings(&mut self) {
        self.players_standings = self.standings_by_tiebreakers(self.tiebreakers.clone());
    }

    pub fn enforce_tiebreakers(&mut self) {
        for tiebreaker in self.tiebreakers.clone() {
            match tiebreaker {
                Tiebreaker::RawPoints => self.raw_points(),
                Tiebreaker::SonnebornBerger => self.sonneborn_berger(),
                Tiebreaker::WinsAsBlack => self.wins_as_black(),
                Tiebreaker::HeadToHead => self.head_to_head(),
            }
        }
        self.update_standings();
    }

    pub fn head_to_head(&mut self) {
        let mut h2h: HashMap<Uuid, f32> = HashMap::new();
        let tiebreakers = self
            .tiebreakers
            .clone()
            .into_iter()
            .unique()
            .collect::<Vec<_>>();
        let pos = tiebreakers
            .iter()
            .position(|t| *t == Tiebreaker::HeadToHead)
            .unwrap_or(0);

        let standings = self.standings_by_tiebreakers(tiebreakers[0..pos].to_vec());

        for players in standings.iter() {
            if players.len() > 1 {
                for combination in players.clone().into_iter().combinations(2) {
                    let (one, two) = (combination[0], combination[1]);
                    let (result_one, result_two) = self.head_to_head_pair(one, two);
                    *h2h.entry(one).or_default() += result_one;
                    *h2h.entry(two).or_default() += result_two;
                }
            }
            for player in players {
                self.players_scores
                    .entry(*player)
                    .or_default()
                    .entry(Tiebreaker::HeadToHead)
                    .and_modify(|v| *v = *h2h.get(player).unwrap_or(&0.0))
                    .or_insert(*h2h.get(player).unwrap_or(&0.0));
            }
        }
    }

    pub fn sonneborn_berger(&mut self) {
        for player in &self.players {
            let wins = self.get_sonneborn_berger(*player);
            self.players_scores
                .entry(*player)
                .or_default()
                .entry(Tiebreaker::SonnebornBerger)
                .and_modify(|w| *w = wins)
                .or_insert(wins);
        }
    }

    fn get_sonneborn_berger(&self, player: Uuid) -> f32 {
        let mut points = 0.0;
        let mut opponents = self.players.clone();
        opponents.remove(&player);
        for opponent in opponents {
            for pairing in self.pairings_between(player, opponent) {
                let mut opponent_points = 0.0;
                if let Some(scores) = self.players_scores.get(&opponent) {
                    if let Some(op) = scores.get(&Tiebreaker::RawPoints) {
                        opponent_points = *op;
                    }
                }
                match pairing.result {
                    TournamentGameResult::Draw => {
                        points += 0.5 * opponent_points;
                    }
                    TournamentGameResult::Winner(Color::White) if pairing.white_uuid == player => {
                        points += opponent_points;
                    }
                    TournamentGameResult::Winner(Color::Black) if pairing.black_uuid == player => {
                        points += opponent_points;
                    }
                    _ => {}
                }
            }
        }
        points
    }

    pub fn wins_as_black(&mut self) {
        for player in &self.players {
            let wins = self.get_wins_as_black(*player);
            self.players_scores
                .entry(*player)
                .or_default()
                .entry(Tiebreaker::WinsAsBlack)
                .and_modify(|w| *w = wins)
                .or_insert(wins);
        }
    }

    pub fn get_wins_as_black(&self, black: Uuid) -> f32 {
        let mut wins = 0.0;
        if let Some(pairings) = self.pairings.get(&black) {
            for pairing in pairings {
                if pairing.black_uuid == black
                    && pairing.result == TournamentGameResult::Winner(Color::Black)
                {
                    wins += 1.0;
                }
            }
        }
        wins
    }

    pub fn get_finished_games(&self, player: &Uuid) -> i32 {
        let mut finished = 0;
        if let Some(pairings) = self.pairings.get(player) {
            for pairing in pairings {
                if pairing.result != TournamentGameResult::Unknown {
                    finished += 1;
                }
            }
        }
        finished
    }

    pub fn head_to_head_pair(&self, one: Uuid, two: Uuid) -> (f32, f32) {
        let mut results = HashMap::new();
        let pairings = self.pairings_between(one, two);
        for pairing in pairings {
            match pairing.result {
                TournamentGameResult::Unknown | TournamentGameResult::DoubeForfeit => {}
                TournamentGameResult::Draw => {
                    *results.entry(pairing.white_uuid).or_default() += 0.5;
                    *results.entry(pairing.black_uuid).or_default() += 0.5;
                }
                TournamentGameResult::Winner(Color::White) => {
                    *results.entry(pairing.white_uuid).or_default() += 1.0;
                }
                TournamentGameResult::Winner(Color::Black) => {
                    *results.entry(pairing.black_uuid).or_default() += 1.0;
                }
            }
        }
        (
            *results.get(&one).unwrap_or(&0.0),
            *results.get(&two).unwrap_or(&0.0),
        )
    }

    pub fn pairings_between(&self, one: Uuid, two: Uuid) -> Vec<Pairing> {
        let mut results = Vec::new();
        if let Some(pairings) = self.pairings.get(&one) {
            for pairing in pairings {
                if pairing.black_uuid == two || pairing.white_uuid == two {
                    results.push((*pairing).clone())
                }
            }
        }
        results
    }

    pub fn raw_points(&mut self) {
        for player in &self.players {
            let wins = self.get_raw_points(*player);
            self.players_scores
                .entry(*player)
                .or_default()
                .entry(Tiebreaker::RawPoints)
                .and_modify(|w| *w = wins)
                .or_insert(wins);
        }
    }

    pub fn get_raw_points(&self, player: Uuid) -> f32 {
        let mut points = 0.0;
        if let Some(pairings) = self.pairings.get(&player) {
            for pairing in pairings {
                match pairing.result {
                    TournamentGameResult::Draw => {
                        points += 0.5;
                    }
                    TournamentGameResult::Winner(Color::White) => {
                        if pairing.white_uuid == player {
                            points += 1.0;
                        }
                    }
                    TournamentGameResult::Winner(Color::Black) => {
                        if pairing.black_uuid == player {
                            points += 1.0;
                        }
                    }
                    _ => {}
                }
            }
        }
        points
    }

    fn standings_by_tiebreakers(&self, tiebreakers: Vec<Tiebreaker>) -> Vec<Vec<Uuid>> {
        let mut scores = self
            .players
            .clone()
            .into_iter()
            .map(|player| {
                (
                    player,
                    tiebreakers
                        .iter()
                        .unique()
                        .map(|tiebreaker| {
                            *self
                                .players_scores
                                .get(&player)
                                .unwrap()
                                .get(tiebreaker)
                                .unwrap_or(&0.0)
                        })
                        .collect::<Vec<f32>>(),
                )
            })
            .collect::<Vec<(Uuid, Vec<f32>)>>();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores
            .into_iter()
            .chunk_by(|(_, score)| score.clone())
            .into_iter()
            .map(|(_, group)| group.map(|(uuid, _)| uuid).collect::<Vec<Uuid>>())
            .collect::<Vec<Vec<Uuid>>>()
    }

    pub fn add_result(
        &mut self,
        white_uuid: Uuid,
        black_uuid: Uuid,
        white_elo: f64,
        black_elo: f64,
        result: TournamentGameResult,
    ) {
        self.players.insert(white_uuid);
        self.players.insert(black_uuid);
        let pairing = Pairing {
            white_uuid,
            black_uuid,
            result: result.clone(),
            black_elo,
            white_elo,
        };
        self.pairings
            .entry(white_uuid)
            .or_default()
            .push(pairing.clone());
        self.pairings.entry(black_uuid).or_default().push(pairing);
    }

    pub fn results(&self) -> Vec<Vec<(Uuid, String, i32, PlayerScores)>> {
        let mut position = 0;
        println!("In results: Standings: {:?}", self.players_standings);
        self.players_standings
            .iter()
            .map(|group| {
                let mut first_in_group = true;
                group
                    .iter()
                    .map(|uuid| {
                        let finished = self.get_finished_games(uuid);
                        position += 1;
                        let position = if first_in_group {
                            first_in_group = false;
                            position.to_string()
                        } else {
                            String::from(" ")
                        };
                        (
                            *uuid,
                            position,
                            finished,
                            self.players_scores.get(uuid).unwrap().clone(),
                        )
                    })
                    .collect()
            })
            .collect()
    }
}

impl Default for Standings {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tests_raw_points() {
        let mut s = Standings::new();
        s.add_tiebreaker(Tiebreaker::WinsAsBlack);
        let one = Uuid::new_v4();
        let one_elo = 100.0;
        let two = Uuid::new_v4();
        let two_elo = 200.0;
        s.add_result(
            one,
            two,
            one_elo,
            two_elo,
            TournamentGameResult::Winner(Color::White),
        );
        s.add_result(
            two,
            one,
            two_elo,
            one_elo,
            TournamentGameResult::Winner(Color::Black),
        );
        assert_eq!(0, s.players_standings.len());
        s.enforce_tiebreakers();
        assert_eq!(vec![vec![one], vec![two]], s.players_standings);
    }

    #[test]
    fn tests_all_even() {
        let mut s = Standings::new();
        s.add_tiebreaker(Tiebreaker::HeadToHead);
        s.add_tiebreaker(Tiebreaker::WinsAsBlack);
        assert_eq!(
            vec![
                Tiebreaker::RawPoints,
                Tiebreaker::HeadToHead,
                Tiebreaker::WinsAsBlack
            ],
            s.tiebreakers
        );
        let one = Uuid::new_v4();
        let one_elo = 100.0;
        let two = Uuid::new_v4();
        let two_elo = 200.0;
        let three = Uuid::new_v4();
        let three_elo = 300.0;
        s.add_result(one, two, one_elo, two_elo, TournamentGameResult::Draw);
        s.add_result(one, three, one_elo, three_elo, TournamentGameResult::Draw);
        s.add_result(three, two, three_elo, two_elo, TournamentGameResult::Draw);
        assert_eq!(0, s.players_standings.len());
        s.enforce_tiebreakers();
        assert_eq!(1, s.players_standings.len());
    }

    #[test]
    fn tests_more_black_wins() {
        let mut s = Standings::new();
        s.add_tiebreaker(Tiebreaker::WinsAsBlack);
        let one = Uuid::new_v4();
        let one_elo = 100.0;
        let two = Uuid::new_v4();
        let two_elo = 200.0;
        s.add_result(
            one,
            two,
            one_elo,
            two_elo,
            TournamentGameResult::Winner(Color::White),
        );
        s.add_result(
            one,
            two,
            one_elo,
            two_elo,
            TournamentGameResult::Winner(Color::Black),
        );
        assert_eq!(0, s.players_standings.len());
        s.enforce_tiebreakers();
        assert_eq!(vec![vec![two], vec![one]], s.players_standings);
    }

    #[test]
    fn tests_sonneborn_berger() {
        let mut s = Standings::new();
        s.add_tiebreaker(Tiebreaker::SonnebornBerger);
        let one = Uuid::new_v4();
        let one_elo = 100.0;
        let two = Uuid::new_v4();
        let two_elo = 200.0;
        s.add_result(
            one,
            two,
            one_elo,
            two_elo,
            TournamentGameResult::Winner(Color::White),
        );
        s.add_result(
            one,
            two,
            one_elo,
            two_elo,
            TournamentGameResult::Winner(Color::Black),
        );
        s.add_result(one, two, one_elo, two_elo, TournamentGameResult::Draw);
        assert_eq!(0, s.players_standings.len());
        s.enforce_tiebreakers();
        assert_eq!(
            *s.players_scores
                .get(&one)
                .unwrap()
                .get(&Tiebreaker::SonnebornBerger)
                .unwrap(),
            2.25
        );
        assert_eq!(
            *s.players_scores
                .get(&two)
                .unwrap()
                .get(&Tiebreaker::SonnebornBerger)
                .unwrap(),
            2.25
        );
    }

    #[test]
    fn tests_head2head() {
        let mut s = Standings::new();
        s.add_tiebreaker(Tiebreaker::HeadToHead);
        let one = Uuid::new_v4();
        let one_elo = 100.0;
        let two = Uuid::new_v4();
        let two_elo = 200.0;
        s.add_result(
            one,
            two,
            one_elo,
            two_elo,
            TournamentGameResult::Winner(Color::White),
        );
        s.add_result(
            two,
            one,
            two_elo,
            one_elo,
            TournamentGameResult::Winner(Color::White),
        );
        assert_eq!(0, s.players_standings.len());
        s.enforce_tiebreakers();
        assert_eq!(
            *s.players_scores
                .get(&one)
                .unwrap()
                .get(&Tiebreaker::HeadToHead)
                .unwrap(),
            1.0
        );
        assert_eq!(
            *s.players_scores
                .get(&two)
                .unwrap()
                .get(&Tiebreaker::HeadToHead)
                .unwrap(),
            1.0
        );
    }

    #[test]
    fn tests_all_tiebreakers() {
        let mut s = Standings::new();
        s.add_tiebreaker(Tiebreaker::RawPoints);
        s.add_tiebreaker(Tiebreaker::HeadToHead);
        s.add_tiebreaker(Tiebreaker::WinsAsBlack);
        s.add_tiebreaker(Tiebreaker::SonnebornBerger);
        let one = Uuid::new_v4();
        let one_elo = 100.0;
        let two = Uuid::new_v4();
        let two_elo = 200.0;
        s.add_result(
            one,
            two,
            one_elo,
            two_elo,
            TournamentGameResult::Winner(Color::Black),
        );
        s.add_result(
            two,
            one,
            two_elo,
            one_elo,
            TournamentGameResult::Winner(Color::Black),
        );
        assert_eq!(0, s.players_standings.len());
        s.enforce_tiebreakers();
        assert_eq!(
            *s.players_scores
                .get(&one)
                .unwrap()
                .get(&Tiebreaker::RawPoints)
                .unwrap(),
            1.0
        );
        assert_eq!(
            *s.players_scores
                .get(&two)
                .unwrap()
                .get(&Tiebreaker::RawPoints)
                .unwrap(),
            1.0
        );
        assert_eq!(
            *s.players_scores
                .get(&one)
                .unwrap()
                .get(&Tiebreaker::HeadToHead)
                .unwrap(),
            1.0
        );
        assert_eq!(
            *s.players_scores
                .get(&two)
                .unwrap()
                .get(&Tiebreaker::HeadToHead)
                .unwrap(),
            1.0
        );
        assert_eq!(
            *s.players_scores
                .get(&one)
                .unwrap()
                .get(&Tiebreaker::WinsAsBlack)
                .unwrap(),
            1.0
        );
        assert_eq!(
            *s.players_scores
                .get(&two)
                .unwrap()
                .get(&Tiebreaker::WinsAsBlack)
                .unwrap(),
            1.0
        );
        assert_eq!(
            *s.players_scores
                .get(&one)
                .unwrap()
                .get(&Tiebreaker::SonnebornBerger)
                .unwrap(),
            1.0
        );
        assert_eq!(
            *s.players_scores
                .get(&two)
                .unwrap()
                .get(&Tiebreaker::SonnebornBerger)
                .unwrap(),
            1.0
        );
    }
}
