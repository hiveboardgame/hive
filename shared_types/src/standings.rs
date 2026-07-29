use crate::Tiebreaker;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub type PlayerScores = HashMap<Tiebreaker, f32>;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct PlayerStanding {
    pub player: Uuid,
    /// Competition ranking: tied players share the lower number, and the next
    /// group skips ahead (1, 2, 2, 4).
    pub position: u32,
    pub games_played: i32,
    pub scores: PlayerScores,
}

/// Computed in `db` from the tournament's games, by `Tournament::standings`.
/// This type carries the answer; it no longer knows how to work it out.
#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct Standings {
    /// The order actually applied, primary score first.
    pub tiebreakers: Vec<Tiebreaker>,
    /// Best first; each inner `Vec` is a group the tiebreakers could not split.
    pub groups: Vec<Vec<PlayerStanding>>,
}

impl Standings {
    pub fn players(&self) -> impl Iterator<Item = &PlayerStanding> {
        self.groups.iter().flatten()
    }

    pub fn position_of(&self, player: Uuid) -> Option<u32> {
        self.players()
            .find(|standing| standing.player == player)
            .map(|standing| standing.position)
    }

    pub fn score(&self, player: Uuid, tiebreaker: Tiebreaker) -> Option<f32> {
        self.players()
            .find(|standing| standing.player == player)
            .and_then(|standing| standing.scores.get(&tiebreaker).copied())
    }

    /// Player ids best-first, grouped by tie — the shape the old `Standings`
    /// exposed as `players_standings`.
    pub fn ordered_groups(&self) -> Vec<Vec<Uuid>> {
        self.groups
            .iter()
            .map(|group| group.iter().map(|standing| standing.player).collect())
            .collect()
    }
}
