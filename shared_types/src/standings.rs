use crate::TournamentGameResult;
use hive_lib::Color;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, default::Default};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Pairing {
    white: Uuid,
    black: Uuid,
    result: TournamentGameResult,
}

impl Pairing {
    pub fn other(&self, player: Uuid) -> Option<Uuid> {
        if self.white == player {
            return Some(self.black);
        }
        if self.black == player {
            return Some(self.white);
        }
        None
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Standings {
    pub players_scores: HashMap<Uuid, f32>,
    pub pairings: HashMap<Uuid, Vec<Pairing>>,
}

impl Standings {
    pub fn new() -> Self {
        Self {
            players_scores: HashMap::new(),
            pairings: HashMap::new(),
        }
    }

    pub fn head_to_head(&self, players: Vec<Uuid>) -> HashMap<Uuid, i32> {
        HashMap::new()
    }

    pub fn pairings_between(&self, one: Uuid, two: Uuid) -> Vec<Pairing> {
        let mut results = Vec::new();
        if let Some(pairings) = self.pairings.get(&one) {
            for pairing in pairings {
                if pairing.black == two || pairing.white == two {
                    results.push((*pairing).clone())
                }
            }
        }
        results
    }

    pub fn head_to_head_pair(&self, one: Uuid, two: Uuid) -> HashMap<Uuid, i32> {
        let mut results = HashMap::new();
        let pairings = self.pairings_between(one, two);
        for pairing in pairings {
            match pairing.result {
                TournamentGameResult::Unknown
                | TournamentGameResult::DoubeForfeit
                | TournamentGameResult::Draw => {}
                TournamentGameResult::Winner(Color::White) => {
                    *results.entry(pairing.white).or_default() += 1;
                }
                TournamentGameResult::Winner(Color::Black) => {
                    *results.entry(pairing.black).or_default() += 1;
                }
            }
        }
        results
    }

    pub fn wins_as_black(&self, black: Uuid) -> i32 {
        let mut wins = 0;
        if let Some(pairings) = self.pairings.get(&black) {
            for pairing in pairings {
                if pairing.black == black
                    && pairing.result == TournamentGameResult::Winner(Color::Black)
                {
                    wins += 1;
                }
            }
        }
        wins
    }

    pub fn add_result(&mut self, white: Uuid, black: Uuid, result: TournamentGameResult) {
        self.pairings.entry(white).or_default().push(Pairing {
            white,
            black,
            result: result.clone(),
        });
        self.pairings.entry(black).or_default().push(Pairing {
            white,
            black,
            result: result.clone(),
        });
        match result {
            TournamentGameResult::Unknown | TournamentGameResult::DoubeForfeit => {}
            TournamentGameResult::Draw => {
                *self.players_scores.entry(white).or_default() += 0.5;
                *self.players_scores.entry(black).or_default() += 0.5;
            }
            TournamentGameResult::Winner(Color::White) => {
                *self.players_scores.entry(white).or_default() += 1.0
            }
            TournamentGameResult::Winner(Color::Black) => {
                *self.players_scores.entry(black).or_default() += 1.0
            }
        }
    }
}

impl Default for Standings {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for Standings {
    type Item = (Uuid, f32);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let mut vec: Vec<_> = self.players_scores.into_iter().collect();
        vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        vec.into_iter()
    }
}
