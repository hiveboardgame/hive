use crate::{
    game::Game,
    line::{self, Line},
    notation,
    prove::Prover,
};
use anyhow::{anyhow, bail, Context, Result};
use hive_lib::{hop, Board, Bug, Color, GameResult, GameStatus, GameType, Piece, Position, State};
use shared_types::{PuzzleImport, PuzzleKind};
use std::{collections::HashMap, str::FromStr};

pub const COLLECTION: &str = "hive-puzzles-vol1";
pub const SOURCE: &str =
    "Hive Puzzles Vol. 1 by Enrique Sedano, Jonas Kronander and Christian Sperling (gripot.se/hive)";

/// One puzzle page of the book: the Entomology position behind its diagram plus the page text.
#[derive(Debug, Clone, Default)]
pub struct BookPage {
    pub number: usize,
    pub board: String,
    pub authors: Vec<String>,
    pub objective: String,
    pub level: String,
    pub label: String,
    pub hint: Option<String>,
}

/// A position as Entomology encodes it: `:<to move>[<last moved coordinate>]` then stacks written
/// top piece first, each followed by its coordinate, then `*`-prefixed reserve counts.
#[derive(Debug, Clone, PartialEq)]
pub struct EntomologyPosition {
    pub to_move: Color,
    pub last_moved: Option<(i32, i32)>,
    pub stacks: Vec<((i32, i32), Vec<Piece>)>,
    pub reserve: HashMap<(Color, Bug), usize>,
}

fn parse_coordinate(chars: &[char], index: &mut usize) -> Option<i32> {
    let sign = match chars.get(*index)? {
        '+' => 1,
        '-' => -1,
        _ => return None,
    };
    *index += 1;
    let start = *index;
    while chars.get(*index).is_some_and(|c| c.is_ascii_digit()) {
        *index += 1;
    }
    let digits: String = chars[start..*index].iter().collect();
    digits.parse::<i32>().ok().map(|value| sign * value)
}

fn parse_piece(chars: &[char], index: &mut usize) -> Option<Piece> {
    let color = *chars.get(*index)?;
    let bug = *chars.get(*index + 1)?;
    if !matches!(color, 'w' | 'b') || !"QABGSMLP".contains(bug) {
        return None;
    }
    let mut end = *index + 2;
    if chars.get(end).is_some_and(|c| c.is_ascii_digit()) {
        end += 1;
    }
    let text: String = chars[*index..end].iter().collect();
    let piece = Piece::from_str(&text).ok()?;
    *index = end;
    Some(piece)
}

pub fn parse_entomology(board: &str) -> Result<EntomologyPosition> {
    let board = board.strip_prefix(':').unwrap_or(board);
    let (stacks_part, reserve_part) = board.split_once('*').unwrap_or((board, ""));
    let chars: Vec<char> = stacks_part.chars().collect();
    let to_move = match chars.first() {
        Some('w') => Color::White,
        Some('b') => Color::Black,
        other => bail!("expected a side to move, found {other:?}"),
    };
    let mut index = 1;
    let mut last_moved = None;
    if matches!(chars.get(index), Some('+') | Some('-')) {
        let q = parse_coordinate(&chars, &mut index).ok_or_else(|| anyhow!("bad last move"))?;
        let r = parse_coordinate(&chars, &mut index).ok_or_else(|| anyhow!("bad last move"))?;
        last_moved = Some((q, r));
    }
    let mut stacks = Vec::new();
    let mut pieces = Vec::new();
    while index < chars.len() {
        if let Some(piece) = parse_piece(&chars, &mut index) {
            pieces.push(piece);
            continue;
        }
        let q = parse_coordinate(&chars, &mut index)
            .ok_or_else(|| anyhow!("unexpected {:?} at {index}", chars.get(index)))?;
        let r = parse_coordinate(&chars, &mut index).ok_or_else(|| anyhow!("missing r"))?;
        if pieces.is_empty() {
            bail!("coordinate without pieces at {index}");
        }
        stacks.push(((q, r), std::mem::take(&mut pieces)));
    }
    // Reserve entries are `<count stars>@<piece>`: each star is one piece of the bug after it.
    let mut reserve = HashMap::new();
    let mut stars = 0;
    let reserve_part = if reserve_part.is_empty() {
        String::new()
    } else {
        format!("*{reserve_part}")
    };
    for token in reserve_part.split('@') {
        let count = token.chars().rev().take_while(|c| *c == '*').count();
        let name = token.trim_end_matches('*');
        if let (Some(color), Some(bug)) = (name.chars().next(), name.chars().nth(1)) {
            let color = if color == 'w' {
                Color::White
            } else {
                Color::Black
            };
            if let Ok(bug) = Bug::from_str(&bug.to_string()) {
                *reserve.entry((color, bug)).or_insert(0) += stars;
            }
        }
        stars = count;
    }
    Ok(EntomologyPosition {
        to_move,
        last_moved,
        stacks,
        reserve,
    })
}

