use crate::game::{Action, Game};
use hive_lib::GameResult;
use std::{
    collections::HashMap,
    hash::{BuildHasherDefault, Hasher},
};

/// Every prune here may only ever lose a puzzle, never invent one: a "no win" answer can be
/// wrong in the conservative direction (budget, unplayed queens), a "win" answer never.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfBudget;

#[derive(Default)]
struct KeyHasher(u64);

impl Hasher for KeyHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = self.0.rotate_left(8) ^ u64::from(*byte);
        }
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = value;
    }
}

#[derive(Default, Clone, Copy)]
struct Entry {
    win_within: u8,
    win_move: Option<Action>,
    no_win_within: u8,
    lose_within: u8,
    no_lose_within: u8,
}

const KILLER_BONUS: i32 = 1_000;
const MAX_TT_ENTRIES: usize = 4_000_000;
const MAX_PLY: usize = 32;

pub struct Prover {
    tt: HashMap<u64, Entry, BuildHasherDefault<KeyHasher>>,
    buffers: Vec<Vec<Action>>,
    scores: Vec<Vec<i32>>,
    killers: [[Option<Action>; 2]; MAX_PLY],
    pub nodes: u64,
    budget: u64,
}

impl Prover {
    pub fn new(budget: u64) -> Self {
        Self {
            tt: HashMap::default(),
            buffers: (0..MAX_PLY).map(|_| Vec::with_capacity(256)).collect(),
            scores: (0..MAX_PLY).map(|_| Vec::with_capacity(256)).collect(),
            killers: [[None; 2]; MAX_PLY],
            nodes: 0,
            budget,
        }
    }

    /// Starts a fresh node budget; the table survives, it stays valid across positions.
    pub fn reset_budget(&mut self, budget: u64) {
        self.nodes = 0;
        self.budget = budget;
        if self.tt.len() > MAX_TT_ENTRIES {
            self.tt.clear();
        }
    }

    /// Shortest forced win (odd plies, at most `max_plies`) for the side to move.
    pub fn shortest_win(
        &mut self,
        game: &mut Game,
        max_plies: u32,
    ) -> Result<Option<(u32, Action)>, OutOfBudget> {
        let mut plies = 1;
        while plies <= max_plies {
            if let Some(action) = self.win_within(game, plies, 0)? {
                return Ok(Some((plies, action)));
            }
            plies += 2;
        }
        Ok(None)
    }

    /// Every move that forces a win within `plies` for the side to move.
    pub fn winning_moves(
        &mut self,
        game: &mut Game,
        plies: u32,
    ) -> Result<Vec<Action>, OutOfBudget> {
        let attacker = game.to_move;
        let mut actions = Vec::new();
        if plies == 1 {
            game.immediate_wins(&mut actions);
            return Ok(actions);
        }
        game.actions(&mut actions);
        let mut winning = Vec::new();
        for action in actions {
            match game.result_after(&action) {
                GameResult::Winner(color) if color == attacker => winning.push(action),
                GameResult::Unknown if plies >= 3 => {
                    let undo = game.make(&action);
                    let lost = self.loses_within(game, plies - 1, 1);
                    game.unmake(undo);
                    if lost? {
                        winning.push(action);
                    }
                }
                _ => {}
            }
        }
        Ok(winning)
    }

    /// The defence that holds out longest, and the attacker's shortest win after it. The side to
    /// move must already be known to lose within `plies`.
    pub fn longest_defence(
        &mut self,
        game: &mut Game,
        plies: u32,
    ) -> Result<Option<(Action, u32)>, OutOfBudget> {
        let defender = game.to_move;
        let mut actions = Vec::new();
        game.actions(&mut actions);
        let mut best: Option<(Action, u32)> = None;
        for action in actions {
            let length = match game.result_after(&action) {
                GameResult::Winner(color) if color != defender => 0,
                GameResult::Unknown => {
                    let undo = game.make(&action);
                    let found = self.shortest_win(game, plies.saturating_sub(1));
                    game.unmake(undo);
                    match found? {
                        Some((length, _)) => length,
                        None => return Ok(None),
                    }
                }
                _ => return Ok(None),
            };
            let better = match best {
                None => true,
                Some((current, current_length)) => {
                    length > current_length
                        || (length == current_length
                            && current == Action::Pass
                            && action != Action::Pass)
                }
            };
            if better {
                best = Some((action, length));
            }
        }
        Ok(best)
    }

