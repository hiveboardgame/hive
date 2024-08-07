use hive_lib::{GameError, GameResult, GameStatus, GameType, History, State};
use std::env;

fn play_game_from_file(file_path: &str) -> Result<State, GameError> {
    let history = History::from_filepath(file_path)?;
    let mut state: State = State::new(GameType::default(), false);
    for _ in 0..10_000 {
        state = State::new_from_history(&history)?;
        //let _foo = state.board.spawnable_positions(Color::White);
    }
    if let GameStatus::Finished(GameResult::Winner(winner)) = state.game_status {
        println!("State says {winner} won!");
    }
    if let GameStatus::Finished(GameResult::Draw) = state.game_status {
        println!("State says it's a draw");
    }
    if let GameResult::Winner(winner) = history.result {
        println!("History says {winner} won!");
    }
    if let GameResult::Winner(hw) = history.result {
        if let GameStatus::Finished(GameResult::Winner(sw)) = state.game_status {
            if sw != hw {
                return Err(GameError::ResultMismatch {
                    reported_result: history.result,
                    actual_result: GameResult::Winner(sw),
                });
            }
        }
        if let GameStatus::Finished(GameResult::Draw) = state.game_status {
            return Err(GameError::ResultMismatch {
                reported_result: history.result,
                actual_result: GameResult::Draw,
            });
        }
    }
    if let GameResult::Draw = history.result {
        println!("History says game ended in a draw");
        if let GameStatus::Finished(GameResult::Winner(sw)) = state.game_status {
            return Err(GameError::ResultMismatch {
                reported_result: history.result,
                actual_result: GameResult::Winner(sw),
            });
        }
    }
    Ok(state)
}

fn main() {
    let game: Vec<String> = env::args().collect();
    if let Some(game) = game.get(1) {
        println!("Playing game: {game}");
        match play_game_from_file(game) {
            Ok(_) => {}
            Err(e) => eprintln!("{e}"),
        }
    } else {
        eprint!("{}", GameError::NoPgnFile);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_play_games_from_valid_files() {
        for entry in fs::read_dir("./test_pgns/valid/").expect("Should be valid directory") {
            let entry = entry.expect("PGN").path().display().to_string();
            println!("{entry}");
            assert!(play_game_from_file(&entry).is_ok());
        }
    }

    #[test]
    fn test_play_games_from_invalid_files() {
        for entry in fs::read_dir("./test_pgns/invalid/").expect("Should be valid directory") {
            let entry = entry.expect("PGN").path().display().to_string();
            println!("{entry}");
            assert!(play_game_from_file(&entry).is_err());
        }
    }

    #[test]
    fn test_hash_from_valid_files() {
        for entry in fs::read_dir("./test_pgns/hash/valid/").expect("Should be valid directory") {
            let entry = entry.expect("PGN").path().display().to_string();
            println!("{entry}");
            assert!(play_game_from_file(&entry).is_ok());
        }
    }

    #[test]
    fn test_hash_from_invalid_files() {
        for entry in fs::read_dir("./test_pgns/hash/invalid/").expect("Should be valid directory") {
            let entry = entry.expect("PGN").path().display().to_string();
            println!("{entry}");
            assert!(play_game_from_file(&entry).is_err());
        }
    }

    #[test]
    fn test_hash_mirroring_from_files() {
        let mut hashes = Vec::new();
        for entry in fs::read_dir("./test_pgns/hash/mirroring/").expect("Should be valid directory")
        {
            let entry = entry.expect("PGN").path().display().to_string();
            println!("{entry}");
            match play_game_from_file(&entry) {
                Err(e) => panic!("{}", e.to_string()),
                Ok(state) => hashes.push(state.hashes),
            };
        }
        assert_eq!(hashes[0], hashes[1]);
        assert_eq!(hashes[0], hashes[2]);
    }

    #[test]
    fn test_hash_same_position_from_files() {
        let mut hashes = Vec::new();
        for entry in
            fs::read_dir("./test_pgns/hash/same_position/").expect("Should be valid directory")
        {
            let entry = entry.expect("PGN").path().display().to_string();
            println!("{entry}");
            match play_game_from_file(&entry) {
                Err(e) => panic!("{}", e.to_string()),
                Ok(state) => hashes.push(state.hashes),
            };
        }
        assert_eq!(hashes[0].last(), hashes[1].last());
    }

    #[test]
    fn test_hash_rotation_from_files() {
        let mut hashes = Vec::new();
        for entry in fs::read_dir("./test_pgns/hash/rotation/").expect("Should be valid directory")
        {
            let entry = entry.expect("PGN").path().display().to_string();
            println!("{entry}");
            match play_game_from_file(&entry) {
                Err(e) => panic!("{}", e.to_string()),
                Ok(state) => hashes.push(state.hashes),
            };
        }
        assert_eq!(hashes[0], hashes[1]);
    }

    #[test]
    fn test_hash_pass_from_file() {
        let file = String::from("./test_pgns/hash/short_pass.pgn");
        println!("{file}");
        match play_game_from_file(&file) {
            Err(e) => panic!("{}", e.to_string()),
            Ok(state) => {
                assert_eq!(state.hashes.len(), state.turn);
            }
        };
    }
}
