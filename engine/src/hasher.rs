use lazy_static::lazy_static;

use crate::{bug_stack::BugStack, piece::Piece, position::Rotation};

lazy_static! {
    static ref BLACK_TO_MOVE: u64 = 0x2d358dccaa6c78a5_u64;
    static ref STUNNED: u64 = 0xc2b2ae3d_u64;
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Hasher {
    pub hash: u64,
    pub hashes: [u64; 2],
    pub stunned: Option<Piece>,
}

impl Hasher {
    pub fn new() -> Self {
        Self {
            hash: 0,
            hashes: [0, 0],
            stunned: None,
        }
    }

    pub fn clear(&mut self, turn: usize) {
        let black_to_move = if turn.is_multiple_of(2) {
            *BLACK_TO_MOVE
        } else {
            0
        };
        self.hash = 0;
        self.hashes = [black_to_move, black_to_move];
        self.stunned = None;
    }

    pub fn update(&mut self, bug_stack: &BugStack, index: Option<u32>, revolution: Rotation) {
        if bug_stack.is_empty() {
            return;
        }
        let index = if let Some(index) = index {
            index as u64
        } else if let Some(index) = bug_stack.index[revolution as usize] {
            index as u64
        } else {
            panic!("We need an index");
        };
        self.hashes[revolution as usize] ^= Self::hash((index << 32) | bug_stack.simple() as u64);
    }

    // This implements wyhash
    pub fn hash(input: u64) -> u64 {
        let input = input + 0xa0761d6478bd642f_u64;
        let output: u128 = input as u128 * (input ^ 0xe7037ed1a0b428db_u64) as u128;
        ((output >> 64) ^ output) as u64
    }

    /// For "pass" set stunned to None
    pub fn finish_turn(&mut self, stunned: Option<Piece>) -> u64 {
        for i in [0, 1] {
            self.hashes[i] ^= *BLACK_TO_MOVE;
            if let Some(stunned) = self.stunned {
                self.hashes[i] ^= Self::hash(*STUNNED * stunned.simple() as u64);
            }
            if let Some(stunned) = stunned {
                self.hashes[i] ^= Self::hash(*STUNNED * stunned.simple() as u64);
            }
        }
        self.stunned = stunned;
        self.hash = *self.hashes.iter().min().unwrap();
        self.hash
    }
}