/// Entomology's axes, mapped onto the engine's: verified by every book position forming one
/// connected hive under it, which no other axis assignment manages.
pub fn to_position((x, y): (i32, i32)) -> Position {
    let centre = Position::initial_spawn_position();
    Position::new(centre.q + x, centre.r + y)
}

pub fn game_type_from_label(label: &str) -> Option<GameType> {
    let expansions = label
        .trim()
        .strip_prefix("Hive")
        .unwrap_or("")
        .trim_start_matches('-');
    let has = |bug: char| expansions.contains(bug);
    Some(match (has('M'), has('L'), has('P')) {
        (false, false, false) => GameType::Base,
        (true, false, false) => GameType::M,
        (false, true, false) => GameType::L,
        (false, false, true) => GameType::P,
        (true, true, false) => GameType::ML,
        (true, false, true) => GameType::MP,
        (false, true, true) => GameType::LP,
        (true, true, true) => GameType::MLP,
    })
}

/// The page label, widened by any expansion bug the position actually uses: a few pages say
/// "Hive" over a diagram with a Mosquito in it.
pub fn game_type_for(label: &str, position: &EntomologyPosition) -> Option<GameType> {
    let used = |bug: Bug| {
        position
            .stacks
            .iter()
            .any(|(_, pieces)| pieces.iter().any(|piece| piece.bug() == bug))
            || position.reserve.keys().any(|(_, held)| *held == bug)
    };
    let mut expansions = String::new();
    for (bug, letter) in [
        (Bug::Mosquito, 'M'),
        (Bug::Ladybug, 'L'),
        (Bug::Pillbug, 'P'),
    ] {
        if used(bug) || label.contains(letter) {
            expansions.push(letter);
        }
    }
    game_type_from_label(&format!("Hive-{expansions}"))
}

/// The playable state for a page, plus whether the book's reserve is exactly what the game type
/// leaves in hand (a puzzle that restricts the reserve would let our engine find other answers).
pub fn build_state(position: &EntomologyPosition, game_type: GameType) -> Result<(State, bool)> {
    let mut board = Board::new();
    for (coordinate, pieces) in &position.stacks {
        for piece in pieces.iter().rev() {
            board.insert(to_position(*coordinate), *piece, true);
        }
    }
    if let Some(coordinate) = position.last_moved {
        let at = to_position(coordinate);
        let piece = board
            .top_piece(at)
            .ok_or_else(|| anyhow!("last move points at an empty cell"))?;
        board.last_moved = Some((piece, at));
    }
    let hop = hop::from_position(&board, game_type, position.to_move);
    let parsed = hop::parse(&hop).map_err(|e| anyhow!("HOP round trip: {e}"))?;
    let state = State::new_from_position(parsed.board, game_type, position.to_move)
        .map_err(|e| anyhow!("{e}"))?;
    let mut reserve_matches = true;
    for color in [Color::White, Color::Black] {
        let in_hand = state.board.reserve(color, game_type);
        for bug in Bug::all() {
            let ours = in_hand.get(&bug).map_or(0, Vec::len);
            let theirs = position.reserve.get(&(color, bug)).copied().unwrap_or(0);
            if ours != theirs {
                reserve_matches = false;
            }
        }
    }
    Ok((state, reserve_matches))
}

