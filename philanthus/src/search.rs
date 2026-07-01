use std::time::{Duration, Instant};

use hudsoni::{Bug, Color, GameResult, Piece, Position};

use crate::{
    eval::{evaluate, INF, WIN},
    game::{Action, Game},
    tt::{Bound, Entry, TranspositionTable},
};

const MAX_DEPTH: u32 = 64;
const MATE_SCORE: i32 = WIN - 1000;
const TT_BITS: u32 = 16;
const ACTION_ORDER_SCORES: [i32; 9] = [i32::MAX, 120, 100, 90, 70, 20, 0, -10, -30];

pub struct Limits {
    pub depth: Option<u32>,
    pub time: Option<Duration>,
}

impl Limits {
    pub fn depth(depth: u32) -> Self {
        Self {
            depth: Some(depth),
            time: None,
        }
    }

    pub fn time(time: Duration) -> Self {
        Self {
            depth: None,
            time: Some(time),
        }
    }
}

pub struct Outcome {
    pub action: Action,
    pub score: i32,
    pub completed_depth: u32,
    pub nodes: u64,
}

pub fn search_outcome(game: &mut Game, limits: Limits) -> Option<Outcome> {
    run(game, limits, TranspositionTable::new(TT_BITS))
}

fn run(game: &mut Game, limits: Limits, tt: TranspositionTable) -> Option<Outcome> {
    let mut searcher = Searcher {
        deadline: limits.time.map(|budget| Instant::now() + budget),
        nodes: 0,
        stopped: false,
        can_abort: false,
        tt,
        pool: Vec::new(),
        target_buffer: Vec::new(),
    };
    let max_depth = limits.depth.unwrap_or(MAX_DEPTH).max(1);

    let mut best: Option<(Action, i32)> = None;
    let mut completed_depth = 0;
    for depth in 1..=max_depth {
        match searcher.run_root(game, depth) {
            Some((action, score)) => {
                best = Some((action, score));
                completed_depth = depth;
                searcher.can_abort = true;
                if score.abs() >= MATE_SCORE {
                    break;
                }
            }
            None => break,
        }
    }
    let nodes = searcher.nodes;
    best.map(|(action, score)| Outcome {
        action,
        score,
        completed_depth,
        nodes,
    })
}

struct Searcher {
    deadline: Option<Instant>,
    nodes: u64,
    stopped: bool,
    can_abort: bool,
    tt: TranspositionTable,
    pool: Vec<Vec<Action>>,
    target_buffer: Vec<Position>,
}

impl Searcher {
    fn take_buffer(&mut self) -> Vec<Action> {
        self.pool.pop().unwrap_or_default()
    }

    fn give_buffer(&mut self, mut buffer: Vec<Action>) {
        buffer.clear();
        self.pool.push(buffer);
    }

    fn take_target_buffer(&mut self) -> Vec<Position> {
        std::mem::take(&mut self.target_buffer)
    }

    fn give_target_buffer(&mut self, mut buffer: Vec<Position>) {
        buffer.clear();
        self.target_buffer = buffer;
    }

    fn should_stop(&mut self) -> bool {
        if self.stopped {
            return true;
        }
        if !self.can_abort {
            return false;
        }
        if self.nodes % 1024 == 0 {
            if let Some(deadline) = self.deadline {
                self.stopped = Instant::now() >= deadline;
            }
        }
        self.stopped
    }

    fn order_actions(&mut self, game: &Game, actions: &mut [Action], tt_move: Option<Action>) {
        if actions.len() < 2 {
            return;
        }

        let us = game.turn_color;
        let opp_queen = queen_position(game, us.opposite_color());
        let own_queen = queen_position(game, us);
        let mut ranks = Vec::with_capacity(actions.len());
        let mut counts = [0_usize; ACTION_ORDER_SCORES.len()];

        for action in actions.iter() {
            let rank = action_order_rank(action, tt_move, opp_queen, own_queen);
            ranks.push(rank as u8);
            counts[rank] += 1;
        }

        let mut starts = [0_usize; ACTION_ORDER_SCORES.len()];
        let mut next = 0;
        for (start, count) in starts.iter_mut().zip(counts) {
            *start = next;
            next += count;
        }
        let mut offsets = starts;

        let mut ordered = self.take_buffer();
        ordered.resize(actions.len(), Action::Pass);
        for (action, rank) in actions.iter().copied().zip(ranks) {
            let rank = rank as usize;
            ordered[offsets[rank]] = action;
            offsets[rank] += 1;
        }
        actions.copy_from_slice(&ordered[..actions.len()]);
        self.give_buffer(ordered);
    }

