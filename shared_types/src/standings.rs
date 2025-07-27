use crate::{ScoringMode, Tiebreaker, TournamentGameResult};
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
    pub scoring_mode: ScoringMode,
}

impl Standings {
    pub fn new(scoring_mode: ScoringMode) -> Self {
        Self {
            players: HashSet::new(),
            players_scores: HashMap::new(),
            pairings: HashMap::new(),
            tiebreakers: vec![Tiebreaker::RawPoints],
            players_standings: Vec::new(),
            scoring_mode,
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
        match self.scoring_mode {
            ScoringMode::Game => self.get_game_points(player),
            ScoringMode::Match => self.get_match_points(player),
        }
    }

    fn get_game_points(&self, player: Uuid) -> f32 {
        let mut points = 0.0;
        if let Some(pairings) = self.pairings.get(&player) {
            for pairing in pairings {
                let pairing_points = match pairing.result {
                    TournamentGameResult::Draw => {
                        0.5
                    }
                    TournamentGameResult::Winner(Color::White) => {
                        if pairing.white_uuid == player {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    TournamentGameResult::Winner(Color::Black) => {
                        if pairing.black_uuid == player {
                            1.0
                        } else {
                            0.0
                        }
                    }
                                         _ => 0.0
                 };
                 points += pairing_points;
            }
        }
        points
    }

    fn get_match_points(&self, player: Uuid) -> f32 {
        let mut match_points = 0.0;
        let mut opponents = self.players.clone();
        opponents.remove(&player);
        
        for opponent in opponents {
            let pairings = self.pairings_between(player, opponent);
            if pairings.is_empty() {
                continue;
            }
            
            let mut player_wins = 0;
            let mut opponent_wins = 0;
            
            for pairing in pairings {
                match pairing.result {
                    TournamentGameResult::Winner(Color::White) => {
                        if pairing.white_uuid == player {
                            player_wins += 1;
                        } else {
                            opponent_wins += 1;
                        }
                    }
                    TournamentGameResult::Winner(Color::Black) => {
                        if pairing.black_uuid == player {
                            player_wins += 1;
                        } else {
                            opponent_wins += 1;
                        }
                    }
                    TournamentGameResult::Draw => {
                        // Draws don't count toward match wins
                    }
                    _ => {}
                }
            }
            
            // Award match points based on who won more games
            if player_wins > opponent_wins {
                match_points += 1.0;
            } else if player_wins == opponent_wins {
                match_points += 0.5;
            }
            // If opponent_wins > player_wins, player gets 0 points for this match
        }
        
        match_points
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
        Self::new(ScoringMode::Game)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tests_raw_points() {
        let mut s = Standings::new(ScoringMode::Game);
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
        let mut s = Standings::new(ScoringMode::Game);
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
        let mut s = Standings::new(ScoringMode::Game);
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
        let mut s = Standings::new(ScoringMode::Game);
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
        let mut s = Standings::new(ScoringMode::Game);
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
        let mut s = Standings::new(ScoringMode::Game);
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
    fn tests_match_scoring_clear_winner() {
        let mut s = Standings::new(ScoringMode::Match);
        let one = Uuid::new_v4();
        let one_elo = 100.0;
        let two = Uuid::new_v4();
        let two_elo = 200.0;
        
        // Player one wins 2 games, player two wins 1 game
        s.add_result(one, two, one_elo, two_elo, TournamentGameResult::Winner(Color::White));
        s.add_result(two, one, two_elo, one_elo, TournamentGameResult::Winner(Color::Black));
        s.add_result(one, two, one_elo, two_elo, TournamentGameResult::Winner(Color::White));
        
        s.enforce_tiebreakers();
        
        // Player one should win the match and get 1 point
        assert_eq!(s.get_raw_points(one), 1.0);
        // Player two should lose the match and get 0 points
        assert_eq!(s.get_raw_points(two), 0.0);
    }

    #[test]
    fn tests_match_scoring_tied_match() {
        let mut s = Standings::new(ScoringMode::Match);
        let one = Uuid::new_v4();
        let one_elo = 100.0;
        let two = Uuid::new_v4();
        let two_elo = 200.0;
        
        // Each player wins 1 game, with 1 draw
        s.add_result(one, two, one_elo, two_elo, TournamentGameResult::Winner(Color::White));
        s.add_result(two, one, two_elo, one_elo, TournamentGameResult::Winner(Color::White));
        s.add_result(one, two, one_elo, two_elo, TournamentGameResult::Draw);
        
        s.enforce_tiebreakers();
        
        // Both players should get 0.5 points for tied match
        assert_eq!(s.get_raw_points(one), 0.5);
        assert_eq!(s.get_raw_points(two), 0.5);
    }

    #[test]
    fn tests_match_scoring_vs_game_scoring() {
        let one = Uuid::new_v4();
        let one_elo = 100.0;
        let two = Uuid::new_v4();
        let two_elo = 200.0;
        
        // Test with game scoring
        let mut game_standings = Standings::new(ScoringMode::Game);
        game_standings.add_result(one, two, one_elo, two_elo, TournamentGameResult::Winner(Color::White));  // one wins as white
        game_standings.add_result(one, two, one_elo, two_elo, TournamentGameResult::Winner(Color::Black));  // two wins as black (one is white, two is black)
        game_standings.add_result(one, two, one_elo, two_elo, TournamentGameResult::Draw);
        game_standings.enforce_tiebreakers();
        
        // In game scoring: one gets 1.5 points (1 win as white + 0.5 draw), two gets 1.5 points (1 win as black + 0.5 draw)
        assert_eq!(game_standings.get_raw_points(one), 1.5);
        assert_eq!(game_standings.get_raw_points(two), 1.5);
        
        // Test with match scoring
        let mut match_standings = Standings::new(ScoringMode::Match);
        match_standings.add_result(one, two, one_elo, two_elo, TournamentGameResult::Winner(Color::White));  // one wins as white
        match_standings.add_result(one, two, one_elo, two_elo, TournamentGameResult::Winner(Color::Black));  // two wins as black
        match_standings.add_result(one, two, one_elo, two_elo, TournamentGameResult::Draw);
        match_standings.enforce_tiebreakers();
        
        // In match scoring: both get 0.5 points (tied 1-1, draw doesn't count)
        assert_eq!(match_standings.get_raw_points(one), 0.5);
        assert_eq!(match_standings.get_raw_points(two), 0.5);
    }

    #[test]
    fn tests_match_scoring_three_players() {
        let mut s = Standings::new(ScoringMode::Match);
        let one = Uuid::new_v4();
        let two = Uuid::new_v4();
        let three = Uuid::new_v4();
        let elo = 1500.0;
        
        // Player one vs two: one wins the match 2-1
        s.add_result(one, two, elo, elo, TournamentGameResult::Winner(Color::White));
        s.add_result(one, two, elo, elo, TournamentGameResult::Winner(Color::Black)); // two wins
        s.add_result(one, two, elo, elo, TournamentGameResult::Winner(Color::White));
        
        // Player one vs three: three wins the match 2-0
        s.add_result(one, three, elo, elo, TournamentGameResult::Winner(Color::Black)); // three wins
        s.add_result(one, three, elo, elo, TournamentGameResult::Winner(Color::Black)); // three wins
        
        // Player two vs three: tied match 1-1
        s.add_result(two, three, elo, elo, TournamentGameResult::Winner(Color::White)); // two wins
        s.add_result(two, three, elo, elo, TournamentGameResult::Winner(Color::Black)); // three wins
        
        s.enforce_tiebreakers();
        
        // one: 1 match win vs two, 0 match wins vs three = 1.0 points
        assert_eq!(s.get_raw_points(one), 1.0);
        // two: 0 match wins vs one, 0.5 match points vs three = 0.5 points  
        assert_eq!(s.get_raw_points(two), 0.5);
        // three: 1 match win vs one, 0.5 match points vs two = 1.5 points
        assert_eq!(s.get_raw_points(three), 1.5);
    }

    #[test]
    fn tests_match_scoring_all_draws() {
        let mut s = Standings::new(ScoringMode::Match);
        let one = Uuid::new_v4();
        let two = Uuid::new_v4();
        let elo = 1500.0;
        
        // All games are draws
        s.add_result(one, two, elo, elo, TournamentGameResult::Draw);
        s.add_result(one, two, elo, elo, TournamentGameResult::Draw);
        s.add_result(one, two, elo, elo, TournamentGameResult::Draw);
        
        s.enforce_tiebreakers();
        
        // In match scoring with all draws, both players get 0.5 points (tied 0-0)
        assert_eq!(s.get_raw_points(one), 0.5);
        assert_eq!(s.get_raw_points(two), 0.5);
    }

    #[test]
    fn tests_match_scoring_no_games() {
        let s = Standings::new(ScoringMode::Match);
        let one = Uuid::new_v4();
        
        // Player with no games should have 0 points
        assert_eq!(s.get_raw_points(one), 0.0);
    }

    #[test]
    fn tests_match_scoring_asymmetric_games() {
        let mut s = Standings::new(ScoringMode::Match);
        let one = Uuid::new_v4();
        let two = Uuid::new_v4();
        let elo = 1500.0;
        
        // Player one wins 3 games, player two wins 1 game
        s.add_result(one, two, elo, elo, TournamentGameResult::Winner(Color::White));
        s.add_result(one, two, elo, elo, TournamentGameResult::Winner(Color::White));
        s.add_result(one, two, elo, elo, TournamentGameResult::Winner(Color::White));
        s.add_result(one, two, elo, elo, TournamentGameResult::Winner(Color::Black)); // two wins
        
        s.enforce_tiebreakers();
        
        // Player one wins the match 3-1
        assert_eq!(s.get_raw_points(one), 1.0);
        assert_eq!(s.get_raw_points(two), 0.0);
    }

    #[test]
    fn tests_sonneborn_berger_with_match_scoring() {
        let mut s = Standings::new(ScoringMode::Match);
        s.add_tiebreaker(Tiebreaker::SonnebornBerger);
        
        let one = Uuid::new_v4();
        let two = Uuid::new_v4();
        let three = Uuid::new_v4();
        let elo = 1500.0;
        
        // Set up a scenario where Sonneborn-Berger matters
        // one beats two, three beats one, two beats three
        s.add_result(one, two, elo, elo, TournamentGameResult::Winner(Color::White));
        s.add_result(one, three, elo, elo, TournamentGameResult::Winner(Color::Black)); // three wins
        s.add_result(two, three, elo, elo, TournamentGameResult::Winner(Color::White));
        
        s.enforce_tiebreakers();
        
        // Each player should have 1 match point
        assert_eq!(s.get_raw_points(one), 1.0);
        assert_eq!(s.get_raw_points(two), 1.0);
        assert_eq!(s.get_raw_points(three), 1.0);
        
        // Sonneborn-Berger should be calculated correctly
        // Each opponent has 1 point, so each player should have 1.0 SB points
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
        assert_eq!(
            *s.players_scores
                .get(&three)
                .unwrap()
                .get(&Tiebreaker::SonnebornBerger)
                .unwrap(),
            1.0
        );
    }

    #[test]
    fn tests_match_scoring_single_game_matches() {
        let mut s = Standings::new(ScoringMode::Match);
        let one = Uuid::new_v4();
        let two = Uuid::new_v4();
        let three = Uuid::new_v4();
        let elo = 1500.0;
        
        // Each pair plays only one game
        s.add_result(one, two, elo, elo, TournamentGameResult::Winner(Color::White)); // one beats two
        s.add_result(one, three, elo, elo, TournamentGameResult::Winner(Color::Black)); // three beats one
        s.add_result(two, three, elo, elo, TournamentGameResult::Draw); // draw between two and three
        
        s.enforce_tiebreakers();
        
        // one: wins vs two (1), loses vs three (0) = 1.0 points
        assert_eq!(s.get_raw_points(one), 1.0);
        // two: loses vs one (0), draws vs three (0.5) = 0.5 points
        assert_eq!(s.get_raw_points(two), 0.5);
        // three: wins vs one (1), draws vs two (0.5) = 1.5 points
        assert_eq!(s.get_raw_points(three), 1.5);
    }
}