/// The book's claimed length in solver moves, when the objective states one.
pub fn claimed_moves(objective: &str) -> Option<u32> {
    let text = objective.to_lowercase();
    if text.contains(" or less") || text.contains("-") {
        return None;
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    words.windows(3).find_map(|window| {
        let [preposition, number, unit] = window else {
            return None;
        };
        let number: u32 = number.parse().ok()?;
        if *preposition != "in" {
            return None;
        }
        // The book defines a turn as one move by each side, but its objectives use "moves" for
        // the solver's moves too (#7 is four solver moves), so both count solver moves.
        match unit.trim_end_matches(['.', '?', ',']) {
            "turns" | "turn" | "moves" | "move" => Some(number),
            _ => None,
        }
    })
}

pub fn seed_rating(level: &str) -> f64 {
    match level.to_ascii_uppercase().as_str() {
        "EASY" => 1250.0,
        "INTERMEDIATE" => 1550.0,
        "EXPERT" => 1850.0,
        "MASTER" => 2150.0,
        _ => 1500.0,
    }
}

#[derive(Debug)]
pub enum Outcome {
    Puzzle(Box<PuzzleImport>),
    Skipped(String),
}

/// Turns a page into a puzzle: the position is proven with the site's engine, and the book's
/// claimed length is checked, so a puzzle our rules read differently is skipped, not guessed.
pub fn import_page(
    page: &BookPage,
    prover: &mut Prover,
    max_plies: u32,
    budget: u64,
) -> Result<Outcome> {
    let position = parse_entomology(&page.board).context("entomology board")?;
    let Some(game_type) = game_type_for(&page.label, &position) else {
        return Ok(Outcome::Skipped(format!(
            "unknown game type {}",
            page.label
        )));
    };
    let (state, reserve_matches) = match build_state(&position, game_type) {
        Ok(built) => built,
        Err(error) => return Ok(Outcome::Skipped(format!("position: {error}"))),
    };
    if !reserve_matches {
        return Ok(Outcome::Skipped(
            "the book restricts the reserve; the engine would see other answers".to_string(),
        ));
    }
    let mut start = state;
    let mut game = Game::from_state(&start);
    prover.reset_budget(budget);
    let mut found = prover
        .shortest_win(&mut game, max_plies)
        .map_err(|_| anyhow!("over budget"))?;
    // "Who wins?" pages can be lost for the side to move: then the puzzle starts after their
    // most stubborn defence, for the winner.
    if found.is_none() {
        let defence = prover
            .longest_defence(&mut game, max_plies + 1)
            .map_err(|_| anyhow!("over budget"))?;
        if let Some((reply, _)) = defence {
            notation::play(&mut start, &reply)?;
            let mut after = Game::from_state(&start);
            found = prover
                .shortest_win(&mut after, max_plies)
                .map_err(|_| anyhow!("over budget"))?;
        }
    }
    let Some((plies, _)) = found else {
        return Ok(Outcome::Skipped(format!(
            "no forced win within {max_plies} plies"
        )));
    };
    let moves = plies.div_ceil(2);
    if let Some(claimed) = claimed_moves(&page.objective) {
        if claimed != moves {
            return Ok(Outcome::Skipped(format!(
                "the book says {claimed} moves, the engine finds {moves}"
            )));
        }
    }
    let game = Game::from_state(&start);
    let Ok(Ok(solved)) = line::solve(prover, &game, plies) else {
        return Ok(Outcome::Skipped(
            "too many equally good answers".to_string(),
        ));
    };
    let mut lines = spell(&start, &solved)?;
    let solution = lines.remove(0);
    let position_hop = hop::from_position(&start.board, start.game_type, start.turn_color);
    let puzzle = PuzzleImport {
        kind: PuzzleKind::WinIn,
        game_type: game_type.to_string(),
        history: None,
        position: Some(position_hop),
        solution,
        variations: lines,
        themes: vec![format!("winIn{moves}"), "book".to_string()],
        authors: page.authors.clone(),
        source: format!("{SOURCE}, puzzle #{}", page.number),
        source_game: None,
        collection: Some(COLLECTION.to_string()),
        title: Some(format!("Hive Puzzles Vol. 1 · #{}", page.number)),
        hint: page.hint.clone(),
        level: Some(page.level.to_lowercase()),
        rating: Some(seed_rating(&page.level)),
    };
    match puzzle.verify() {
        Ok(_) => Ok(Outcome::Puzzle(Box::new(puzzle))),
        Err(error) => Ok(Outcome::Skipped(format!("verification: {error}"))),
    }
}

fn spell(start: &State, line: &Line) -> Result<Vec<Vec<String>>> {
    let mut spelled = Vec::new();
    for actions in &line.lines {
        let mut state = start.clone();
        let mut tokens = Vec::new();
        for action in actions {
            tokens.push(notation::notation(&state, action));
            notation::play(&mut state, action)?;
        }
        if state.game_status != GameStatus::Finished(GameResult::Winner(start.turn_color)) {
            bail!("a line does not end in a win");
        }
        spelled.push(tokens);
    }
    Ok(spelled)
}

/// Pages from `pdfinfo -url` output (the diagram links) and `pdftotext -layout` text.
pub fn read_pages(urls: &str, text: &str) -> Vec<BookPage> {
    let mut pages: Vec<BookPage> = Vec::new();
    for line in urls.lines() {
        let Some(at) = line.find("board=") else {
            continue;
        };
        let Some(title_at) = line.find("title=Puzzle ") else {
            continue;
        };
        let board = line[at + "board=".len()..]
            .split('&')
            .next()
            .unwrap_or_default()
            .to_string();
        let number: usize = line[title_at + "title=Puzzle ".len()..]
            .trim()
            .parse()
            .unwrap_or(0);
        pages.push(BookPage {
            number,
            board,
            ..BookPage::default()
        });
    }
    let (puzzles_text, rest) = text.split_once("\nHints\n").unwrap_or((text, ""));
    let hints_text = rest.split("\nSolutions\n").next().unwrap_or_default();
    let mut level = String::new();
    let lines: Vec<&str> = puzzles_text.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        if let Some(found) = line.split("Difficulty: ").nth(1) {
            level = found.trim().to_string();
        }
        let Some(number) = line
            .trim()
            .strip_prefix("Puzzle #")
            .and_then(|n| n.trim().parse::<usize>().ok())
        else {
            continue;
        };
        let Some(page) = pages.iter_mut().find(|p| p.number == number) else {
            continue;
        };
        page.level = level.clone();
        let mut cursor = index + 1;
        if let Some(by) = lines.get(cursor).and_then(|l| l.trim().strip_prefix("by ")) {
            page.authors = split_authors(by);
            cursor += 1;
        }
        let mut objective = Vec::new();
        for next in lines.iter().skip(cursor) {
            let next = next.trim();
            if next.is_empty() && !objective.is_empty() {
                break;
            }
            if next == "RESERVE" {
                break;
            }
            if !next.is_empty() {
                objective.push(next);
            }
        }
        page.objective = objective.join(" ");
        page.label = lines
            .iter()
            .skip(cursor)
            .take(40)
            .map(|l| l.trim())
            .find(|l| l.starts_with("Hive"))
            .unwrap_or("Hive")
            .to_string();
    }
    let hint_lines: Vec<&str> = hints_text.lines().collect();
    for (index, line) in hint_lines.iter().enumerate() {
        let Some(number) = line
            .trim()
            .strip_prefix("Puzzle #")
            .and_then(|n| n.trim().parse::<usize>().ok())
        else {
            continue;
        };
        let mut hint = Vec::new();
        for next in hint_lines.iter().skip(index + 1) {
            let next = next.trim();
            if next.starts_with("Puzzle #") || next.starts_with("Hive Puzzles - Volume 1") {
                break;
            }
            if !next.is_empty() && next.parse::<u32>().is_err() {
                hint.push(next);
            }
        }
        if let Some(page) = pages.iter_mut().find(|p| p.number == number) {
            if !hint.is_empty() {
                page.hint = Some(hint.join(" "));
            }
        }
    }
    pages.sort_by_key(|page| page.number);
    pages
}

