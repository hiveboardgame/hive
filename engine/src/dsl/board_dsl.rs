use crate::board::Board;
use crate::bug::Bug;
use itertools::Itertools;
use crate::color::Color;
use crate::piece::Piece;
use regex::Regex;
use std::str::FromStr;
use thiserror::Error;
use std::collections::HashMap;

use pest::iterators::Pair;
use pest::{Parser, RuleType};
use pest_derive::Parser;

type Result<T> = std::result::Result<T, ParserError>;

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Duplicate stack id found: {0}")] 
    DuplicateStackId(u8),
    #[error("Could not interpret this piece: {0}")] 
    PieceParse(String),
    #[error("Rows are not all the same length")]
    RowWidth,
    #[error("Could not parse board row: {0}")]
    Row(String),
    #[error("Could not parse start location")]
    StartSyntax,
    #[error("Could not parse stack line: {0}")]
    StackLineSyntax(String),
    #[error("Could not parse stack: {0}")]
    StackParse(String),
}

/// Domain Specific Language (DSL) parser for the [`Board`] struct.
///
/// The idea is to take a string representation such as following and
/// interpret it deterministically as a [`Board`]:
///
/// ```text
///  board:
///
///    *   *   *   *   *
///  *   *  bQ  wB1  *
///    *   2  wQ   *   *
///  *   *   1   *   *
///    *   *   *   *   *
///
///  stack:
///
///  1: bottom -> [wA1 bM] <- top
///  2: bottom -> [bG1 bB2 wB3] <- top
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
/// Note: the goal of this DSL is to strike a balance between:
///
/// - human-readability - so it is useful for debugging in command lines
/// - flexibility - so it is easily written by hand or generated by code
/// - brevity - so it reduces bandwidth required for game state serialization and deserialization
///
/// [`Board`]: crate::board::Board
/// [`Flush`]: crate::Alignment::Flush
/// [`Shifted`]: crate::Alignment::Shifted
#[derive(Parser)]
#[grammar = "dsl/grammar.pest"]
pub struct BoardParser;

/// Describes the details of symbols collect
/// from the board section or stack section of the DSL.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BoardInput {
    Star,
    Piece(Piece),
    StackId(u8),
}

/// Describes the alignment of a row in the board section of the DSL.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Alignment {
    /// Row is flush with the left side of the input
    Flush,
    /// Row is shifted to the right by two space characters
    Shifted,
}

impl BoardParser {
    // TODO: function to help diagnose parsing errors,
    // parses section by section until some error is encountered
    pub fn diagnose() {}

    /// Locate and return all instances of the rule within the syntax
    /// tree. Descends in a depth-first manner.
    fn dig(pair: Pair<Rule>, rule: Rule) -> Vec<Pair<Rule>> {
        let mut res = Vec::new();

        if pair.as_rule() == rule {
            res.push(pair.clone());
        }
        for p in pair.into_inner() {
            let r = BoardParser::dig(p, rule);
            res.extend(r);
        }
        res
    }

    /// Locate and return the first instance of the rule within the syntax
    /// tree. Descends in a depth-first manner.
    fn find(pair: Pair<Rule>, rule: Rule) -> Option<Pair<Rule>> {
        if pair.as_rule() == rule {
            return Some(pair);
        }
        for p in pair.into_inner() {
            let res = BoardParser::find(p, rule);
            if let Some(r) = res {
                return Some(r);
            }
        }
        None
    }

    fn handle_hex(hex: Pair<Rule>) -> BoardInput {
        assert!(hex.as_rule() == Rule::hex);
        let symbol = hex.into_inner().next().unwrap();
        match symbol.as_rule() {
            Rule::star => return BoardInput::Star,
            Rule::piece => {
                let piece = symbol.into_inner().next().unwrap();
                let piece = Piece::from_str(piece.as_str()).unwrap();
                return BoardInput::Piece(piece);
            }
            Rule::stack_num => {
                let stack_num = symbol.as_str().parse::<u8>().unwrap();
                return BoardInput::StackId(stack_num);
            }
            _ => panic!("Unexpected input: {:?}, expected hex symbol", symbol),
        }
    }


