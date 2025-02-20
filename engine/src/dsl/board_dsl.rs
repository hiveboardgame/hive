use regex::Regex;
use thiserror::Error;
use crate::color::Color;
use crate::bug::Bug;
use crate::piece::Piece;

use pest::Parser;
use pest_derive::Parser;


pub type Result<T> = std::result::Result<T, ParserError>;

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Could not parse board row: {0}")]
    RowError(String),
    #[error("Could not parse start location")]
    StartSyntaxError,
    #[error("Could not parse stack line: {0}")]
    StackLineSyntaxError(String),
    #[error("Could not parse stack: {0}")]
    StackParseError(String),
}

/// Domain Specific Language parser for the [`Board`] struct.
///
/// The idea is to take a string representation such as following and
/// interpret it deterministically as a [`Board`]:
///
/// ```
/// board:
/// 
///   *   *   *   *   * 
/// *   *  bQ  wB1  * 
///   *   2  wQ   *   * 
/// *   *   1   *   * 
///   *   *   *   *   * 
/// 
/// stack:
/// 
/// 1: bottom -> [wA1 bM] <- top
/// 2: bottom -> [bG1 bB2 wB3] <- top
/// ```
///
///
/// The `board:` section specifies visually which pieces are where on the `Board`.
/// The board section must be staggered such that the rows alternate between 
/// aligning [`Flush`] with the left side or [`Shifted`] to the right by two spaces.
/// Each row must contain the same number of space separated tokens.
///
/// The `stack:` section specifies which pieces are in which stacks. 
/// Each number is *stack id* (a number 1-7) which corresponds to the stack's
/// position on the board. This is followed by a space separated *piece list*.
///
/// Comprehensive syntax rules for the DSL can be found in the `dsl/grammar.pest` file.
/// 
/// 
/// [`Board`]: crate::board::Board
/// [`Flush`]: crate::Alignment::Flush
/// [`Shifted`]: crate::Alignment::Shifted
#[derive(Parser)]
#[grammar = "dsl/grammar.pest"]
pub struct BoardParser;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BoardInput {
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Alignment {
    /// Row is flush with the left side of the input
    Flush,
    /// Row is shifted to the right by two space characters
    Shifted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_dsl_syntax() {
        let dsls = [
            concat!(
                "board:\n",
                "  *   *   *   *   * \n",
                "*   *  bQ  wB1  * \n",
                "  *   2  wQ   *   * \n",
                "*   *   1   *   * \n",
                "  *   *   *   *   * \n",
                "\n\n",
                "stack : \n",
                "1: bottom -> [wA1 bM] <- top\n",
                "2: bottom -> [bG1 bB2 wB3] <- top\n",
            ),
            concat!(
                "board:\n",
                "  *   *   *   *   * \n",
                "*   *  bA1 wB1  * \n",
                "  *  bA2 wQ   *   * \n",
                "*   *  wM   *   * \n",
                "  *   *   *   *   * \n",
                "\n\n",
            ),
            concat!(
                "board : \n\n\n",
                "  *   *   *   *   * \n",
                "*   *  bA1 wB1  * \n",
                "  *  bA2 wQ   *   * \n",
                "*   *  wM   *   * \n",
                "  *   *   *   *   * \n",
                "\n\n",
                "stack: \n",
            ),
            concat!(
                "board:\n",
                "  *   *   *   *   * \n",
                "*   *   *   *   * \n",
                "  *   *   *   *   * \n",
                "*   *   *   *   * \n",
                "  *   *   *   *   * \n",
                "stack: \n",
            ),
            concat!(
                "board: \n",
                "stack:\n",
            ),
        ];

        for dsl in dsls.iter() {
            let parsed = BoardParser::parse(Rule::valid_dsl, dsl);
            if parsed.is_err() {
                panic!("Failed to parse dsl: {:?} {:?}", dsl, parsed);
            }
        }

        let invalid_dsls = [
            concat!(
                "board:\n",
                "  *   *   *   *   * \n",
                "*   *  bQ  wB1  * \n",
                "  *   2  wQ   *   * \n",
                "*   *   1   *   * \n",
                "  *   *   *   *   * \n",
                "\n\n",
                "stack \n", // missing colon
                "1:bottom->[wA1 bM]<-top\n",
                "2:bottom->[bG1 bB2 wB3]<-top\n",
            ),
            concat!(
                "board\n", // missing colon
                "  *   *   *   *   * \n",
                "*   *  bA1 wB1  * \n",
                "  *  bA2 wQ   *   * \n",
                "*   *  wM   *   * \n",
                "  *   *   *   *   * \n",
                "\n\n",
            ),
            concat!(
                "stack: \n", // only stack section, needs a board section
            ),
            concat!(
                // missing "board:"
                "  *   *   *   *   * \n",
                "*   *  bQ  wB1  * \n",
                "  *   2  wQ   *   * \n",
                "*   *   1   *   * \n",
                "  *   *   *   *   * \n",
                "\n\n",
                "stack :\n", 
                "1:bottom->[wA1 bM]<-top\n",
                "2:bottom->[bG1 bB2 wB3]<-top\n",
            ),

            "", // empty string (missing "board:")
        ];

        for dsl in invalid_dsls.iter() {
            let parsed = BoardParser::parse(Rule::valid_dsl, dsl);
            if parsed.is_ok() {
                panic!("Expected {:?} to fail. Got {:?}", dsl, parsed);
            }
        }
    }

    #[test]
    pub fn test_stack_rules() {
        let valid_stack_descs = [
            concat!(
                "3:bottom->[wA1 bM]<-top\n", 
                " 1:bottom -> [wA1 bM] <- top \n",      
                "2: [wA1 bM   bQ wB2 ] <-     top ",  // "bottom ->" is optional 
                "5 : bottom  ->[ bA1 wG3]", // "<- top" is optional
            ),
        ];

        for stack_desc in valid_stack_descs.iter() {
            let parsed = BoardParser::parse(Rule::stack_desc, stack_desc);
            if parsed.is_err() {
                panic!("Failed to parse stack_desc: {:?} {:?}", stack_desc, parsed);
            }
        }

        let invalid_stack_descs = [
            "3:bottom->[wA1]<-top\n",  // single piece doesn't make sense
            "1 bottom -> [wA1 bM] <- top\n", // missing colon
            "6: bottom [wA1 bM] <- top\n", // missing "->"
            "7: bottom -> [wA1 bM] top\n", // missing "<-"
            "4: bottom -> [] <- top\n", // empty stack doesn't make sense
            "2: [wA1 bM   bQ wB2 Bb] <-     top ", // bad piece
            "5 : bottom  ->[ bA1 wG3] <- ",  // <- missing "top"
            "8:bottom->[bA1 wA1]<-top\n",  // 8 is out of range
        ];

        for stack_desc in invalid_stack_descs.iter() {
            let parsed = BoardParser::parse(Rule::stack_desc, stack_desc);
            if parsed.is_ok() {
                panic!("Expected {:?} to fail. Got {:?}", stack_desc, parsed);
            }
        }
    }

    #[test]
    pub fn test_board_rules() {
        let boards = [
            concat!(
                "*   *   *   *   * \n", // starts with standard alignment
                "  *   *  bQ  wB1  * \n",
                "*   2  wQ   *   * \n",
                "  *   *   1   *   * \n",
                "*   *   *   *   * \n",
            ),
            concat!(
                "  *   *  bQ  wB1  * \n", // starts shifted alignment
                "*   2  wQ   *   *\n",
                "  *   *   1   *   * \n",
                "*   *   *   *   * \n",
                "  * * * * * \n", // doesn't enforce space consistency
            ),
            concat!(
                "  *   *  bQ  wB1  * \n",
                "*   2  wQ   *   *\n",
                "  *   *   1   *   * \n",
                "*   *   *   *   * \n",
                "  1   *   *   *   *   *  * * * *\n", // doesn't enforce size of rows
            ),
        ];


        for board in boards.iter() {
            let parsed = BoardParser::parse(Rule::board_desc, board);
            if parsed.is_err() {
                panic!("Failed to parse board: {:?} {:?}", board, parsed);
            }

        }
        let malformed_boards = [
            concat!(
                "*   *   *   *   * \n",
                "*   *  bQ  wB1  * \n", // board shifting is incorrect
                "*   2  wQ   *   * \n",
                "  *   *   1   *   * \n",
                "*   *   *   *   * \n",
            ),
            concat!(
                "  *   *  bQ2 wB1  * \n", // illegal piece
                "*   2  wQ   *   *\n",
                "  *   *   1   *   * \n",
                "*   *   *   *   * \n",
                "  1   *   *   *   *   * \n",
            ),
        ];


        for board in malformed_boards.iter() {
            let parsed = BoardParser::parse(Rule::board_desc, board);
            if parsed.is_ok() {
                panic!("Expected \n{} to fail. Got {:?}", board, parsed);
            }
        }
    }

    #[test]
    pub fn test_shifted_row_rules() {
        let shifted_rows = [
            "  *  *  1  *  *\n", // requires two spaces infront
            "  wQ    *     *  *  *\n",
            "  *   *     bB2  *     *   \n",
            "  ",
            "  \n",
            "  *",
            "  *\n***" // trailing characters should be ignored
        ];

        for row in shifted_rows.iter() {
            let parsed = BoardParser::parse(Rule::shifted_row, row);
            if parsed.is_err() {
                panic!("Failed to parse row: {:?} {:?}", row, parsed);
            }
        }

        let shifted_rows_malformed = [
            "*  * **  *\n",
            "**\n",
            "\t*****\n",
            "-",
            "",
            "        * * \n",
            "* *  *  *  *\n",
            "*     *     *  *  *\n",
            "*   *     *  *     *   \n",
            " **",
            "\n",
            "bB2",
            "*\n***",
            "*\n"
        ];

        for row in shifted_rows_malformed.iter() {
            let parsed = BoardParser::parse(Rule::shifted_row, row);
            if parsed.is_ok() {
                panic!("Expected {:?} to fail. Got {:?}", row, parsed);
            }
        }
    }

    #[test]
    pub fn test_aligned_row_rules() {
        let aligned_rows = [
            "*  *  1  *  *\n", // requires two spaces infront
            "wQ    *     *  *  *\n",
            "*   *     bB2  *     *   \n",
            "",
            "\n",
            "*",
            "*\n***" // trailing characters should be ignored
        ];

        for row in aligned_rows.iter() {
            let parsed = BoardParser::parse(Rule::aligned_row, row);
            if parsed.is_err() {
                panic!("Failed to parse row: {:?} {:?}", row, parsed);
            }
        }

        let aligned_rows_malformed = [
            "*  * **  *\n",
            "**\n",
            "*****\n",
            "-",
            " * * \n",
            "   *\n"
        ];

        for row in aligned_rows_malformed.iter() {
            let parsed = BoardParser::parse(Rule::aligned_row, row);
            if parsed.is_ok() {
                panic!("Expected {:?} to fail. Got {:?}", row, parsed);
            }
        }
    }

    #[test]
    pub fn tests_piece_rule () {
        let pieces = vec![
            "wQ",
            "bQ",
            "wM",
            "bS2",
            "bG3 ", // trailing space should be ignored
            "wM5", // nonsense after correct bug (wM) shouldn't matter
            "wA12", // nonsense after correct bug (wA1) shouldn't matter
            "wQwQ", // nonsense after correct bug (wQ) shouldn't matter
            "bP3sdfsfssfsf", // nonsense after correct bug (bP) shouldn't matter
        ];

        for piece in pieces {
            let parsed = BoardParser::parse(Rule::piece, piece);
            assert!(parsed.is_ok());
        }

        let malformed_pieces = vec![
            " wQ",
            "Q",
            "BS2",
            "w A1",
            "bG 3",
            "wB",
            "wA4",
        ];

        for piece in malformed_pieces {
            let parsed = BoardParser::parse(Rule::piece, piece);
            println!("Expected {:?} to fail. Got {:?}", piece, parsed);
            assert!(parsed.is_err());
        }
   }
}