    fn leaf_score(&mut self, game: &mut Game, ply: i32) -> i32 {
        self.nodes += 1;
        if self.should_stop() {
            return 0;
        }
        if game.is_terminal() {
            terminal_score(game, ply)
        } else {
            evaluate(game)
        }
    }

    fn search_depth_one_action(
        &mut self,
        game: &mut Game,
        action: Action,
        alpha: &mut i32,
        beta: i32,
        ply: i32,
        value: &mut i32,
        best_action: &mut Option<Action>,
    ) -> bool {
        let reversal = game.make_with_pinned_update(&action, false);
        let score = -self.leaf_score(game, ply + 1);
        game.unmake(reversal);
        if self.stopped {
            return false;
        }
        if score > *value {
            *value = score;
            *best_action = Some(action);
        }
        if *value > *alpha {
            *alpha = *value;
        }
        *alpha < beta
    }

    fn negamax_depth_one(
        &mut self,
        game: &mut Game,
        mut alpha: i32,
        beta: i32,
        ply: i32,
        key: u64,
        alpha_orig: i32,
    ) -> i32 {
        let color = game.turn_color;
        let mut value = -INF;
        let mut best_action = None;
        let mut saw_action = false;
        let mut targets = self.take_target_buffer();

        'actions: {
            if game.board.queen_played(color) {
                for offset in 0..game.board.positions.len() {
                    let Some(pos) = game.board.positions[offset] else {
                        continue;
                    };
                    let piece = game.board.offset_to_piece(offset);
                    if game.board.top_piece(pos) != Some(piece) {
                        continue;
                    }
                    if !piece.is_color(color) || game.board.last_moved == Some((piece, pos)) {
                        continue;
                    }

                    if !game.board.is_pinned(piece) {
                        Bug::normal_moves_into(pos, &game.board, &mut targets);
                        for i in 0..targets.len() {
                            saw_action = true;
                            let action = Action::Move(piece, pos, targets[i]);
                            if !self.search_depth_one_action(
                                game,
                                action,
                                &mut alpha,
                                beta,
                                ply,
                                &mut value,
                                &mut best_action,
                            ) {
                                if self.stopped {
                                    self.give_target_buffer(targets);
                                    return 0;
                                }
                                break 'actions;
                            }
                        }
                    }

                    if piece_can_throw(piece, pos, game) {
                        for source in pos.positions_around() {
                            let Some(thrown) = game.board.top_piece(source) else {
                                continue;
                            };
                            if game.board.last_moved == Some((thrown, source)) {
                                continue;
                            }
                            for target in pos.positions_around() {
                                if !Bug::can_throw(pos, source, target, &game.board) {
                                    continue;
                                }
                                saw_action = true;
                                let action = Action::Move(thrown, source, target);
                                if !self.search_depth_one_action(
                                    game,
                                    action,
                                    &mut alpha,
                                    beta,
                                    ply,
                                    &mut value,
                                    &mut best_action,
                                ) {
                                    if self.stopped {
                                        self.give_target_buffer(targets);
                                        return 0;
                                    }
                                    break 'actions;
                                }
                            }
                        }
                    }
                }
            }

            let must_place_queen = game.board.queen_required(game.turn, color);
            let queen_banned = game.tournament && game.turn < 2;
            let mut placeable: [Option<Piece>; 8] = [None; 8];
            let mut count = 0;
            game.board
                .for_each_placeable_piece(color, game.game_type, |piece| {
                    if piece.bug() == Bug::Queen && queen_banned {
                        return;
                    }
                    if must_place_queen && piece.bug() != Bug::Queen {
                        return;
                    }
                    placeable[count] = Some(piece);
                    count += 1;
                });
            if count > 0 {
                targets.clear();
                targets.extend(game.board.spawnable_positions(color));
                for i in 0..targets.len() {
                    let to = targets[i];
                    for piece in placeable.iter().take(count).flatten() {
                        saw_action = true;
                        let action = Action::Place(*piece, to);
                        if !self.search_depth_one_action(
                            game,
                            action,
                            &mut alpha,
                            beta,
                            ply,
                            &mut value,
                            &mut best_action,
                        ) {
                            if self.stopped {
                                self.give_target_buffer(targets);
                                return 0;
                            }
                            break 'actions;
                        }
                    }
                }
            }

            if !saw_action && game.board.is_shutout(color, game.game_type) {
                let action = Action::Pass;
                saw_action = true;
                if !self.search_depth_one_action(
                    game,
                    action,
                    &mut alpha,
                    beta,
                    ply,
                    &mut value,
                    &mut best_action,
                ) && self.stopped
                {
                    self.give_target_buffer(targets);
                    return 0;
                }
            }
        }

