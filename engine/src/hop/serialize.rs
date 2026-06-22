use std::collections::HashSet;

use crate::{
    board::{Board, Stacks},
    color::Color,
    direction::Direction,
    game_type::GameType,
    piece::Piece,
    position::{Position, Rotation},
};

use super::{
    error::HopError,
    frame::{HOP_MINUS, HOP_PLUS},
};

/// Canonical HOP for a board, as `<game_type>,<topology>,<player>`.
pub fn from_position(board: &Board, game_type: GameType, to_move: Color) -> String {
    let topology = best_topology(board);
    let player = if to_move == Color::White { 'w' } else { 'b' };
    match serialize_game_type(game_type) {
        Some(prefix) => format!("{prefix},{topology},{player}"),
        None => format!("{topology},{player}"),
    }
}

/// Parse a HOP string and re-emit it in canonical form.
pub fn canonicalize(hop: &str) -> Result<String, HopError> {
    let parsed = super::parse(hop)?;
    Ok(from_position(
        &parsed.board,
        parsed.game_type,
        parsed.to_move,
    ))
}

/// Lexicographically smallest topology over every root and both chiralities (reflection-invariant).
fn best_topology(board: &Board) -> String {
    let cells: Vec<Position> = board.all_taken_positions().collect();
    let stacks = board.stacks();
    let headings = Direction::all();

    let mut best: Option<String> = None;
    for turns in CHIRALITIES {
        let renderer = Renderer {
            stacks: &stacks,
            turns,
            last_moved: board.last_moved,
        };
        for &start in &cells {
            for &heading in &headings {
                if let Some(topology) = renderer.topology(board, &cells, start, heading) {
                    if best.as_ref().is_none_or(|best| topology < *best) {
                        best = Some(topology);
                    }
                }
            }
        }
    }
    best.unwrap_or_default()
}

#[derive(Clone, Copy)]
struct Turns {
    plus: Rotation,
    minus: Rotation,
}

const CHIRALITIES: [Turns; 2] = [
    Turns {
        plus: HOP_PLUS,
        minus: HOP_MINUS,
    },
    Turns {
        plus: HOP_MINUS,
        minus: HOP_PLUS,
    },
];

struct Chain {
    nodes: Vec<(Position, Direction)>,
    branches: Vec<(usize, bool, Chain)>,
}

struct Walker<'a> {
    board: &'a Board,
    turns: Turns,
    visited: HashSet<Position>,
}

impl Walker<'_> {
    fn chain(&mut self, start: Position, heading: Direction) -> Chain {
        let nodes = self.follow_line(start, heading);
        let branches = self.collect_branches(&nodes);
        Chain { nodes, branches }
    }

    fn follow_line(&mut self, start: Position, heading: Direction) -> Vec<(Position, Direction)> {
        let mut nodes = vec![(start, heading)];
        self.visited.insert(start);
        let (mut cur, mut dir) = (start, heading);
        while let Some((next, next_dir)) = self.step(cur, dir) {
            nodes.push((next, next_dir));
            self.visited.insert(next);
            (cur, dir) = (next, next_dir);
        }
        nodes
    }

    fn step(&self, from: Position, dir: Direction) -> Option<(Position, Direction)> {
        [
            dir,
            dir.next_direction(self.turns.plus),
            dir.next_direction(self.turns.minus),
        ]
        .into_iter()
        .map(|d| (from.to(d), d))
        .find(|&(cell, _)| self.unvisited(cell))
    }

    fn collect_branches(&mut self, nodes: &[(Position, Direction)]) -> Vec<(usize, bool, Chain)> {
        let mut branches = Vec::new();
        for (index, &(cell, into)) in nodes.iter().enumerate() {
            for (rotation, is_plus) in [(self.turns.plus, true), (self.turns.minus, false)] {
                let dir = into.next_direction(rotation);
                let neighbour = cell.to(dir);
                if self.unvisited(neighbour) {
                    branches.push((index + 1, is_plus, self.chain(neighbour, dir)));
                }
            }
        }
        branches
    }

    fn unvisited(&self, cell: Position) -> bool {
        self.board.occupied(cell) && !self.visited.contains(&cell)
    }
}