    fn handle_aligned_row(pair: Pair<Rule>) -> Vec<BoardInput> {
        assert!(pair.as_rule() == Rule::aligned_row);
        let mut row = Vec::new();
        let hexes = BoardParser::dig(pair, Rule::hex);

        for hex in hexes.into_iter() {
            let input = BoardParser::handle_hex(hex);
            row.push(input)
        }
        println!("{:#?}", row);

        row
    }

    fn handle_rows_start_aligned(pair : Pair<Rule>) -> Vec<Vec<BoardInput>> {
        assert!(pair.as_rule() == Rule::starts_aligned);

        let aligned_rows = BoardParser::dig(pair.clone(), Rule::aligned_row);

        let mut rows = Vec::new();
        for row in aligned_rows.into_iter() {
            let inputs = BoardParser::handle_aligned_row(row);
            rows.push(inputs)
        }

        rows
    }

    fn star_inserted_left(row : Vec<BoardInput>) -> Vec<BoardInput> {
        let mut new_row = Vec::new();
        new_row.push(BoardInput::Star);
        new_row.extend(row);
        new_row
    }

    fn handle_rows_start_shifted(pair : Pair<Rule>) -> Vec<Vec<BoardInput>> {
        assert!(pair.as_rule() == Rule::starts_shifted);

        let aligned_rows = BoardParser::dig(pair.clone(), Rule::aligned_row);

        let mut rows = Vec::new();
        for row in aligned_rows.into_iter() {
            let inputs = BoardParser::handle_aligned_row(row);
            rows.push(inputs)
        }

        rows.into_iter().enumerate().map(|(i, row)| {
            match i {
                i if i % 2 == 0 => row,
                _ => BoardParser::star_inserted_left(row)
            }
        }).collect()
    }

    fn handle_rows_empty(pair : Pair<Rule>) -> Vec<Vec<BoardInput>> {
        Vec::new()
    }

    fn board_from_inputs(inputs : Vec<Vec<BoardInput>>) -> Board {
        todo!()
    }

    fn handle_board_section(pair: Pair<Rule>) -> Result<Board> {
        assert!(pair.as_rule() == Rule::board_section);

        let result = BoardParser::find(pair, Rule::board_desc);
        let board_desc = result.expect("Grammar error: missing board_desc");

        let starts_shifted = BoardParser::find(board_desc.clone(), Rule::starts_shifted);
        let starts_aligned = BoardParser::find(board_desc.clone(), Rule::starts_aligned);
        let empty = BoardParser::find(board_desc, Rule::empty);

        let rows = match (starts_shifted, starts_aligned, empty) {
            (Some(pair), None, None) => BoardParser::handle_rows_start_shifted(pair),
            (None, Some(pair), None) => BoardParser::handle_rows_start_aligned(pair),
            (None, None, Some(pair)) => BoardParser::handle_rows_empty(pair),
            _ => panic!("Grammar error: unexpected board_desc"),
        };

        // Filter out extraneous empty rows 
        let rows = rows.into_iter().filter(|row| row.len() > 0).collect_vec();

        // Ensure all remaining rows are the same width or there are 
        // no remaining rows
        let rows_same_width = rows.iter().map(|row| row.len()).unique().count() <= 1;
        if !rows_same_width {
            return Err(ParserError::RowWidth)
        }

        let board = BoardParser::board_from_inputs(rows);
        Ok(board)
    }

    fn handle_stack_desc(pair: Pair<Rule>) -> Result<(u8, Vec<Piece>)> {
        assert!(pair.as_rule() == Rule::stack_desc);

        let num = BoardParser::find(pair.clone(), Rule::stack_id).expect("Grammar error missing 'stack_id'"); 
        let num : u8 = num.as_str().parse::<u8>().unwrap();
        let pieces = BoardParser::dig(pair, Rule::piece).into_iter().map(|p| p.as_str()).map(|p| (p, Piece::from_str(p)));

        let mut final_pieces = Vec::new();
        for (string, result) in pieces {
            if result.is_err() { return Err(ParserError::PieceParse(string.to_owned())) }
            final_pieces.push(result.unwrap());
        }


        Ok((num, final_pieces))
    }