    fn tick(&mut self) -> Result<(), OutOfBudget> {
        self.nodes += 1;
        if self.nodes > self.budget {
            return Err(OutOfBudget);
        }
        Ok(())
    }

    fn entry(&self, key: u64) -> Entry {
        self.tt.get(&key).copied().unwrap_or_default()
    }

    fn win_within(
        &mut self,
        game: &mut Game,
        plies: u32,
        ply: usize,
    ) -> Result<Option<Action>, OutOfBudget> {
        self.tick()?;
        if ply + 1 >= MAX_PLY || game.is_over() {
            return Ok(None);
        }
        let key = game.key();
        let entry = self.entry(key);
        if entry.win_within != 0 && u32::from(entry.win_within) <= plies {
            return Ok(entry.win_move);
        }
        if u32::from(entry.no_win_within) >= plies {
            return Ok(None);
        }

        let mut actions = std::mem::take(&mut self.buffers[ply]);
        game.immediate_wins(&mut actions);
        let mut found = actions.first().copied();

        if found.is_none() && plies >= 3 {
            game.actions(&mut actions);
            self.order(game, &mut actions, ply, entry.win_move, true);
            for action in actions.iter().copied() {
                if game.result_after(&action) != GameResult::Unknown {
                    continue;
                }
                let undo = game.make(&action);
                let lost = self.loses_within(game, plies - 1, ply + 1);
                game.unmake(undo);
                if lost? {
                    found = Some(action);
                    self.remember_killer(ply, action);
                    break;
                }
            }
        }
        self.buffers[ply] = actions;

        let slot = self.tt.entry(key).or_default();
        match found {
            Some(action) => {
                if slot.win_within == 0 || u32::from(slot.win_within) > plies {
                    slot.win_within = plies as u8;
                    slot.win_move = Some(action);
                }
            }
            None => slot.no_win_within = slot.no_win_within.max(plies as u8),
        }
        Ok(found)
    }

    /// Whether the side to move loses, against best play, within `plies` (even).
    fn loses_within(
        &mut self,
        game: &mut Game,
        plies: u32,
        ply: usize,
    ) -> Result<bool, OutOfBudget> {
        self.tick()?;
        if ply + 1 >= MAX_PLY || game.is_over() {
            return Ok(false);
        }
        let key = game.key();
        let entry = self.entry(key);
        if entry.lose_within != 0 && u32::from(entry.lose_within) <= plies {
            return Ok(true);
        }
        if u32::from(entry.no_lose_within) >= plies {
            return Ok(false);
        }

        let defender = game.to_move;
        let own_queen = Game::queen(defender);
        let Some(queen_at) = game.queen_position(defender) else {
            return Ok(false);
        };
        // One defender move adds at most one neighbour, and the attacker's finishing move needs five.
        if plies == 2
            && queen_at
                .positions_around()
                .filter(|around| game.board.occupied(*around))
                .count()
                <= 3
            && !game.queen_throw_risk_after(defender, &Action::Pass)
        {
            let slot = self.tt.entry(key).or_default();
            slot.no_lose_within = slot.no_lose_within.max(plies as u8);
            return Ok(false);
        }
        let mut actions = std::mem::take(&mut self.buffers[ply]);
        game.actions(&mut actions);
        self.order(game, &mut actions, ply, None, false);

        let mut lost = true;
        for action in actions.iter().copied() {
            match game.result_after(&action) {
                GameResult::Winner(color) if color != defender => continue,
                GameResult::Unknown => {}
                _ => {
                    lost = false;
                    break;
                }
            }
            if plies == 2
                && game
                    .queen_neighbors_after(own_queen, &action)
                    .is_none_or(|neighbors| neighbors < 5)
                && !game.queen_throw_risk_after(defender, &action)
            {
                lost = false;
                self.remember_killer(ply, action);
                break;
            }
            let undo = game.make(&action);
            let win = self.win_within(game, plies - 1, ply + 1);
            game.unmake(undo);
            if win?.is_none() {
                lost = false;
                self.remember_killer(ply, action);
                break;
            }
        }
        self.buffers[ply] = actions;

        let slot = self.tt.entry(key).or_default();
        if lost {
            if slot.lose_within == 0 || u32::from(slot.lose_within) > plies {
                slot.lose_within = plies as u8;
            }
        } else {
            slot.no_lose_within = slot.no_lose_within.max(plies as u8);
        }
        Ok(lost)
    }

