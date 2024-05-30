use std::collections::HashMap;
use 
// 8 distinct bugs
// 2 colors
// 14 bugs each
// 28 bugs total
// 28 bugs <= 32, 32 = 5 bits
// A move is 2 bugs 1 position
// A move is 5 bits + 5 bits + Position
// Position: NE, NW, W, SW, SE, E NE = 6, 2^3
// A move can therefore be encoded in 16bits
// A game is on average 100 moves
// 16000 bits or 200bytes
// 200 bytes is 200 ASCII characters
// So all human vs human PLM games played on BS can be encoded in 2MB
struct Nanoid(String);
struct Username(String);

pub struct Book {
    total_games: usize,
    moves: Vec<u16>,
    games: Vec<(usize, usize)>, // start position in "moves"
    // number of moves to get
    nanoid_games: HashMap<Nanoid, usize>,
    players_games: HashMap<Username, Vec<String>>,
    white_won: Vec<Nanoid>,
    black_won: Vec<Nanoid>,
    draws: Vec<Nanoid>,
    wp_games: Vec<Nanoid>,
    wl_games: Vec<Nanoid>,
    buffer_games: Vec<Nanoid>,
}

impl Default for Book {
    fn default() -> Self {
        Self::new()
    }
}

impl Book {
    pub fn new() -> Self {
        Self {
            total_games: 0,
            moves: Vec::new(),
            games: Vec::new(),
            nanoid_games: HashMap::new(),
            players_games: HashMap::new(),
            white_won: Vec::new(),
            black_won: Vec::new(),
            draws: Vec::new(),
            // these need to be implemented
            wp_games: Vec::new(),
            wl_games: Vec::new(),
            buffer_games: Vec::new(),
        }
    }

    pub fn add_game_from_file(path: String) {

    }

    pub fn check
}

fn main() {

}
