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
pub enum Pairing {
    Encounter {
        white_uuid: Uuid,
        black_uuid: Uuid,
        white_elo: f64,
        black_elo: f64,
        result: TournamentGameResult,
    },
    Bye {
        player_uuid: Uuid,
    },
}

impl Pairing {
    pub fn other(&self, player: Uuid) -> Option<Uuid> {
        match self {
            Pairing::Encounter {
                white_uuid,
                black_uuid,
                ..
            } => {
                if *white_uuid == player {
                    return Some(*black_uuid);
                }
                if *black_uuid == player {
                    return Some(*white_uuid);
                }
                None
            }
            Pairing::Bye { .. } => None,
        }
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
                Tiebreaker::Buchholz => self.buchholz(),
                Tiebreaker::BuchholzCut1 => self.buchholz_cut1(),
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
                if let Pairing::Encounter {
                    white_uuid,
                    black_uuid,
                    result,
                    ..
                } = pairing
                {
                    let mut opponent_points = 0.0;
                    if let Some(scores) = self.players_scores.get(&opponent) {
                        if let Some(op) = scores.get(&Tiebreaker::RawPoints) {
                            opponent_points = *op;
                        }
                    }
                    match result {
                        TournamentGameResult::Draw => {
                            points += 0.5 * opponent_points;
                        }
                        TournamentGameResult::Winner(Color::White) if white_uuid == player => {
                            points += opponent_points;
                        }
                        TournamentGameResult::Winner(Color::Black) if black_uuid == player => {
                            points += opponent_points;
                        }
                        _ => {}
                    }
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
                if let Pairing::Encounter {
                    black_uuid, result, ..
                } = pairing
                {
                    if *black_uuid == black && *result == TournamentGameResult::Winner(Color::Black)
                    {
                        wins += 1.0;
                    }
                }
            }
        }
        wins
    }

    pub fn get_finished_games(&self, player: &Uuid) -> i32 {
        let mut finished = 0;
        if let Some(pairings) = self.pairings.get(player) {
            for pairing in pairings {
                match pairing {
                    Pairing::Encounter { result, .. } => {
                        if *result != TournamentGameResult::Unknown {
                            finished += 1;
                        }
                    }
                    Pairing::Bye { .. } => {
                        finished += 1; // BYEs count as finished games
                    }
                }
            }
        }
        finished
    }

    pub fn head_to_head_pair(&self, one: Uuid, two: Uuid) -> (f32, f32) {
        let mut results = HashMap::new();
        let pairings = self.pairings_between(one, two);
        for pairing in pairings {
            match pairing {
                Pairing::Encounter {
                    white_uuid,
                    black_uuid,
                    result,
                    ..
                } => match result {
                    TournamentGameResult::Unknown | TournamentGameResult::DoubeForfeit => {}
                    TournamentGameResult::Draw => {
                        *results.entry(white_uuid).or_default() += 0.5;
                        *results.entry(black_uuid).or_default() += 0.5;
                    }
                    TournamentGameResult::Winner(Color::White) => {
                        *results.entry(white_uuid).or_default() += 1.0;
                    }
                    TournamentGameResult::Winner(Color::Black) => {
                        *results.entry(black_uuid).or_default() += 1.0;
                    }
                    TournamentGameResult::Bye => {}
                },
                Pairing::Bye { .. } => {} // BYEs don't affect head-to-head
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
                if let Pairing::Encounter {
                    black_uuid,
                    white_uuid,
                    ..
                } = pairing
                {
                    if *black_uuid == two || *white_uuid == two {
                        results.push((*pairing).clone())
                    }
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
                match pairing {
                    Pairing::Encounter {
                        white_uuid,
                        black_uuid,
                        result,
                        ..
                    } => match result {
                        TournamentGameResult::Draw => {
                            points += 0.5;
                        }
                        TournamentGameResult::Winner(Color::White) => {
                            if *white_uuid == player {
                                points += 1.0;
                            }
                        }
                        TournamentGameResult::Winner(Color::Black) => {
                            if *black_uuid == player {
                                points += 1.0;
                            }
                        }
                        _ => {}
                    },
                    Pairing::Bye { .. } => {
                        points += 1.0; // BYE gives 1 point
                    }
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
        let pairing = Pairing::Encounter {
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

    pub fn add_bye(&mut self, player_uuid: Uuid) {
        self.players.insert(player_uuid);
        let pairing = Pairing::Bye { player_uuid };
        self.pairings.entry(player_uuid).or_default().push(pairing);
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

    pub fn buchholz(&mut self) {
        for player in &self.players {
            let buchholz = self.get_buchholz(*player);
            self.players_scores
                .entry(*player)
                .or_default()
                .entry(Tiebreaker::Buchholz)
                .and_modify(|w| *w = buchholz)
                .or_insert(buchholz);
        }
    }

    fn get_buchholz(&self, player: Uuid) -> f32 {
        let mut buchholz = 0.0;
        if let Some(pairings) = self.pairings.get(&player) {
            for pairing in pairings {
                match pairing {
                    Pairing::Encounter {
                        white_uuid,
                        black_uuid,
                        ..
                    } => {
                        let opponent = if *white_uuid == player {
                            *black_uuid
                        } else {
                            *white_uuid
                        };
                        if let Some(scores) = self.players_scores.get(&opponent) {
                            if let Some(points) = scores.get(&Tiebreaker::RawPoints) {
                                buchholz += points;
                            }
                        }
                    }
                    Pairing::Bye { .. } => {
                        // BYEs don't contribute to Buchholz
                    }
                }
            }
        }
        buchholz
    }

    pub fn buchholz_cut1(&mut self) {
        for player in &self.players {
            let buchholz = self.get_buchholz_cut1(*player);
            self.players_scores
                .entry(*player)
                .or_default()
                .entry(Tiebreaker::BuchholzCut1)
                .and_modify(|w| *w = buchholz)
                .or_insert(buchholz);
        }
    }

    fn get_buchholz_cut1(&self, player: Uuid) -> f32 {
        let mut opponent_scores = Vec::new();
        if let Some(pairings) = self.pairings.get(&player) {
            for pairing in pairings {
                match pairing {
                    Pairing::Encounter {
                        white_uuid,
                        black_uuid,
                        ..
                    } => {
                        let opponent = if *white_uuid == player {
                            *black_uuid
                        } else {
                            *white_uuid
                        };
                        if let Some(scores) = self.players_scores.get(&opponent) {
                            if let Some(points) = scores.get(&Tiebreaker::RawPoints) {
                                opponent_scores.push(*points);
                            }
                        }
                    }
                    Pairing::Bye { .. } => {
                        // BYEs don't contribute to Buchholz Cut 1
                    }
                }
            }
        }

        // If we have less than 2 opponents, use regular Buchholz
        if opponent_scores.len() < 2 {
            return self.get_buchholz(player);
        }

        // Sort scores and remove lowest
        opponent_scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
        opponent_scores.remove(0); // Remove lowest

        // Sum remaining scores
        opponent_scores.iter().sum()
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

    #[test]
    fn tests_buchholz() {
        let mut s = Standings::new();

        // Create three players with different scores
        let one = Uuid::new_v4();
        let two = Uuid::new_v4();
        let three = Uuid::new_v4();

        // Add some results to establish scores
        s.add_result(
            one,
            two,
            100.0,
            200.0,
            TournamentGameResult::Winner(Color::White),
        );
        s.add_result(
            two,
            three,
            200.0,
            300.0,
            TournamentGameResult::Winner(Color::White),
        );
        s.add_result(
            three,
            one,
            300.0,
            100.0,
            TournamentGameResult::Winner(Color::White),
        );

        // First calculate raw points for all players
        s.raw_points();

        // Then calculate Buchholz (which uses raw points)
        s.buchholz();

        // Player one played against two (1 point) and three (1 point)
        assert_eq!(
            *s.players_scores
                .get(&one)
                .unwrap()
                .get(&Tiebreaker::Buchholz)
                .unwrap(),
            2.0
        );

        // Player two played against one (1 point) and three (1 point)
        assert_eq!(
            *s.players_scores
                .get(&two)
                .unwrap()
                .get(&Tiebreaker::Buchholz)
                .unwrap(),
            2.0
        );

        // Player three played against two (1 point) and one (1 point)
        assert_eq!(
            *s.players_scores
                .get(&three)
                .unwrap()
                .get(&Tiebreaker::Buchholz)
                .unwrap(),
            2.0
        );
    }

    #[test]
    fn tests_buchholz_with_bye() {
        let mut s = Standings::new();

        let one = Uuid::new_v4();
        let two = Uuid::new_v4();

        // Add a game and a bye
        s.add_result(
            one,
            two,
            100.0,
            200.0,
            TournamentGameResult::Winner(Color::White),
        );
        s.add_bye(one);

        // First calculate raw points for all players
        s.raw_points();

        // Then calculate Buchholz (which uses raw points)
        s.buchholz();

        // Player one played against two (0 points) and had a bye (not counted)
        assert_eq!(
            *s.players_scores
                .get(&one)
                .unwrap()
                .get(&Tiebreaker::Buchholz)
                .unwrap(),
            0.0
        );

        // Player two only played against one (2 points)
        assert_eq!(
            *s.players_scores
                .get(&two)
                .unwrap()
                .get(&Tiebreaker::Buchholz)
                .unwrap(),
            2.0
        );
    }

    #[test]
    fn tests_buchholz_cut1() {
        let mut s = Standings::new();

        // Create four players with different scores
        let one = Uuid::new_v4();
        let two = Uuid::new_v4();
        let three = Uuid::new_v4();
        let four = Uuid::new_v4();

        // Add results to establish scores
        s.add_result(
            one,
            two,
            100.0,
            200.0,
            TournamentGameResult::Winner(Color::White),
        );
        s.add_result(
            one,
            three,
            100.0,
            300.0,
            TournamentGameResult::Winner(Color::White),
        );
        s.add_result(
            one,
            four,
            100.0,
            400.0,
            TournamentGameResult::Winner(Color::White),
        );

        // First calculate raw points for all players
        s.raw_points();

        // Then calculate Buchholz Cut 1 (which uses raw points)
        s.buchholz_cut1();

        // Player one played against:
        // - two (0 points)
        // - three (0 points)
        // - four (0 points)
        // Buchholz Cut 1 should drop the lowest score (0 points) and sum the rest (0 + 0 = 0)
        assert_eq!(
            *s.players_scores
                .get(&one)
                .unwrap()
                .get(&Tiebreaker::BuchholzCut1)
                .unwrap(),
            0.0
        );
    }

    #[test]
    fn tests_buchholz_cut1_fallback() {
        let mut s = Standings::new();

        let one = Uuid::new_v4();
        let two = Uuid::new_v4();

        // Add only one game (less than 2 opponents)
        s.add_result(
            one,
            two,
            100.0,
            200.0,
            TournamentGameResult::Winner(Color::White),
        );

        // First calculate raw points for all players
        s.raw_points();

        // Then calculate Buchholz Cut 1 (which uses raw points)
        s.buchholz_cut1();

        // With less than 2 opponents, should fall back to regular Buchholz
        // Player one played against two (0 points)
        assert_eq!(
            *s.players_scores
                .get(&one)
                .unwrap()
                .get(&Tiebreaker::BuchholzCut1)
                .unwrap(),
            0.0
        );
    }

    #[test]
    fn test_complex_tournament_buchholz() {
        let mut s = Standings::new();

        // Create 7 players (A through G)
        let player_a = Uuid::new_v4();
        let player_b = Uuid::new_v4();
        let player_c = Uuid::new_v4();
        let player_d = Uuid::new_v4();
        let player_e = Uuid::new_v4();
        let player_f = Uuid::new_v4();
        let player_g = Uuid::new_v4();

        // Set up a tournament with 11 games:

        // A beats B
        s.add_result(
            player_a,
            player_b,
            1000.0,
            1000.0,
            TournamentGameResult::Winner(Color::White),
        );

        // A beats C
        s.add_result(
            player_a,
            player_c,
            1000.0,
            1000.0,
            TournamentGameResult::Winner(Color::White),
        );

        // A draws with D
        s.add_result(
            player_a,
            player_d,
            1000.0,
            1000.0,
            TournamentGameResult::Draw,
        );

        // B beats E
        s.add_result(
            player_b,
            player_e,
            1000.0,
            1000.0,
            TournamentGameResult::Winner(Color::White),
        );

        // B beats F
        s.add_result(
            player_b,
            player_f,
            1000.0,
            1000.0,
            TournamentGameResult::Winner(Color::White),
        );

        // C beats D
        s.add_result(
            player_c,
            player_d,
            1000.0,
            1000.0,
            TournamentGameResult::Winner(Color::White),
        );

        // C beats G
        s.add_result(
            player_c,
            player_g,
            1000.0,
            1000.0,
            TournamentGameResult::Winner(Color::White),
        );

        // D beats E
        s.add_result(
            player_d,
            player_e,
            1000.0,
            1000.0,
            TournamentGameResult::Winner(Color::White),
        );

        // E beats F
        s.add_result(
            player_e,
            player_f,
            1000.0,
            1000.0,
            TournamentGameResult::Winner(Color::White),
        );

        // E beats G
        s.add_result(
            player_e,
            player_g,
            1000.0,
            1000.0,
            TournamentGameResult::Winner(Color::White),
        );

        // F beats G
        s.add_result(
            player_f,
            player_g,
            1000.0,
            1000.0,
            TournamentGameResult::Winner(Color::White),
        );

        // First calculate raw points
        s.raw_points();

        // Expected raw points based on the games:
        // A: 2.5 points (2 wins, 1 draw)
        // B: 2.0 points (2 wins, 1 loss)
        // C: 2.0 points (2 wins, 1 loss)
        // D: 1.5 points (1 win, 1 draw, 1 loss)
        // E: 2.0 points (2 wins, 2 losses)
        // F: 1.0 points (1 win, 2 losses)
        // G: 0.0 points (3 losses)

        // Verify raw points calculations
        assert_eq!(
            *s.players_scores
                .get(&player_a)
                .unwrap()
                .get(&Tiebreaker::RawPoints)
                .unwrap(),
            2.5
        );
        assert_eq!(
            *s.players_scores
                .get(&player_b)
                .unwrap()
                .get(&Tiebreaker::RawPoints)
                .unwrap(),
            2.0
        );
        assert_eq!(
            *s.players_scores
                .get(&player_c)
                .unwrap()
                .get(&Tiebreaker::RawPoints)
                .unwrap(),
            2.0
        );
        assert_eq!(
            *s.players_scores
                .get(&player_d)
                .unwrap()
                .get(&Tiebreaker::RawPoints)
                .unwrap(),
            1.5
        );
        assert_eq!(
            *s.players_scores
                .get(&player_e)
                .unwrap()
                .get(&Tiebreaker::RawPoints)
                .unwrap(),
            2.0
        );
        assert_eq!(
            *s.players_scores
                .get(&player_f)
                .unwrap()
                .get(&Tiebreaker::RawPoints)
                .unwrap(),
            1.0
        );
        assert_eq!(
            *s.players_scores
                .get(&player_g)
                .unwrap()
                .get(&Tiebreaker::RawPoints)
                .unwrap(),
            0.0
        );

        // Now calculate Buchholz scores
        s.buchholz();

        // Expected Buchholz scores (sum of opponents' raw points):
        // A: B(2.0) + C(2.0) + D(1.5) = 5.5
        // B: A(2.5) + E(2.0) + F(1.0) = 5.5
        // C: A(2.5) + D(1.5) + G(0.0) = 4.0
        // D: A(2.5) + C(2.0) + E(2.0) = 6.5
        // E: B(2.0) + D(1.5) + F(1.0) + G(0.0) = 4.5
        // F: B(2.0) + E(2.0) + G(0.0) = 4.0
        // G: C(2.0) + E(2.0) + F(1.0) = 5.0

        // Verify Buchholz calculations
        assert_eq!(
            *s.players_scores
                .get(&player_a)
                .unwrap()
                .get(&Tiebreaker::Buchholz)
                .unwrap(),
            5.5
        );
        assert_eq!(
            *s.players_scores
                .get(&player_b)
                .unwrap()
                .get(&Tiebreaker::Buchholz)
                .unwrap(),
            5.5
        );
        assert_eq!(
            *s.players_scores
                .get(&player_c)
                .unwrap()
                .get(&Tiebreaker::Buchholz)
                .unwrap(),
            4.0
        );
        assert_eq!(
            *s.players_scores
                .get(&player_d)
                .unwrap()
                .get(&Tiebreaker::Buchholz)
                .unwrap(),
            6.5
        );
        assert_eq!(
            *s.players_scores
                .get(&player_e)
                .unwrap()
                .get(&Tiebreaker::Buchholz)
                .unwrap(),
            4.5
        );
        assert_eq!(
            *s.players_scores
                .get(&player_f)
                .unwrap()
                .get(&Tiebreaker::Buchholz)
                .unwrap(),
            4.0
        );
        assert_eq!(
            *s.players_scores
                .get(&player_g)
                .unwrap()
                .get(&Tiebreaker::Buchholz)
                .unwrap(),
            5.0
        );

        // Now calculate Buchholz Cut 1 scores
        s.buchholz_cut1();

        // Expected Buchholz Cut 1 scores (sum of opponents' raw points minus lowest):
        // A: B(2.0) + C(2.0) + D(1.5) → drop D(1.5) = 4.0
        // B: A(2.5) + E(2.0) + F(1.0) → drop F(1.0) = 4.5
        // C: A(2.5) + D(1.5) + G(0.0) → drop G(0.0) = 4.0
        // D: A(2.5) + C(2.0) + E(2.0) → drop C(2.0) or E(2.0) = 4.5
        // E: B(2.0) + D(1.5) + F(1.0) + G(0.0) → drop G(0.0) = 4.5
        // F: B(2.0) + E(2.0) + G(0.0) → drop G(0.0) = 4.0
        // G: C(2.0) + E(2.0) + F(1.0) → drop F(1.0) = 4.0

        // Verify Buchholz Cut 1 calculations
        assert_eq!(
            *s.players_scores
                .get(&player_a)
                .unwrap()
                .get(&Tiebreaker::BuchholzCut1)
                .unwrap(),
            4.0
        );
        assert_eq!(
            *s.players_scores
                .get(&player_b)
                .unwrap()
                .get(&Tiebreaker::BuchholzCut1)
                .unwrap(),
            4.5
        );
        assert_eq!(
            *s.players_scores
                .get(&player_c)
                .unwrap()
                .get(&Tiebreaker::BuchholzCut1)
                .unwrap(),
            4.0
        );
        assert_eq!(
            *s.players_scores
                .get(&player_d)
                .unwrap()
                .get(&Tiebreaker::BuchholzCut1)
                .unwrap(),
            4.5
        );
        assert_eq!(
            *s.players_scores
                .get(&player_e)
                .unwrap()
                .get(&Tiebreaker::BuchholzCut1)
                .unwrap(),
            4.5
        );
        assert_eq!(
            *s.players_scores
                .get(&player_f)
                .unwrap()
                .get(&Tiebreaker::BuchholzCut1)
                .unwrap(),
            4.0
        );
        assert_eq!(
            *s.players_scores
                .get(&player_g)
                .unwrap()
                .get(&Tiebreaker::BuchholzCut1)
                .unwrap(),
            4.0
        );
    }
}