    fn remember_killer(&mut self, ply: usize, action: Action) {
        let killers = &mut self.killers[ply];
        if killers[0] != Some(action) {
            killers[1] = killers[0];
            killers[0] = Some(action);
        }
    }

    /// Philanthus's cheap queen-geometry ordering, split by role: the attacker wants kill spots,
    /// the defender wants the moves most likely to refute (queen walks, unblocking, counter-threats).
    fn order(
        &mut self,
        game: &Game,
        actions: &mut [Action],
        ply: usize,
        tt_move: Option<Action>,
        attacking: bool,
    ) {
        let mover = game.to_move;
        let own_queen = game.queen_position(mover);
        let enemy_queen = game.queen_position(mover.opposite_color());
        let killers = self.killers[ply];
        let mut scores = std::mem::take(&mut self.scores[ply]);
        scores.clear();
        for action in actions.iter() {
            let mut score = 0;
            if Some(*action) == tt_move {
                score += 10 * KILLER_BONUS;
            }
            if killers.contains(&Some(*action)) {
                score += KILLER_BONUS;
            }
            let touches = |at: Option<hive_lib::Position>, queen: Option<hive_lib::Position>| matches!((at, queen), (Some(at), Some(queen)) if at.is_neighbor(queen));
            let to = action.to();
            let from = action.from();
            if attacking {
                if touches(to, enemy_queen) {
                    score += 100;
                }
                if touches(to, own_queen) {
                    score -= 30;
                }
                if touches(from, own_queen) {
                    score += 20;
                }
                if touches(from, enemy_queen) {
                    score -= 60;
                }
            } else {
                if action.piece() == Some(Game::queen(mover)) {
                    score += 200;
                }
                if touches(from, own_queen) {
                    score += 120;
                }
                if touches(to, enemy_queen) {
                    score += 60;
                }
                if touches(to, own_queen) {
                    score -= 50;
                }
                if *action == Action::Pass {
                    score -= 500;
                }
            }
            scores.push(score);
        }
        let mut indexed: Vec<(i32, Action)> = scores
            .iter()
            .copied()
            .zip(actions.iter().copied())
            .collect();
        indexed.sort_by_key(|(score, _)| -score);
        for (slot, (_, action)) in actions.iter_mut().zip(indexed) {
            *slot = action;
        }
        self.scores[ply] = scores;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use hive_lib::{History, State};

    /// Positions from hivegame games with a forced win of a known length (philanthus fixtures).
    pub(crate) fn fixtures() -> Vec<(State, u32)> {
        include_str!("../test_data/forced_wins.csv")
            .lines()
            .filter_map(|line| {
                let tokens: Vec<&str> = line.split_whitespace().collect();
                let index = tokens.iter().position(|t| t.parse::<u32>().is_ok())?;
                let plies = tokens[index].parse().ok()?;
                let history = History::from_uhp_str(&tokens[..index].join(" ")).ok()?;
                Some((State::new_from_history(&history).ok()?, plies))
            })
            .collect()
    }

    #[test]
    fn proves_every_fixture_at_exactly_its_length() {
        let fixtures = fixtures();
        assert!(fixtures.len() >= 50);
        let mut prover = Prover::new(50_000_000);
        for (index, (state, plies)) in fixtures.iter().enumerate() {
            let mut game = Game::from_state(state);
            prover.reset_budget(50_000_000);
            let found = prover
                .shortest_win(&mut game, *plies)
                .expect("within budget");
            assert_eq!(
                found.map(|(length, _)| length),
                Some(*plies),
                "fixture {index}"
            );
        }
    }

    #[test]
    fn every_winning_move_wins() {
        let mut prover = Prover::new(50_000_000);
        for (state, plies) in fixtures().iter().take(20) {
            let mut game = Game::from_state(state);
            let winning = prover
                .winning_moves(&mut game, *plies)
                .expect("within budget");
            assert!(!winning.is_empty());
            if *plies == 1 {
                for action in winning {
                    assert_eq!(
                        game.result_after(&action),
                        GameResult::Winner(state.turn_color)
                    );
                }
            }
        }
    }
}