        self.give_target_buffer(targets);

        if !saw_action {
            return evaluate(game);
        }

        let bound = if value <= alpha_orig {
            Bound::Upper
        } else if value >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };
        self.tt.store(Entry {
            key,
            depth: 1,
            score: to_tt_score(value, ply),
            bound,
            best: best_action,
        });
        value
    }

    fn run_root(&mut self, game: &mut Game, depth: u32) -> Option<(Action, i32)> {
        let key = game.hash;
        let tt_move = self.tt.probe(key).and_then(|entry| entry.best);
        let mut actions = self.take_buffer();
        game.legal_actions_into(&mut actions);
        // At depth 1, children are evaluated immediately, so ordering only affects
        // alpha-beta efficiency. With the current cheap evaluator, skipping ordering
        // is faster despite visiting more nodes. Revisit if evaluation becomes
        // expensive or starts depending on legal move generation / pinned state.
        if depth > 1 {
            self.order_actions(game, &mut actions, tt_move);
        }
        let mut best: Option<(Action, i32)> = None;
        let mut alpha = -INF;
        for &action in &actions {
            let reversal = game.make_with_pinned_update(&action, depth > 1);
            let score = -self.negamax(game, depth - 1, -INF, -alpha, 1);
            game.unmake(reversal);
            if self.stopped {
                return None;
            }
            if best.is_none() || score > alpha {
                alpha = score;
                best = Some((action, score));
            }
        }
        self.give_buffer(actions);
        if let Some((action, score)) = best {
            self.tt.store(Entry {
                key,
                depth,
                score: to_tt_score(score, 0),
                bound: Bound::Exact,
                best: Some(action),
            });
        }
        best
    }

    fn negamax(&mut self, game: &mut Game, depth: u32, mut alpha: i32, beta: i32, ply: i32) -> i32 {
        self.nodes += 1;
        if self.should_stop() {
            return 0;
        }
        if game.is_terminal() {
            return terminal_score(game, ply);
        }
        if depth == 0 {
            return evaluate(game);
        }

        let key = game.hash;
        let alpha_orig = alpha;
        let mut tt_move = None;
        if let Some(entry) = self.tt.probe(key) {
            tt_move = entry.best;
            if entry.depth >= depth {
                let score = from_tt_score(entry.score, ply);
                match entry.bound {
                    Bound::Exact => return score,
                    Bound::Lower if score >= beta => return score,
                    Bound::Upper if score <= alpha => return score,
                    _ => {}
                }
            }
        }

        if depth == 1 {
            return self.negamax_depth_one(game, alpha, beta, ply, key, alpha_orig);
        }

        let mut actions = self.take_buffer();
        game.legal_actions_into(&mut actions);
        if actions.is_empty() {
            self.give_buffer(actions);
            return evaluate(game);
        }
        // At depth 1, children are evaluated immediately, so ordering only affects
        // alpha-beta efficiency. With the current cheap evaluator, skipping ordering
        // is faster despite visiting more nodes. Revisit if evaluation becomes
        // expensive or starts depending on legal move generation / pinned state.
        if depth > 1 {
            self.order_actions(game, &mut actions, tt_move);
        }
        let mut value = -INF;
        let mut best_action = None;
        for &action in &actions {
            let reversal = game.make_with_pinned_update(&action, depth > 1);
            let score = -self.negamax(game, depth - 1, -beta, -alpha, ply + 1);
            game.unmake(reversal);
            if self.stopped {
                return 0;
            }
            if score > value {
                value = score;
                best_action = Some(action);
            }
            if value > alpha {
                alpha = value;
            }
            if alpha >= beta {
                break;
            }
        }
        self.give_buffer(actions);

        let bound = if value <= alpha_orig {
            Bound::Upper
        } else if value >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };
        self.tt.store(Entry {
            key,
            depth,
            score: to_tt_score(value, ply),
            bound,
            best: best_action,
        });
        value
    }
}

fn to_tt_score(score: i32, ply: i32) -> i32 {
    if score >= MATE_SCORE {
        score + ply
    } else if score <= -MATE_SCORE {
        score - ply
    } else {
        score
    }
}

fn from_tt_score(score: i32, ply: i32) -> i32 {
    if score >= MATE_SCORE {
        score - ply
    } else if score <= -MATE_SCORE {
        score + ply
    } else {
        score
    }
}

