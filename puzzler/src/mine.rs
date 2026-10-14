use crate::{
    corpus::CorpusGame,
    game::Game,
    line::{self, Line},
    notation,
    prove::Prover,
};
use hive_lib::{Color, GameResult, GameStatus, State};
use shared_types::{PuzzleImport, PuzzleKind};

pub const AUTHOR: &str = "philanthus";
const MISSED_SPACING: usize = 4;

#[derive(Debug, Clone, Copy)]
pub struct MineConfig {
    /// Longest forced win searched for near the end of a game, in plies (odd).
    pub max_plies: u32,
    /// How many plies before the end of a game get the full `max_plies` search.
    pub window: usize,
    /// Node budget per position; positions over budget are skipped, never guessed.
    pub budget: u64,
    /// The rest of the game is only searched this deep, which is cheap and finds missed wins.
    pub early_plies: u32,
    /// After the scan, the position before the earliest found win is tried at this depth.
    pub extend_plies: u32,
    pub extend_budget: u64,
    /// Plies a game must have before its positions count (openings make poor puzzles).
    pub min_ply: usize,
}

impl Default for MineConfig {
    fn default() -> Self {
        Self {
            max_plies: 5,
            window: 20,
            budget: 200_000,
            early_plies: 3,
            extend_plies: 7,
            extend_budget: 1_500_000,
            min_ply: 12,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MineStats {
    pub positions: u64,
    pub wins: u64,
    pub over_budget: u64,
    pub ambiguous: u64,
    /// Wins in one that only fill the Queen's last gap, left out on purpose.
    pub plain: u64,
    pub puzzles: u64,
    pub nodes: u64,
}

impl MineStats {
    pub fn add(&mut self, other: &MineStats) {
        self.positions += other.positions;
        self.wins += other.wins;
        self.over_budget += other.over_budget;
        self.ambiguous += other.ambiguous;
        self.plain += other.plain;
        self.puzzles += other.puzzles;
        self.nodes += other.nodes;
    }
}

struct Scanned {
    ply: usize,
    state: State,
    plies: u32,
}

fn winner(game: &CorpusGame) -> Option<Color> {
    match game.game_status.as_str() {
        "Finished(1-0)" => Some(Color::White),
        "Finished(0-1)" => Some(Color::Black),
        _ => None,
    }
}

fn finished(state: &State) -> bool {
    matches!(
        state.game_status,
        GameStatus::Finished(_) | GameStatus::Adjudicated
    )
}

fn search(
    prover: &mut Prover,
    state: &State,
    plies: u32,
    budget: u64,
    stats: &mut MineStats,
) -> Option<u32> {
    stats.positions += 1;
    prover.reset_budget(budget);
    let found = prover.shortest_win(&mut Game::from_state(state), plies);
    stats.nodes += prover.nodes;
    match found {
        Ok(Some((plies, _))) => {
            stats.wins += 1;
            Some(plies)
        }
        Ok(None) => None,
        Err(_) => {
            stats.over_budget += 1;
            None
        }
    }
}

/// Puzzles from one game: the earliest unique position of the winner's final forced win (a
/// "found" win), and positions where the side to move had a forced win and went on to not win the
/// game (a "missed" win).
pub fn mine_game(
    prover: &mut Prover,
    game: &CorpusGame,
    config: &MineConfig,
    version: &str,
) -> (Vec<PuzzleImport>, MineStats) {
    let mut stats = MineStats::default();
    let moves = game.moves();
    if moves.len() < config.min_ply {
        return (Vec::new(), stats);
    }
    let Some(states) = replay(game, &moves, config.min_ply) else {
        return (Vec::new(), stats);
    };
    let window_start = moves.len().saturating_sub(config.window);

    let mut scanned: Vec<Scanned> = Vec::new();
    for (offset, state) in states.iter().enumerate() {
        let ply = config.min_ply + offset;
        if finished(state) {
            continue;
        }
        let (plies, budget) = if ply >= window_start {
            (config.max_plies, config.budget)
        } else {
            (config.early_plies, config.budget)
        };
        if let Some(plies) = search(prover, state, plies, budget, &mut stats) {
            scanned.push(Scanned {
                ply,
                state: state.clone(),
                plies,
            });
        }
    }

    let game_winner = winner(game);
    let mut puzzles = Vec::new();

    // The found win: earliest unique position of the winner's final run, one winner move apart.
    if let Some(winner) = game_winner {
        let mut chain: Vec<&Scanned> = Vec::new();
        let mut next = scanned
            .iter()
            .filter(|c| c.state.turn_color == winner && c.ply >= window_start)
            .map(|c| c.ply)
            .max();
        while let Some(ply) = next {
            let Some(candidate) = scanned.iter().find(|c| c.ply == ply) else {
                break;
            };
            chain.push(candidate);
            next = ply.checked_sub(2);
        }
        let mut extended = None;
        if let Some(earliest) = chain.last() {
            if let Some(ply) = earliest
                .ply
                .checked_sub(2)
                .filter(|ply| *ply >= config.min_ply)
            {
                let state = &states[ply - config.min_ply];
                if !finished(state) {
                    extended = search(
                        prover,
                        state,
                        config.extend_plies,
                        config.extend_budget,
                        &mut stats,
                    )
                    .filter(|plies| *plies > earliest.plies)
                    .map(|plies| Scanned {
                        ply,
                        state: state.clone(),
                        plies,
                    });
                }
            }
        }
        let candidates = extended.iter().chain(chain.iter().rev().copied());
        for candidate in candidates {
            if let Some(puzzle) = build(prover, game, &moves, candidate, false, version, &mut stats)
            {
                puzzles.push(puzzle);
                break;
            }
        }
    }

    // Missed wins: the mover had a forced win and did not win the game. Neighbouring positions of
    // one miss are the same idea, so they must be at least four plies apart.
    let mut last_missed: Option<usize> = None;
    for candidate in &scanned {
        if game_winner == Some(candidate.state.turn_color)
            || last_missed.is_some_and(|ply| candidate.ply < ply + MISSED_SPACING)
        {
            continue;
        }
        if let Some(puzzle) = build(prover, game, &moves, candidate, true, version, &mut stats) {
            last_missed = Some(candidate.ply);
            puzzles.push(puzzle);
        }
    }
    stats.puzzles = puzzles.len() as u64;
    (puzzles, stats)
}

/// States after `first..=moves.len()` moves, or None if the record does not replay.
fn replay(game: &CorpusGame, moves: &[&str], first: usize) -> Option<Vec<State>> {
    let mut state = State::new(game.game_type.parse().ok()?, game.tournament_queen_rule);
    state.set_replaying(true);
    let mut states = Vec::new();
    for (ply, token) in moves.iter().enumerate() {
        if ply >= first {
            states.push(live(&state));
        }
        let (piece, position) = shared_types::split_move(token);
        state.play_turn_from_history(piece, position).ok()?;
    }
    states.push(live(&state));
    Some(states)
}

fn live(state: &State) -> State {
    let mut live = state.clone();
    live.set_replaying(false);
    live
}

fn build(
    prover: &mut Prover,
    game: &CorpusGame,
    moves: &[&str],
    candidate: &Scanned,
    missed: bool,
    version: &str,
    stats: &mut MineStats,
) -> Option<PuzzleImport> {
    let search = Game::from_state(&candidate.state);
    let line: Line = match line::solve(prover, &search, candidate.plies) {
        Ok(Ok(line)) => line,
        Ok(Err(_)) => {
            stats.ambiguous += 1;
            return None;
        }
        Err(_) => {
            stats.over_budget += 1;
            return None;
        }
    };
    let mut lines = spell(&candidate.state, &line)?;
    let solution = lines.remove(0);
    let history: String = moves[..candidate.ply]
        .iter()
        .map(|token| format!("{};", token.trim()))
        .collect();
    let solver_moves = (candidate.plies as usize).div_ceil(2);
    let mut themes = vec![format!("winIn{solver_moves}")];
    if missed {
        themes.push("missedWin".to_string());
    }
    themes.extend(motifs(&search, &line.lines[0]));
    let puzzle = PuzzleImport {
        kind: PuzzleKind::WinIn,
        game_type: game.game_type.clone(),
        history: Some(history),
        position: None,
        solution,
        variations: lines,
        themes,
        authors: vec![AUTHOR.to_string()],
        source: format!(
            "puzzler {version}: philanthus forced-win search, proven in {} plies",
            candidate.plies
        ),
        source_game: Some(game.nanoid.clone()),
        collection: None,
        title: None,
        hint: None,
        level: None,
        rating: Some(seed_rating(solver_moves, missed)),
    };
    let mut puzzle = puzzle;
    match puzzle.verify() {
        Ok(_) => match crate::select::verdict(&mut puzzle) {
            Ok(crate::select::Verdict::Keep) => Some(puzzle),
            Ok(crate::select::Verdict::Plain) => {
                stats.plain += 1;
                None
            }
            Err(error) => {
                tracing::warn!(game = game.nanoid, ply = candidate.ply, %error, "puzzle could not be judged");
                None
            }
        },
        Err(error) => {
            tracing::warn!(game = game.nanoid, ply = candidate.ply, %error, "puzzle failed verification");
            None
        }
    }
}

/// Notation for a line, played out on a real `State` so every token is the site's own spelling.
fn spell(start: &State, line: &Line) -> Option<Vec<Vec<String>>> {
    let solver = start.turn_color;
    let mut spelled = Vec::new();
    for actions in &line.lines {
        let mut state = start.clone();
        let mut tokens = Vec::new();
        for action in actions {
            tokens.push(notation::notation(&state, action));
            notation::play(&mut state, action).ok()?;
        }
        if state.game_status != GameStatus::Finished(GameResult::Winner(solver)) {
            return None;
        }
        spelled.push(tokens);
    }
    Some(spelled)
}

/// Where Glicko starts a puzzle before anyone has played it: longer forced wins, and wins the
/// player at the board missed, are harder.
pub fn seed_rating(solver_moves: usize, missed: bool) -> f64 {
    let base = match solver_moves {
        0 | 1 => 1000.0,
        2 => 1300.0,
        3 => 1550.0,
        4 => 1750.0,
        _ => 1900.0,
    };
    if missed {
        base + 100.0
    } else {
        base
    }
}

/// Motifs read off the main line's solver moves, for filtering and for the result screen.
fn motifs(start: &Game, actions: &[crate::game::Action]) -> Vec<String> {
    use crate::game::Action;
    use hive_lib::Bug;
    let solver = start.to_move;
    let enemy_queen = Game::queen(solver.opposite_color());
    let mut game = start.clone();
    let mut themes = Vec::new();
    let mut add = |theme: &str| {
        if !themes.iter().any(|t| t == theme) {
            themes.push(theme.to_string());
        }
    };
    for (index, action) in actions.iter().enumerate() {
        if index % 2 == 0 {
            match action {
                Action::Move { piece, to, .. } => {
                    if piece.color() != solver {
                        add("throw");
                        if *piece == enemy_queen {
                            add("queenThrow");
                        }
                    }
                    if piece.bug() == Bug::Beetle && game.board.occupied(*to) {
                        add("climb");
                    }
                }
                Action::Place { .. } => {}
                Action::Pass => {}
            }
            if index == 0 && actions.len() > 1 && crate::select::quiet_first_move(&game, action) {
                add("quietMove");
            }
        }
        game.make(action);
    }
    themes
}
