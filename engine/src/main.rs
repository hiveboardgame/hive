use clap::{Parser, Subcommand};
use hive_lib::{Color, GameError, GameResult, GameStatus, GameType, History, State};
use std::path::PathBuf;

fn print_game_from_file(file: PathBuf, turn: usize) -> Result<(), GameError> {
    let history = History::from_filepath(file.clone())?;
    State::print_turn_from_history(&history, turn, file)?;
    Ok(())
}

fn play_game_from_file(file: PathBuf) -> Result<State, GameError> {
    println!("Playing game: {}", file.display());
    let history = History::from_filepath(file)?;
    let mut state: State = State::new(GameType::default(), false);
    for _ in 0..1 {
        state = State::new_from_history(&history)?;
        let _foo = state.board.spawnable_positions(Color::White);
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

#[derive(Parser)]
#[command(author, version, about = "Evaluates Hive games from PGN")]
struct Cli {
    #[arg(value_parser)]
    file: PathBuf,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(name = "print")]
    Print {
        /// Move to be printed, defaults to 0 i.e. last move
        #[arg(short, long, default_value_t = 0)]
        turn: usize,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Print { turn }) => match print_game_from_file(cli.file, turn) {
            Ok(_) => {}
            Err(e) => eprintln!("{e}"),
        },
        // TODO @neal @leex: this is what we need to implement
        None => match play_game_from_file(cli.file) {
            Ok(_) => {}
            Err(e) => eprintln!("{e}"),
        },
    }
}

#[cfg(test)]
mod tests;