fn queen_position(game: &Game, color: Color) -> Option<Position> {
    game.board
        .position_of_piece(Piece::new_from(Bug::Queen, color, 0))
}

fn piece_can_throw(piece: Piece, position: Position, game: &Game) -> bool {
    match piece.bug() {
        Bug::Pillbug => true,
        Bug::Mosquito => {
            game.board.level(position) == 1 && game.board.neighbor_is_a(position, Bug::Pillbug)
        }
        _ => false,
    }
}

fn action_order_rank(
    action: &Action,
    tt_move: Option<Action>,
    opp_queen: Option<Position>,
    own_queen: Option<Position>,
) -> usize {
    let key = action_order_key(action, tt_move, opp_queen, own_queen);
    for (rank, score) in ACTION_ORDER_SCORES.iter().enumerate() {
        if key == *score {
            return rank;
        }
    }
    debug_assert!(false, "unexpected action order key {key}");
    ACTION_ORDER_SCORES
        .iter()
        .position(|score| key > *score)
        .unwrap_or(ACTION_ORDER_SCORES.len() - 1)
}

fn action_order_key(
    action: &Action,
    tt_move: Option<Action>,
    opp_queen: Option<Position>,
    own_queen: Option<Position>,
) -> i32 {
    if tt_move == Some(*action) {
        return i32::MAX;
    }
    let (from, to) = match action {
        Action::Move(_, from, to) => (Some(*from), Some(*to)),
        Action::Place(_, to) => (None, Some(*to)),
        Action::Pass => (None, None),
    };
    let mut score = 0;
    if let Some(to) = to {
        if opp_queen.is_some_and(|queen| adjacent(to, queen)) {
            score += 100;
        }
        if own_queen.is_some_and(|queen| adjacent(to, queen)) {
            score -= 30;
        }
    }
    if let Some(from) = from {
        if own_queen.is_some_and(|queen| adjacent(from, queen)) {
            score += 20;
        }
    }
    score
}

fn adjacent(a: Position, b: Position) -> bool {
    a.is_neighbor(b)
}