/// `by A, B and C` - with the page number pdftotext glues onto the line's end removed.
fn split_authors(by: &str) -> Vec<String> {
    let cleaned = by.replace(", and ", ", ").replace(" and ", ", ");
    cleaned
        .split(',')
        .map(|name| {
            name.trim()
                .trim_start_matches("edit by ")
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string()
        })
        .map(|name| strip_page_number(&name))
        .filter(|name| !name.is_empty() && name != "edit" && name != "by")
        .collect()
}

fn strip_page_number(name: &str) -> String {
    // "image134" on page 4 is image13: names in the book never end in a run of 2+ digits that
    // also matches a page, so drop trailing digits beyond a known author's own.
    const KNOWN: [&str; 2] = ["image13", "Eucalyx"];
    for known in KNOWN {
        if name.starts_with(known) {
            return known.to_string();
        }
    }
    name.trim_end_matches(|c: char| c.is_ascii_digit())
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stacks_last_move_and_reserve() {
        let parsed = parse_entomology(":b+2-1wB1bB1+0+1bG2+0-2*@bA**@bG").expect("parses");
        assert_eq!(parsed.to_move, Color::Black);
        assert_eq!(parsed.last_moved, Some((2, -1)));
        assert_eq!(parsed.stacks.len(), 2);
        assert_eq!(parsed.stacks[0].0, (0, 1));
        assert_eq!(parsed.stacks[0].1.len(), 2, "wB1 sits on bB1");
        assert_eq!(parsed.reserve.get(&(Color::Black, Bug::Ant)), Some(&1));
        assert_eq!(
            parsed.reserve.get(&(Color::Black, Bug::Grasshopper)),
            Some(&2)
        );
    }

    #[test]
    fn claimed_length_comes_from_the_objective() {
        assert_eq!(
            claimed_moves("White moves next. Find a sequence for White to win in 4 turns."),
            Some(4)
        );
        assert_eq!(claimed_moves("Black moves next. Who wins and how?"), None);
    }

    #[test]
    fn authors_lose_glued_page_numbers() {
        assert_eq!(split_authors("image134"), vec!["image13"]);
        assert_eq!(
            split_authors("HappyKiwi, JonasK, and eSedano"),
            vec!["HappyKiwi", "JonasK", "eSedano"]
        );
        assert_eq!(
            split_authors("image13, edit by eSedano3"),
            vec!["image13", "eSedano"]
        );
    }
}
