use hudsoni::{Hasher, Position};

use crate::game::Action;

const SIDE_TAG: u64 = 1 << 50;
const TURN_TAG: u64 = 2 << 50;
const STUN_TAG: u64 = 3 << 50;

fn square_index(position: Position) -> u64 {
    ((position.r & 31) * 32 + (position.q & 31)) as u64
}

pub fn square_key(position: Position, simple: u32) -> u64 {
    Hasher::hash((square_index(position) << 32) | simple as u64)
}

pub fn side_key() -> u64 {
    Hasher::hash(SIDE_TAG)
}

pub fn turn_key(turn: usize) -> u64 {
    Hasher::hash(TURN_TAG | (turn as u64 & 0xFFFF_FFFF))
}

pub fn stunned_key(offset: usize) -> u64 {
    Hasher::hash(STUN_TAG | offset as u64)
}

#[derive(Clone, Copy)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Clone, Copy)]
pub struct Entry {
    pub key: u64,
    pub depth: u32,
    pub score: i32,
    pub bound: Bound,
    pub best: Option<Action>,
}

pub struct TranspositionTable {
    entries: Vec<Option<Entry>>,
    mask: usize,
    disabled: bool,
}

impl TranspositionTable {
    pub fn new(bits: u32) -> Self {
        let size = 1_usize << bits;
        Self {
            entries: vec![None; size],
            mask: size - 1,
            disabled: false,
        }
    }

    #[cfg(test)]
    pub fn disabled() -> Self {
        Self {
            entries: Vec::new(),
            mask: 0,
            disabled: true,
        }
    }

    pub fn probe(&self, key: u64) -> Option<Entry> {
        if self.disabled {
            return None;
        }
        self.entries[key as usize & self.mask].filter(|entry| entry.key == key)
    }

    pub fn store(&mut self, entry: Entry) {
        if self.disabled {
            return;
        }
        let slot = &mut self.entries[entry.key as usize & self.mask];
        if slot
            .map(|existing| entry.depth >= existing.depth)
            .unwrap_or(true)
        {
            *slot = Some(entry);
        }
    }
}