fn terminal_score(game: &Game, ply: i32) -> i32 {
    match game.result() {
        GameResult::Winner(color) => {
            if color == game.turn_color {
                WIN - ply
            } else {
                ply - WIN
            }
        }
        GameResult::Draw => 0,
        GameResult::Unknown => evaluate(game),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hudsoni::{GameStatus, GameType, History, State};
    use std::time::Duration;

    const PUZZLE_BUDGET: Duration = Duration::from_millis(300);

    #[test]
    fn search_returns_a_legal_action_without_mutating_state() {
        let state = State::new(GameType::MLP, true);
        let mut game = Game::from_state(&state);
        let legal = game.legal_actions();
        let chosen = search_outcome(&mut game, Limits::depth(2))
            .map(|outcome| outcome.action)
            .expect("opening has legal actions");
        assert!(legal.contains(&chosen));
        assert_eq!(
            game,
            Game::from_state(&state),
            "search must restore the game"
        );
    }

    #[test]
    fn finds_an_immediate_win_in_decided_corpus_games() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../engine/test_pgns/valid");
        let mut tested = 0;
        for entry in std::fs::read_dir(dir).expect("corpus directory") {
            let path = entry.expect("entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("pgn") {
                continue;
            }
            let Ok(history) = History::from_filepath(path.clone()) else {
                continue;
            };
            if history.moves.len() < 2 {
                continue;
            }
            let Ok(full) = State::new_from_history(&history) else {
                continue;
            };
            let winner = match full.game_status {
                GameStatus::Finished(GameResult::Winner(color)) => color,
                _ => continue,
            };

            let mut before_final = history.clone();
            before_final.moves.pop();
            let Ok(state) = State::new_from_history(&before_final) else {
                continue;
            };
            let mut game = Game::from_state(&state);
            if game.is_terminal() {
                continue;
            }
            if winner != game.turn_color {
                continue;
            }
            let action = search_outcome(&mut game, Limits::depth(1))
                .map(|outcome| outcome.action)
                .expect("position has legal moves");
            let mut probe = game.clone();
            probe.make(&action);
            assert_eq!(
                probe.result(),
                GameResult::Winner(winner),
                "search missed an immediate win in {}",
                path.display()
            );
            tested += 1;
        }
        assert!(tested > 0, "no decided corpus games were exercised");
    }

    struct PuzzleStats {
        solved: usize,
        timed_out: usize,
        exhausted: usize,
        total: usize,
        unsolved: Vec<String>,
    }

    fn puzzle_solve_rate(plies: u32) -> PuzzleStats {
        let csv = concat!(env!("CARGO_MANIFEST_DIR"), "/test/puzzles.csv");
        let contents = std::fs::read_to_string(csv).expect("puzzles.csv");
        let mut stats = PuzzleStats {
            solved: 0,
            timed_out: 0,
            exhausted: 0,
            total: 0,
            unsolved: Vec::new(),
        };
        for line in contents.lines() {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            let Some(index) = tokens.iter().position(|token| token.parse::<u32>().is_ok()) else {
                continue;
            };
            if tokens[index].parse::<u32>() != Ok(plies) {
                continue;
            }
            let gamestring = tokens[..index].join(" ");
            let Ok(history) = History::from_uhp_str(gamestring.clone()) else {
                continue;
            };
            let Ok(state) = State::new_from_history(&history) else {
                continue;
            };
            let mut game = Game::from_state(&state);
            if game.is_terminal() {
                continue;
            }
            stats.total += 1;
            let outcome = search_outcome(
                &mut game,
                Limits {
                    depth: Some(plies),
                    time: Some(PUZZLE_BUDGET),
                },
            );
            match outcome {
                Some(outcome) if outcome.score >= MATE_SCORE => stats.solved += 1,
                Some(outcome) if outcome.completed_depth < plies => {
                    stats.timed_out += 1;
                    stats.unsolved.push(format!(
                        "[timeout depth {}/{plies}] {line}",
                        outcome.completed_depth
                    ));
                }
                _ => {
                    stats.exhausted += 1;
                    stats
                        .unsolved
                        .push(format!("[no mate to depth {plies}] {line}"));
                }
            }
        }
        stats
    }

    #[test]
    fn solves_mate_in_one_puzzles() {
        let stats = puzzle_solve_rate(1);
        eprintln!(
            "mate-in-1: {} solved, {} failed of {} (failures: {} timeout, {} no-mate)",
            stats.solved,
            stats.timed_out + stats.exhausted,
            stats.total,
            stats.timed_out,
            stats.exhausted
        );
        for puzzle in stats.unsolved.iter().take(10) {
            eprintln!("  {puzzle}");
        }
        assert!(stats.total > 0, "no mate-in-1 puzzles were loaded");
        assert!(
            stats.solved * 100 >= stats.total * 90,
            "mate-in-1 solve rate {}/{} below 90%",
            stats.solved,
            stats.total
        );
    }

    #[test]
    fn tt_search_matches_plain_search() {
        let csv = concat!(env!("CARGO_MANIFEST_DIR"), "/test/puzzles.csv");
        let contents = std::fs::read_to_string(csv).expect("puzzles.csv");
        let mut checked = 0;
        for line in contents.lines() {
            if checked >= 200 {
                break;
            }
            let tokens: Vec<&str> = line.split_whitespace().collect();
            let Some(index) = tokens.iter().position(|token| token.parse::<u32>().is_ok()) else {
                continue;
            };
            let gamestring = tokens[..index].join(" ");
            let Ok(history) = History::from_uhp_str(gamestring.clone()) else {
                continue;
            };
            let Ok(state) = State::new_from_history(&history) else {
                continue;
            };
            let mut with_tt = Game::from_state(&state);
            if with_tt.is_terminal() {
                continue;
            }
            let mut without_tt = with_tt.clone();
            let scored = run(
                &mut with_tt,
                Limits::depth(3),
                TranspositionTable::new(TT_BITS),
            )
            .map(|outcome| outcome.score);
            let plain = run(
                &mut without_tt,
                Limits::depth(3),
                TranspositionTable::disabled(),
            )
            .map(|outcome| outcome.score);
            assert_eq!(scored, plain, "TT changed the score for {gamestring}");
            checked += 1;
        }
        assert!(checked > 0, "no positions were checked");
    }

    #[test]
    #[ignore = "needs the transposition table to be fast enough"]
    fn reports_deeper_puzzle_solve_rates() {
        for plies in [3_u32, 5, 7] {
            let stats = puzzle_solve_rate(plies);
            eprintln!(
                "mate in {plies} plies: {} solved, {} failed of {} (failures: {} timeout, {} no-mate)",
                stats.solved,
                stats.timed_out + stats.exhausted,
                stats.total,
                stats.timed_out,
                stats.exhausted
            );
            for puzzle in &stats.unsolved {
                eprintln!("  {puzzle}");
            }
        }
    }
}