    fn handle_stack_section(pair : Pair<Rule>) -> Result<HashMap<u8, Vec<Piece>>> {
        assert!(pair.as_rule() == Rule::stack_section);

        let descs = BoardParser::dig(pair, Rule::stack_desc);
        let mut map = HashMap::new();

        for d in descs {
            let (id, pieces) = BoardParser::handle_stack_desc(d)?;
            if map.contains_key(&id) { 
                return Err(ParserError::DuplicateStackId(id))
            }
            map.insert(id, pieces);
        }

        Ok(map)
    }

    fn parse_board( input : &str ) -> Result<Board> {

        let mut board = Board::new(); 
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_handle_board_section_syntax_semantics() {
        // Some syntactically correct boards are incorrect semantically.
        // They cannot be converted into a fixed-width grid of board inputs
        // but do not contain missing symbols per se. The BoardParser must 
        // reject them as some step 

        let reject = [
            concat!(
                "board:\n",
                "  *   *   *   *   * \n",
                "*   *  bQ  wB1  * \n",
                "  *   2  wQ   *   * \n",
                "*   *   1   *   * \n",
                "  *   *   *  *   *   * \n", // extra column here
                "\n"
            ),
        ];

        for board in reject  {

            
            let pair = BoardParser::parse(Rule::board_section, board).expect("Board Section did not parse").next().unwrap();
            let board = BoardParser::handle_board_section(pair);
            assert!(board.is_err());

        }
    }

    #[test]
    pub fn test_handle_board_section() {
        let boards = [
            (
                concat!(
                    "board:\n",
                    "  *   *   *   *   \n",
                    "*   *  bQ  wB1   \n",
                    "  *   2  wQ   *    \n",
                    ), 3, 4
            ),

            (
                concat!(
                    "board:\n",
                    "  *   *   *   *   * \n",
                    "*   *  bQ  wB1  * \n",
                    "  *   2  wQ   *   * \n",
                    "*   *   1   *   * \n",
                    "  *   *   *   *   * \n",
                    "\n"), 5, 5
            ),


            (
                concat!(
                    "board:\n",
                    "  *   *   *   *   * \n",
                    "*   *  bQ  wB1  * \n",
                    "  *   2  wQ   *   * \n",
                    "*   *   1   *   * \n",
                    "  *   *   *   *   * \n",
                    "\n\n"
                ), 5, 5
            ),
        ];

        for (board, rows, cols) in boards {
            let pair = BoardParser::parse(Rule::board_section, board)
                .unwrap()
                .next()
                .unwrap();
            let board = BoardParser::handle_board_section(pair);
            let board = board.expect("Must be able to handle board section");
            assert!( board.len() == rows );
            assert!( board[0].len() == cols );
        }
    }

    #[test]
    pub fn test_dsl_rules() {
        let dsls = [
            concat!(
                // make sure that the example parses correctly
                "board:\n",
                "\n",
                "  *   *   *   *   *\n",
                "*   *  bQ  wB1  *\n",
                "  *   2  wQ   *   *\n",
                "*   *   1   *   *\n",
                "  *   *   *   *   *\n",
                "\n",
                "stack:\n",
                //"\n",
                "1: bottom -> [wA1 bM] <- top\n",
                "2: bottom -> [bG1 bB2 wB3] <- top\n",
            ),
            concat!(
                "board:\n",
                "  *   *   1   2   * \n", // can parse single row
                "stack: \n",
                "1: bottom -> [wA1 bM] <- top\n",
                "2: bottom -> [bG1 bB2 wB3] <- top\n",
            ),
            concat!(
                "board :\n", // odd rows, starts shifted
                "  1   5   *   *   * \n",
                "*   *  bA1 wB1  * \n",
                "  *  bA2 wQ   *   * \n",
                // stack can be omitted
                // Note : syntactically correct but not semantically correct
                // 1 and 5 stacks are missing
            ),
            concat!(
                "board :\n", // even rows, starts flush, space after board allowed
                "1   6   *   *   * \n",
                "  *   *  bA1 wB1  * \n",
                // stack can be omitted
                // Note : syntactically correct but not semantically correct,
                // 1 and 6 stacks are missing
            ),
            concat!(
                "board:\n",
                "*   *  bA1 wB1  * \n",
                "  1   *   *   *   * \n",
                "*   *  bA1 wB1  * \n",
                "  *  bA2 wQ   *   * \n",
                "*   *  wM   *   * \n",
                // stack can be omitted
                // Note : syntactically correct but not semantically correct
                // 1 stack is missing
            ),
            concat!(
                "board: \n", // space after colon
                "  2   *   *   *   * \n",
                "*   *  bA1 wB1  * \n",
                "  *  bA2 wQ   *   * \n",
                "*   *  wM   *   * \n",
                "  *   *   *   *   * \n",
                "stack:", // stack ids can be omitted
                          // Note: syntactically correct but not
                          // semantically correct, the 2 stack is missing
            ),
            concat!(
                " \n", // new lines and space before "board:" allowed
                " board:\n",
                "  3   *   *   * \n",
                "*   *   *   * \n",
                "  *   *   *   * \n",
                "*   *   *   *     \n", // trailing spaces allowed
                "  *   *   *   *\n",
                "stack: \n", // stack ids can be omitted Note:
                             // syntactically correct but not semantically correct
                             // the 3 stack is missing
            ),
            concat!(
                "\n",
                "board:\n",
                "\n", // new lines and space after "board:" allowed
                "*   *   *   * \n",
                "  *   *   *   * \n",
                "*   *   *   * \n",
                "  *   *   *   * \n",
                "stack: \n", // stack ids can be omitted (syntactically correct and
                             // semantically correct, there are no stacks)
            ),
            concat!(
                " \n\n",
                "board:\n",
                "  *\n", // single piece allowed
                "stack: \n",
            ),
            concat!(
                " \n\n",
                "board:\n",
                "  *   *   *   *   * \n",
                "*   *   *   *   * \n",
                "  *   *          *   *   * \n", // too many intra-row spaces allowed
                "*   *   * *   * \n",            // almost too few intra-row spaces allowed
                "  *   *   *   *   * \n",
                "stack: \n",
            ),
            concat!(
                "board: \n", // empty board allowed
                "stack:\n",  // empty stack allowed
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
                panic!(
                    "Expected\n-----\n{}\n------\n to fail.\nGot {:?}",
                    dsl, parsed
                );
            }
        }
    }

    #[test]
    pub fn test_board_section_rules() {
        let board = concat!(
            "board:\n",
            "  *   *   *   *   * \n",
            "*   *  bQ  wB1  * \n",
            "  *   2  wQ   *   * \n",
            "*   *   1   *   * \n",
            "  *   *   *   *   * \n",
            "\n"
        );

        let _ = BoardParser::parse(Rule::board_section, board)
            .unwrap()
            .next()
            .unwrap();

        let invalid = concat!(
            "board:\n",
            "  *   *   *   *   * \n",
            "*   *  bQ  wB1  * \n",
            "\n", // extra section now allowed here
            "  *   2  wQ   *   * \n", 
            "*   *   1   *   * \n",
            "\n",
        );

        let board = BoardParser::parse(Rule::board_section_isolated, invalid);

        if board.is_ok() {
            panic!("Expected board parser to fail on syntactically invalid input")
        }
    }

    #[test]
    pub fn test_stack_section_rules() {
        let valid_stack_section = [
            concat!("stack:\n", "\n", "3:bottom->[wA1 bM]<-top\n",),
            concat!(
                "stack:\n",
                "\n",
                "3:bottom->[wA1 bM]<-top\n",
                "1:bottom -> [wA1 bM] <- top \n",
            ),
            concat!(
                "stack:\n",
                "\n",
                "3:bottom->[wA1 bM]<-top\n",
                "1:bottom -> [wA1 bM] <- top \n",
                "2: [wA1 bM   bQ wB2 ] <-     top\n", // "bottom ->" is optional
                "5: bottom  ->[ bA1 wG3]",            // "<- top" is optional
            ),
            concat!(
                "stack:\n",
                "3:bottom->[wA1 bM]<-top\n",
                "1:bottom -> [wA1 bM] <- top \n",
                "2: [wA1 bM   bQ wB2 ] <-     top\n", // "bottom ->" is optional
                "5 : bottom  ->[ bA1 wG3]",           // "<- top" is optional
            ),
            concat!("stack:\n",),
        ];

        for stack_section in valid_stack_section.iter() {
            let parsed = BoardParser::parse(Rule::stack_section_test, stack_section);
            if parsed.is_err() {
                panic!(
                    "Failed to parse stack_section: {:?} {:?}",
                    stack_section, parsed
                );
            }
        }
    }

    #[test]
    pub fn test_stack_desc_rules() {
        let valid_stack_descs = [
            "3:bottom->[wA1 bM]<-top\n",
            "1:bottom -> [wA1 bM] <- top \n",
            "2: [wA1 bM   bQ wB2 ] <-     top ", // "bottom ->" is optional
            "5 : bottom  ->[ bA1 wG3]",          // "<- top" is optional
        ];

        for stack_desc in valid_stack_descs.iter() {
            let parsed = BoardParser::parse(Rule::stack_desc, stack_desc);
            if parsed.is_err() {
                panic!("Failed to parse stack_desc: {:?} {:?}", stack_desc, parsed);
            }
        }

        let invalid_stack_descs = [
            "3:bottom->[wA1]<-top\n",              // single piece doesn't make sense
            "1 bottom -> [wA1 bM] <- top\n",       // missing colon
            "6: bottom [wA1 bM] <- top\n",         // missing "->"
            "7: bottom -> [wA1 bM] top\n",         // missing "<-"
            "4: bottom -> [] <- top\n",            // empty stack doesn't make sense
            "2: [wA1 bM   bQ wB2 Bb] <-     top ", // bad piece
            "5 : bottom  ->[ bA1 wG3] <- ",        // <- missing "top"
            "8:bottom->[bA1 wA1]<-top\n",          // 8 is out of range
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
                "  4   *   *   *   * \n", "*   *  bA1 wB1  * \n",),

            concat!(
                "  2   *   *   *   * \n",
                "*   *  bA1 wB1  * \n",
                "  *  bA2 wQ   *   * \n",
            ),
            concat!(
                "  1   *   *   *   * \n",
                "*   *  bA1 wB1  * \n",
                "  *  bA2 wQ   *   * \n",
                "*   *  wM   *   * \n",
            ),
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
            let parsed = BoardParser::parse(Rule::board_desc_test, board);
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
            let parsed = BoardParser::parse(Rule::board_desc_test, board);
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
            "  *\n***", // trailing characters should be ignored
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
            "*\n",
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
            "*\n***", // trailing characters should be ignored
        ];

        for row in aligned_rows.iter() {
            let parsed = BoardParser::parse(Rule::aligned_row, row);
            if parsed.is_err() {
                panic!("Failed to parse row: {:?} {:?}", row, parsed);
            }
        }

        let aligned_rows_malformed = ["*  * **  *\n", "**\n", "*****\n", "-", " * * \n", "   *\n"];

        for row in aligned_rows_malformed.iter() {
            let parsed = BoardParser::parse(Rule::aligned_row, row);
            if parsed.is_ok() {
                panic!("Expected {:?} to fail. Got {:?}", row, parsed);
            }
        }
    }

    #[test]
    pub fn tests_piece_rule() {
        let pieces = vec![
            "wQ",
            "bQ",
            "wM",
            "bS2",
            "bG3 ",          // trailing space should be ignored
            "wM5",           // nonsense after correct bug (wM) shouldn't matter
            "wA12",          // nonsense after correct bug (wA1) shouldn't matter
            "wQwQ",          // nonsense after correct bug (wQ) shouldn't matter
            "bP3sdfsfssfsf", // nonsense after correct bug (bP) shouldn't matter
        ];

        for piece in pieces {
            let parsed = BoardParser::parse(Rule::piece, piece);
            assert!(parsed.is_ok());
        }

        let malformed_pieces = vec![" wQ", "Q", "BS2", "w A1", "bG 3", "wB", "wA4"];

        for piece in malformed_pieces {
            let parsed = BoardParser::parse(Rule::piece, piece);
            println!("Expected {:?} to fail. Got {:?}", piece, parsed);
            assert!(parsed.is_err());
        }
    }
}