struct Renderer<'a> {
    stacks: &'a Stacks,
    turns: Turns,
    last_moved: Option<(Piece, Position)>,
}

impl Renderer<'_> {
    /// `None` when this root cannot reach every cell using only single-step turns.
    fn topology(
        &self,
        board: &Board,
        cells: &[Position],
        start: Position,
        heading: Direction,
    ) -> Option<String> {
        let mut walker = Walker {
            board,
            turns: self.turns,
            visited: HashSet::new(),
        };
        let chain = walker.chain(start, heading);
        if walker.visited.len() != cells.len() {
            return None;
        }
        let mut out = String::new();
        self.chain(&chain, &mut out);
        Some(out)
    }

    fn chain(&self, chain: &Chain, out: &mut String) {
        self.main_line(chain, out);
        self.stacks_on(chain, out);
        self.branches(chain, out);
    }

    fn main_line(&self, chain: &Chain, out: &mut String) {
        let mut prev = chain.nodes[0].1;
        for (idx, &(pos, dir)) in chain.nodes.iter().enumerate() {
            if idx > 0 {
                self.push_turn(out, prev, dir);
            }
            self.push_piece(out, self.bottom(pos), pos);
            prev = dir;
        }
    }

    fn stacks_on(&self, chain: &Chain, out: &mut String) {
        for (idx, &(pos, _)) in chain.nodes.iter().enumerate() {
            for &piece in self.stacks.get_ref(pos.q, pos.r).iter().skip(1) {
                out.push_str(&(idx + 1).to_string());
                out.push('=');
                self.push_piece(out, piece, pos);
            }
        }
    }

    fn branches(&self, chain: &Chain, out: &mut String) {
        for &(anchor, is_plus, ref sub) in &chain.branches {
            out.push_str(&anchor.to_string());
            out.push(if is_plus { '+' } else { '-' });
            if self.needs_parens(sub) {
                out.push('(');
                self.chain(sub, out);
                out.push(')');
            } else {
                self.chain(sub, out);
            }
        }
    }

    fn push_turn(&self, out: &mut String, prev: Direction, dir: Direction) {
        if dir == prev.next_direction(self.turns.plus) {
            out.push('+');
        } else if dir == prev.next_direction(self.turns.minus) {
            out.push('-');
        }
    }

    fn push_piece(&self, out: &mut String, piece: Piece, pos: Position) {
        out.push(hop_letter(piece));
        if self.last_moved == Some((piece, pos)) {
            out.push('!');
        }
    }

    fn bottom(&self, pos: Position) -> Piece {
        self.stacks.get_ref(pos.q, pos.r)[0]
    }

    /// A bare branch suffices only when purely linear; an internal `N=X` or sub-branch needs a scope.
    fn needs_parens(&self, chain: &Chain) -> bool {
        !chain.branches.is_empty()
            || chain
                .nodes
                .iter()
                .any(|&(pos, _)| self.stacks.get_ref(pos.q, pos.r).len() > 1)
    }
}

fn hop_letter(piece: Piece) -> char {
    let letter = piece.bug().as_str().chars().next().expect("bug letter");
    if piece.color() == Color::White {
        letter
    } else {
        letter.to_ascii_lowercase()
    }
}

fn serialize_game_type(game_type: GameType) -> Option<&'static str> {
    match game_type {
        GameType::MLP => None,
        GameType::Base => Some("base"),
        GameType::M => Some("base+m"),
        GameType::L => Some("base+l"),
        GameType::P => Some("base+p"),
        GameType::ML => Some("base+ml"),
        GameType::LP => Some("base+lp"),
        GameType::MP => Some("base+mp"),
    }
}
