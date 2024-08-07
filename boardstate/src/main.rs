use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
struct Position {
    r: u8,
    q: u8,
    l: u8,
}

impl Position {
    fn new(r: u8, q: u8, l: u8) -> Self {
        Self { r, q, l }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct BoardState {
    positions: [Option<Position>; 28],
}

impl BoardState {
    fn new() -> Self {
        Self {
            positions: [const { None }; 28],
        }
    }
}

fn main() {
    let mut buf = Vec::new();
    let mut bs = BoardState::new();
    for i in 0..28 {
        bs.positions[i] = Some(Position::new(i as u8, i as u8 * 2, 0));
    }
    bs.serialize(&mut Serializer::new(&mut buf)).unwrap();
    println!("Buf len: {}",  buf.len());
}
