use super::{
    curriculum::{find_lesson, lessons, neighbours, CHAPTERS},
    goal::{hex, spec_pieces, Condition, Goal},
    lesson::{Lesson, Reply, Setup, Step},
    session::{Outcome, Session},
};
use hive_lib::{Color, GameType, Piece, Position, State};
use std::collections::HashSet;

const SEARCH_DEPTH: usize = 4;

#[test]
fn lesson_ids_are_unique_and_prefixed_by_their_chapter() {
    let mut seen = HashSet::new();
    for chapter in CHAPTERS {
        for lesson in chapter.lessons {
            assert!(seen.insert(lesson.id), "duplicate lesson id {}", lesson.id);
            assert!(
                lesson.id.starts_with(&format!("{}.", chapter.id)),
                "{} is not under chapter {}",
                lesson.id,
                chapter.id
            );
        }
    }
}

#[test]
fn every_lesson_has_text_and_steps() {
    for lesson in lessons() {
        assert!(!lesson.title.is_empty(), "{} has no title", lesson.id);
        assert!(!lesson.steps.is_empty(), "{} has no steps", lesson.id);
        for step in lesson.steps {
            assert!(
                !step.prompt.is_empty(),
                "{} has a step without a prompt",
                lesson.id
            );
            if !matches!(step.goal, Goal::Continue) {
                assert!(
                    !step.hint.is_empty(),
                    "{} has a playable step without a hint",
                    lesson.id
                );
            }
        }
    }
}

#[test]
fn every_lesson_describes_its_position() {
    for lesson in lessons() {
        assert!(
            !lesson.brief.is_empty(),
            "{} has no position brief",
            lesson.id
        );
    }
}

#[test]
fn every_piece_spec_parses() {
    for lesson in lessons() {
        for step in lesson.steps {
            for spec in step_specs(step) {
                assert!(
                    spec_pieces(spec).is_some(),
                    "{}: bad piece spec {spec:?}",
                    lesson.id
                );
            }
        }
    }
}

#[test]
fn ready_lessons_fill_in_every_hex_and_reply() {
    for lesson in lessons().filter(|lesson| !lesson.is_pending()) {
        for step in lesson.steps {
            match &step.goal {
                Goal::Move { to, .. } | Goal::Lift { to, .. } | Goal::Travel { to, .. } | Goal::MoveExcept { except: to, .. } => {
                    assert!(!to.is_empty(), "{}: Move without a target", lesson.id)
                }
                Goal::Visit { hexes, .. } => {
                    assert!(!hexes.is_empty(), "{}: Visit without stars", lesson.id);
                    assert!(
                        hexes.iter().all(|hex| !hex.is_empty()),
                        "{}: empty star",
                        lesson.id
                    );
                }
                Goal::PlayAny | Goal::LiftAnyOf { .. } | Goal::Continue | Goal::MoveAny | Goal::PlaceAny | Goal::PlaceIn(_) | Goal::LiftAny { .. } | Goal::Explore { .. } | Goal::MoveSatisfying { .. } | Goal::MovePiece { .. } | Goal::Reach(_) => {}
            }
            match step.then {
                Reply::RandomPlacementNear { .. } | Reply::InitialPosition | Reply::LearnerAgain | Reply::NextPlayer(_) | Reply::RandomPreservingWin { .. } => {}
                Reply::Opponent { piece, to } => {
                    assert!(
                        !piece.is_empty() && !to.is_empty(),
                        "{}: scripted reply not filled in",
                        lesson.id
                    );
                }
                Reply::OpponentPlacement { piece, .. } => {
                    assert!(
                        !piece.is_empty(),
                        "{}: scripted placement reply not filled in",
                        lesson.id
                    );
                }
            }
            assert!(
                step.forbidden.iter().chain(step.hint_highlights.iter())
                    .all(|notation| !notation.is_empty()),
                "{}: empty forbidden or hint hex",
                lesson.id
            );
            assert!(
                step.marks.iter().all(|mark| !mark.is_empty()),
                "{}: empty mark",
                lesson.id
            );
        }
    }
}

#[test]
fn ready_lessons_load_and_can_be_finished() {
    for lesson in lessons().filter(|lesson| !lesson.is_pending()) {
        let session = Session::new(lesson)
            .unwrap_or_else(|error| panic!("{} does not load: {error}", lesson.id));
        if let Err(error) = solve(session) {
            panic!("{} cannot be finished: {error}", lesson.id);
        }
    }
}

#[test]
fn opening_highlights_and_arrows_point_at_real_hexes() {
    for lesson in lessons().filter(|lesson| lesson.has_board()) {
        let session = Session::new(lesson)
            .unwrap_or_else(|error| panic!("{} does not load: {error}", lesson.id));
        let Some(step) = session.current_step() else {
            continue;
        };
        assert_eq!(session.forbidden().len(), step.forbidden.len());
        assert!(step.hint_highlights.iter().all(|notation| {
            hex(&session.state().board, notation).is_some()
        }));
        assert_eq!(
            session.highlights().len(),
            step.highlights.len(),
            "{}: a highlight does not resolve",
            lesson.id
        );
        assert_eq!(
            session.arrows().len(),
            expected_arrow_count(&session, step),
            "{}: an arrow does not resolve",
            lesson.id
        );
    }
}

#[test]
fn lessons_without_a_board_only_ask_to_continue() {
    for lesson in lessons().filter(|lesson| matches!(lesson.setup, Setup::TextOnly)) {
        assert!(
            lesson
                .steps
                .iter()
                .all(|step| matches!(step.goal, Goal::Continue) && !step.grid),
            "{} has no board but asks for a move",
            lesson.id
        );
    }
}

#[test]
fn pending_lessons_refuse_to_start() {
    for lesson in lessons().filter(|lesson| lesson.is_pending()) {
        assert!(
            Session::new(lesson).is_err(),
            "{} started without a HOP",
            lesson.id
        );
    }
}

#[test]
fn neighbours_walk_across_chapters() {
    let all: Vec<&Lesson> = lessons().collect();
    let (previous, next) = neighbours(all[0].id);
    assert!(previous.is_none());
    assert_eq!(next.map(|lesson| lesson.id), Some(all[1].id));
    let last = all[all.len() - 1];
    assert_eq!(neighbours(last.id).1.map(|lesson| lesson.id), None);
    let first_of_second_chapter = CHAPTERS[1].lessons[0].id;
    let last_of_first_chapter = CHAPTERS[0].lessons[CHAPTERS[0].lessons.len() - 1].id;
    assert_eq!(
        neighbours(first_of_second_chapter)
            .0
            .map(|lesson| lesson.id),
        Some(last_of_first_chapter)
    );
}

const PLACE_QUEEN: Lesson = Lesson {
    id: "test.place_queen",
    title: "Place the Queen",
    intro: &["test"],
    brief: "test",
    setup: Setup::Position("A+G+S-a-g-s,w"),
    steps: &[
        Step::play(
            "Place the Queen.",
            Goal::Reach(Condition::Placed("wQ")),
            "Queen.",
        ),
        Step::play(
            "Place a Beetle.",
            Goal::Reach(Condition::Placed("wB")),
            "Beetle.",
        ),
    ],
    outro: "",
};

#[test]
fn wrong_moves_stay_visible_until_retry_restarts_the_whole_lesson() {
    let mut session = Session::new(&PLACE_QUEEN).unwrap();
    let initial = session.state().board.clone();
    let (queen, at) = spawn_of(&session, |piece| piece.bug() == hive_lib::Bug::Queen);
    assert_eq!(session.play(queen, at), Outcome::StepDone);
    let (ant, target) = spawn_of(&session, |piece| piece.bug() == hive_lib::Bug::Ant);
    assert_eq!(session.play(ant, target), Outcome::Wrong);
    assert_eq!(session.state().board.position_of_piece(ant), Some(target));
    assert!(session.needs_retry());
    assert!(!session.accepts_moves());
    assert_eq!(session.play(ant, target), Outcome::Illegal);
    session.retry();
    assert_eq!(session.step_index(), 0);
    assert!(session.state().board == initial);
    assert!(!session.needs_retry());
    assert!(session.accepts_moves());
}

#[test]
fn learner_again_hands_the_move_back() {
    let mut session = Session::new(&PLACE_QUEEN).expect("test lesson loads");
    let (piece, position) = spawn_of(&session, |piece| piece.bug() == hive_lib::Bug::Queen);

    assert_eq!(session.play(piece, position), Outcome::StepDone);
    assert_eq!(session.state().turn_color, Color::White);
    assert!(session.accepts_moves());
    let (piece, position) = spawn_of(&session, |piece| piece.bug() == hive_lib::Bug::Beetle);
    assert_eq!(session.play(piece, position), Outcome::LessonDone);
    assert!(session.is_finished());
}

#[test]
fn conditions_read_the_board() {
    let mut state = State::new(GameType::MLP, false);
    for (piece, position) in [("wQ", ""), ("bQ", "-wQ"), ("wA1", "wQ-"), ("bA1", "-bQ")] {
        state
            .play_turn_from_history(piece, position)
            .expect("opening is legal");
    }
    assert!(Condition::Touching("wQ", "bQ").holds(&state));
    assert!(Condition::Touching("wA", "wQ").holds(&state));
    assert!(!Condition::Touching("wA1", "bQ").holds(&state));
    assert!(Condition::Placed("bA").holds(&state));
    assert!(!Condition::Placed("bG").holds(&state));
    assert!(Condition::KillSpotsFilled {
        queen: Color::White,
        at_least: 2
    }
    .holds(&state));
    assert!(!Condition::KillSpotsFilled {
        queen: Color::White,
        at_least: 3
    }
    .holds(&state));
    assert!(!Condition::Sandwiched(Color::White).holds(&state));
    assert!(Condition::Pinned("wQ").holds(&state));
    assert!(!Condition::Surrounded("wQ").holds(&state));
    assert!(Condition::Not(&Condition::Placed("bG")).holds(&state));
}

#[test]
fn welcome_finishes_only_after_the_user_places_for_both_colours() {
    let lesson = find_lesson("basics.welcome").unwrap();
    let mut session = Session::new(lesson).unwrap();
    assert!(session.grid().is_empty());
    assert_eq!(session.state().board.spawnable_positions(Color::White).count(), 1);
    let (white, target) = spawn_of(&session, |piece| piece.to_string() != "wQ");
    assert_eq!(session.play(white, target), Outcome::StepDone);
    assert!(!session.is_finished());
    assert_eq!(session.learner(), Color::Black);
    assert_eq!(session.state().board.all_taken_positions().count(), 1);
    assert!(session.accepts_moves());
    let (black, target) = spawn_of(&session, |piece| piece.to_string() != "bQ");
    assert_eq!(session.play(black, target), Outcome::LessonDone);
    let white_at = session.state().board.position_of_piece(white).unwrap();
    let black_at = session.state().board.position_of_piece(black).unwrap();
    assert!(white_at.is_neighbor(black_at));
    assert_eq!(session.state().turn_color, Color::White);
    session.retry();
    assert_eq!(session.learner(), Color::White);
    assert_eq!(session.state().board.all_taken_positions().count(), 0);
}

#[test]
fn placing_is_second_and_black_replies_with_a_legal_queen() {
    assert_eq!(CHAPTERS[0].lessons[1].id, "basics.placing");
    assert_eq!(CHAPTERS[0].lessons[2].id, "basics.queen_rule");
    assert_eq!(CHAPTERS[0].lessons[3].id, "basics.move_pieces");
    assert_eq!(CHAPTERS[0].lessons[4].id, "basics.one_hive");
    assert_eq!(CHAPTERS[0].lessons[5].id, "basics.surround");
    assert_eq!(CHAPTERS[0].lessons[6].id, "basics.draw");
    assert_eq!(CHAPTERS[1].lessons[0].id, "bugs.queen");
    assert_eq!(CHAPTERS[1].lessons[1].id, "bugs.freedom_to_move");
    assert_eq!(CHAPTERS[4].lessons[0].id, "tactics.gates_and_doors");

    let lesson = find_lesson("basics.placing").expect("placing lesson exists");
    let mut session = Session::new(lesson).expect("placing lesson loads");
    let ant: Piece = "wA1".parse().expect("White Ant parses");
    let target = session
        .state()
        .board
        .spawnable_positions(Color::White)
        .next()
        .expect("White has a legal placement");

    assert_eq!(session.play(ant, target), Outcome::LessonDone);

    let queen: Piece = "bQ".parse().expect("Black Queen parses");
    let queen_at = session
        .state()
        .board
        .position_of_piece(queen)
        .expect("Black replies with the Queen");
    let ladybug: Piece = "bL".parse().expect("Black Ladybug parses");
    let ladybug_at = session
        .state()
        .board
        .position_of_piece(ladybug)
        .expect("Black Ladybug is on the board");

    assert!(queen_at.is_neighbor(ladybug_at));
    for white in ["wL", "wA1"] {
        let piece: Piece = white.parse().expect("White piece parses");
        let position = session
            .state()
            .board
            .position_of_piece(piece)
            .expect("White piece is on the board");
        assert!(!queen_at.is_neighbor(position));
    }
    assert_eq!(session.state().turn_color, Color::White);
}

#[test]
fn queen_follows_the_eight_basics_lessons_and_only_accepts_the_two_move_route() {
    assert_eq!(lessons().nth(8).map(|lesson| lesson.id), Some("bugs.queen"));
    let lesson = find_lesson("bugs.queen").expect("Queen lesson exists");
    let mut session = Session::new(lesson).expect("Queen lesson loads");
    let queen: Piece = "wQ".parse().expect("White Queen parses");
    let first = hex(&session.state().board, "bA2\\").expect("first Queen step resolves");
    let target = hex(&session.state().board, "bA2-").expect("final Queen hex resolves");
    assert_eq!(session.highlights(), vec![target]);
    assert!(session.arrows().is_empty());

    for (piece, position) in legal_moves(session.state()) {
        if piece == queen && position == first {
            continue;
        }
        let mut attempt = session.clone();
        assert_eq!(attempt.play(piece, position), Outcome::Wrong);
        assert_eq!(attempt.step_index(), 0);
        assert!(attempt.needs_retry());
        assert_eq!(attempt.state().board.position_of_piece(piece), Some(position));
    }
    assert_eq!(session.play(queen, first), Outcome::StepDone);
    assert!(!session.is_finished());
    assert_eq!(session.highlights(), vec![target]);
    assert_eq!(session.state().turn_color, Color::White);

    let black_grasshopper: Piece = "bG1".parse().expect("Black Grasshopper parses");
    let black_ladybug: Piece = "bL".parse().expect("Black Ladybug parses");
    let grasshopper_at = session
        .state()
        .board
        .position_of_piece(black_grasshopper)
        .expect("Black placed a Grasshopper");
    let ladybug_at = session
        .state()
        .board
        .position_of_piece(black_ladybug)
        .expect("Black Ladybug is on the board");
    assert!(grasshopper_at.is_neighbor(ladybug_at));

    for (piece, position) in legal_moves(session.state()) {
        if piece == queen && position == target {
            continue;
        }
        let mut attempt = session.clone();
        assert_eq!(attempt.play(piece, position), Outcome::Wrong);
        assert_eq!(attempt.step_index(), 1);
        assert!(attempt.needs_retry());
        assert_eq!(attempt.state().board.position_of_piece(piece), Some(position));
    }
    assert_eq!(session.play(queen, target), Outcome::LessonDone);
    assert!(session.is_finished());
    assert_eq!(session.state().board.position_of_piece(queen), Some(target));
    for ant in ["bA1", "bA2"] {
        assert!(Condition::Pinned(ant).holds(session.state()));
    }
    assert!(!session.current_step().unwrap().completion_tip.is_empty());
}

#[test]
fn how_to_win_has_one_highlight_and_no_arrows() {
    let lesson = find_lesson("basics.surround").expect("How to Win lesson exists");
    assert_eq!(lesson.title, "How to Win");

    let session = Session::new(lesson).expect("How to Win lesson loads");
    assert_eq!(session.highlights().len(), 1);
    assert!(session.arrows().is_empty());
}

#[test]
fn draw_only_accepts_the_grasshopper_move_that_surrounds_both_queens() {
    let lesson = find_lesson("basics.draw").expect("Draw lesson exists");
    let mut session = Session::new(lesson).expect("Draw lesson loads");
    assert_eq!(session.highlights().len(), 1);

    let grasshopper: Piece = "wG1".parse().expect("White Grasshopper parses");
    let target = hex(&session.state().board, "wQ-").expect("draw target resolves");
    let moves = legal_moves(session.state());
    assert!(
        moves.contains(&(grasshopper, target)),
        "wG1 can jump into the hole between the Queens"
    );

    let (wrong_piece, wrong_target) = moves.into_iter()
        .find(|(piece, position)| *piece != grasshopper || *position != target)
        .expect("there is another legal move");
    assert_eq!(session.play(wrong_piece, wrong_target), Outcome::Wrong);
    assert!(!session.is_finished());

    session.retry();
    assert_eq!(session.play(grasshopper, target), Outcome::LessonDone);
    assert!(Condition::Surrounded("wQ").holds(session.state()));
    assert!(Condition::Surrounded("bQ").holds(session.state()));
}

#[test]
fn one_hive_completes_on_a_legal_move_but_not_a_placement() {
    let lesson = find_lesson("basics.one_hive").expect("One Hive lesson exists");
    assert_eq!(lesson.title, "One Hive Rule");

    let mut session = Session::new(lesson).expect("One Hive lesson loads");
    let movers: HashSet<String> = session
        .state()
        .board
        .moves(Color::White)
        .into_iter()
        .filter(|(_, targets)| !targets.is_empty())
        .map(|((piece, _), _)| piece.to_string())
        .collect();
    for movable in ["wQ", "wA1", "wM"] {
        assert!(movers.contains(movable), "{movable} should be able to move");
    }
    for pinned in ["wP", "wG1"] {
        assert!(!movers.contains(pinned), "{pinned} should be pinned");
    }

    let marks = session.marks();
    assert_eq!(marks.len(), 2);
    assert!(marks.iter().all(|mark| mark.number.is_none()));

    let before = session.state().board.clone();
    let (reserve_piece, spawn) = spawn_of(&session, |_| true);
    assert_eq!(session.play(reserve_piece, spawn), Outcome::Wrong);
    assert_eq!(session.state().board.position_of_piece(reserve_piece), Some(spawn));
    assert!(session.needs_retry());
    session.retry();
    assert!(session.state().board == before);
    assert!(!session.is_finished());

    let (piece, target) = session
        .state()
        .board
        .moves(Color::White)
        .into_iter()
        .flat_map(|((piece, _), targets)| {
            targets.into_iter().map(move |target| (piece, target))
        })
        .next()
        .expect("White has a legal movement");

    assert_eq!(session.play(piece, target), Outcome::StepDone);
    assert_eq!(session.learner(), Color::Black);
    assert!(!session.is_finished());
    assert!(session.accepts_moves());
    let (piece, target) = board_moves(session.state()).into_iter().next().unwrap();
    assert_eq!(session.play(piece, target), Outcome::LessonDone);
    assert!(session.is_finished());
}

fn spawn_of(session: &Session, wanted: impl Fn(Piece) -> bool) -> (Piece, Position) {
    let state = session.state();
    let color = state.turn_color;
    let piece = state
        .reserve(color)
        .values()
        .flatten()
        .filter_map(|name| name.parse::<Piece>().ok())
        .find(|piece| wanted(*piece))
        .expect("reserve holds the wanted piece");
    let position = state
        .board
        .spawnable_positions(color)
        .next()
        .expect("there is somewhere to place");
    (piece, position)
}

fn solve(mut session: Session) -> Result<(), String> {
    while !session.is_finished() {
        let step = session.step_index();
        if let Some(current) = session.current_step() {
            if session.highlights().len() != current.highlights.len()
                || session.arrows().len() != expected_arrow_count(&session, current)
                || (!current.immobile_marks && session.marks().len() != current.marks.len())
            {
                return Err(format!(
                    "step {} points at a hex that does not exist",
                    step + 1
                ));
            }
        }
        if session
            .current_step()
            .is_some_and(|step| matches!(step.goal, Goal::Continue))
        {
            session.advance();
            continue;
        }
        if !session.accepts_moves() {
            return Err(format!(
                "step {} is not the learner's move; did the scripted reply fail?",
                step + 1
            ));
        }
        session = search(&session, step, SEARCH_DEPTH).ok_or_else(|| {
            format!(
                "step {} has no solution within {SEARCH_DEPTH} moves",
                step + 1
            )
        })?;
    }
    if let Some(closing) = session.current_step() {
        if !closing.immobile_marks && session.marks().len() != closing.marks.len() {
            return Err("the closing step points at a hex that does not exist".to_string());
        }
        if session.state().turn_color != session.learner() {
            return Err("the scripted reply before the closing step failed".to_string());
        }
    }
    Ok(())
}

fn search(session: &Session, step: usize, depth: usize) -> Option<Session> {
    if let Some(Goal::Explore { pieces, .. }) = session.current_step().map(|step| &step.goal) {
        let key = |session: &Session| pieces.iter().map(|spec| {
            session.state().board.position_of_piece(spec.parse::<Piece>().unwrap())
        }).collect::<Vec<_>>();
        let mut seen = HashSet::from([key(session)]);
        let mut queue = std::collections::VecDeque::from([(session.clone(), 0)]);
        while let Some((current, used)) = queue.pop_front() {
            if used >= depth.max(6) { continue; }
            for (piece, target) in legal_moves(current.state()) {
                let mut next = current.clone();
                match next.play(piece, target) {
                    Outcome::StepDone | Outcome::LessonDone => return Some(next),
                    Outcome::Progress => {
                        if seen.insert(key(&next)) { queue.push_back((next, used + 1)); }
                    }
                    _ => {}
                }
            }
            if seen.len() > 2048 { return None; }
        }
        return None;
    }
    if depth == 0 {
        return None;
    }
    let mut continuations = Vec::new();
    for (piece, position) in legal_moves(session.state()) {
        let mut next = session.clone();
        match next.play(piece, position) {
            Outcome::StepDone | Outcome::LessonDone => return Some(next),
            Outcome::Visited | Outcome::Progress => continuations.push(next),
            Outcome::Illegal | Outcome::Wrong | Outcome::Refuted => {}
        }
    }
    continuations
        .into_iter()
        .find_map(|next| search(&next, step, depth - 1))
}

fn legal_moves(state: &State) -> Vec<(Piece, Position)> {
    let color = state.turn_color;
    let mut moves: Vec<(Piece, Position)> = state
        .board
        .moves(color)
        .into_iter()
        .flat_map(|((piece, _), targets)| targets.into_iter().map(move |target| (piece, target)))
        .collect();
    let spawns: Vec<Position> = state.board.spawnable_positions(color).collect();
    for piece in state
        .reserve(color)
        .values()
        .filter_map(|names| names.first())
        .filter_map(|name| name.parse::<Piece>().ok())
    {
        moves.extend(spawns.iter().map(|position| (piece, *position)));
    }
    moves
}

fn step_specs(step: &Step) -> Vec<&'static str> {
    let mut specs = Vec::new();
    specs.extend(step.legal_arrows);
    match &step.goal {
        Goal::PlayAny | Goal::Continue | Goal::MoveAny | Goal::PlaceAny | Goal::PlaceIn(_) => {}
        Goal::LiftAny { piece, by } => {
            specs.push(piece);
            specs.push(by);
        }
        Goal::Move { piece, .. } | Goal::Lift { piece, .. } | Goal::Travel { piece, .. } | Goal::MoveExcept { piece, .. } | Goal::Visit { piece, .. } => specs.push(*piece),
        Goal::MovePiece { piece, touching } => {
            specs.push(*piece);
            if let Some(other) = touching {
                specs.push(*other);
            }
        }
        Goal::Explore { pieces, condition } => {
            specs.extend(pieces.iter().copied());
            condition_specs(condition, &mut specs);
        }
        Goal::MoveSatisfying { piece, condition } => {
            specs.push(*piece);
            condition_specs(condition, &mut specs);
        }
        Goal::LiftAnyOf { pieces, by } => {
            specs.extend(pieces.iter().copied());
            specs.push(by);
        }
        Goal::Reach(condition) => condition_specs(condition, &mut specs),
    }
    specs
}

fn condition_specs(condition: &Condition, specs: &mut Vec<&'static str>) {
    match condition {
        Condition::AnyPlaced(_) => {}
        Condition::CanMoveTo { piece, .. } => specs.push(piece),
        Condition::Placed(a)
        | Condition::Pinned(a)
        | Condition::Stuck(a)
        | Condition::Surrounded(a)
        | Condition::SpawnBlocked { around: a, .. } => specs.push(a),
        Condition::Touching(a, b) | Condition::OnTop(a, b) => {
            specs.push(a);
            specs.push(b);
        }
        Condition::All(conditions) => conditions
            .iter()
            .for_each(|condition| condition_specs(condition, specs)),
        Condition::Not(condition) => condition_specs(condition, specs),
        Condition::KillSpotsFilled { .. }
        | Condition::DirectDrops { .. }
        | Condition::Sandwiched(_)
        | Condition::ShutOut(_) | Condition::Won(_) => {}
    }
}

const PLACE_THEN_READ: Lesson = Lesson {
    id: "test.place_then_read",
    title: "Place, then read",
    intro: &["test"],
    brief: "test",
    setup: Setup::Position("A+G+S-a-g-s,w"),
    steps: &[
        Step::play(
            "Place the Queen.",
            Goal::Reach(Condition::Placed("wQ")),
            "Queen.",
        ),
        Step::read("Well placed.", &[]),
    ],
    outro: "",
};

#[test]
fn a_closing_read_step_does_not_need_continue() {
    let mut session = Session::new(&PLACE_THEN_READ).expect("test lesson loads");
    let (queen, position) = spawn_of(&session, |piece| piece.bug() == hive_lib::Bug::Queen);

    assert_eq!(session.play(queen, position), Outcome::LessonDone);
    assert!(session.is_finished());
    assert!(!session.awaits_continue());
    assert_eq!(
        session.current_step().map(|step| step.prompt),
        Some("Well placed.")
    );
}

#[test]
fn lessons_with_opponent_pieces_do_not_skip_replies() {
    let mut offenders = Vec::new();
    // This authored solo rescue only gives Black a turn when White springs the trap.
    for lesson in lessons().filter(|lesson| !lesson.uses_blank_tiles() && !matches!(lesson.id, "bugs.pillbug_gate" | "bugs.pillbug_save" | "bugs.mosquito_climb" | "bugs.mosquito_ability")) {
        let moves: Vec<&Step> = lesson
            .steps
            .iter()
            .filter(|step| !matches!(step.goal, Goal::Continue))
            .collect();
        for (index, step) in moves.iter().enumerate() {
            if index + 1 < moves.len() && matches!(step.then, Reply::LearnerAgain) {
                offenders.push(lesson.id);
            }
        }
    }
    offenders.dedup();
    assert!(
        offenders.is_empty(),
        "these lessons make the learner move twice without a reply: {offenders:?}"
    );
}

#[test]
fn every_real_game_lesson_links_its_game() {
    for lesson in lessons().filter(|lesson| {
        lesson.has_board()
            && !lesson.id.starts_with("openings.")
            && !matches!(lesson.id, "basics.pass" | "bugs.mosquito_climb" | "bugs.mosquito_ability" | "bugs.mosquito_other" | "bugs.mosquito" | "bugs.pillbug" | "bugs.pillbug_save" | "bugs.pillbug_enemy" | "bugs.pillbug_gate" | "bugs.pillbug_deactivate" | "bugs.ladybug_gate" | "bugs.grasshopper_place" | "bugs.grasshopper_paths" | "bugs.beetle" | "bugs.beetle_spawn" | "bugs.spider_dead_end" | "bugs.spider_door" | "bugs.spider" | "bugs.spider_power" | "basics.move_pieces" | "bugs.ant" | "bugs.ant_attack" | "bugs.ant_first_moves" | "basics.draw" | "bugs.queen" | "bugs.freedom_to_move" | "bugs.dont_break_the_hive")
            && !matches!(lesson.setup, Setup::Position(",w"))
    }) {
        assert!(
            super::source_game(lesson.id).is_some(),
            "{} has no source game",
            lesson.id
        );
    }
}

#[test]
fn freeing_a_black_piece_in_the_race_gets_punished() {
    let lesson = find_lesson("strategy.race").expect("race lesson exists");
    let mut session = Session::new(lesson).expect("race loads");
    let bad = hive_lib::Position::from_string("-wA3", &session.state().board).expect("hex exists");
    let mosquito: Piece = "wM".parse().expect("piece");

    assert_eq!(session.play(mosquito, bad), Outcome::Refuted);
    assert!(session.is_refuted());
    assert!(!session.accepts_moves());

    session.retry();
    assert!(!session.is_refuted());
    assert!(session.accepts_moves());
}

#[test]
fn queen_hints_survive_full_lesson_retries_and_remain_step_specific() {
    let lesson = find_lesson("bugs.queen").unwrap();
    let mut session = Session::new(lesson).unwrap();
    let queen: Piece = "wQ".parse().unwrap();
    let from = session.state().board.position_of_piece(queen).unwrap();
    let first = hex(&session.state().board, "bA2\\").unwrap();
    let target = hex(&session.state().board, "bA2-").unwrap();
    let wrong = legal_moves(session.state()).into_iter()
        .find(|(piece, position)| *piece != queen || *position != first).unwrap();
    for count in 1..=2 {
        assert_eq!(session.play(wrong.0, wrong.1), Outcome::Wrong);
        assert!(session.arrows().is_empty());
        session.retry();
        assert_eq!(session.arrows().is_empty(), count < 2);
    }
    assert_eq!(session.arrows(), vec![(from, first)]);
    assert_eq!(session.play(queen, first), Outcome::StepDone);
    assert!(session.arrows().is_empty());
    for count in 1..=2 {
        let wrong = legal_moves(session.state()).into_iter()
            .find(|(piece, position)| *piece != queen || *position != target).unwrap();
        assert_eq!(session.play(wrong.0, wrong.1), Outcome::Wrong);
        session.retry();
        assert_eq!(session.step_index(), 0);
        assert_eq!(session.play(queen, first), Outcome::StepDone);
        assert_eq!(session.arrows().is_empty(), count < 2);
    }
    assert_eq!(session.arrows(), vec![(first, target)]);
    assert_eq!(session.play(queen, target), Outcome::LessonDone);
    assert!(session.arrows().is_empty());
    assert!(Session::new(lesson).unwrap().arrows().is_empty());
}

#[test]
fn every_freedom_ant_move_except_the_goal_opens_the_queen_route_after_blacks_reply() {
    assert_eq!(lessons().nth(9).map(|lesson| lesson.id), Some("bugs.freedom_to_move"));
    let lesson = find_lesson("bugs.freedom_to_move").unwrap();
    let initial = Session::new(lesson).unwrap();
    let ant: Piece = "wA2".parse().unwrap();
    let queen: Piece = "wQ".parse().unwrap();
    let black_ant: Piece = "bA2".parse().unwrap();
    let grasshopper: Piece = "bG1".parse().unwrap();
    let target = hex(&initial.state().board, "-wL").unwrap();
    assert_eq!(initial.highlights(), vec![target]);
    assert!(!legal_moves(initial.state()).contains(&(queen, target)));
    let mut accepted = 0;
    let mut fallback = 0;
    for (piece, position) in legal_moves(initial.state()) {
        let mut session = initial.clone();
        if piece != ant || position == target {
            assert_eq!(session.play(piece, position), Outcome::Wrong);
            assert_eq!(session.step_index(), 0);
            assert!(session.needs_retry());
            assert_eq!(session.state().board.position_of_piece(piece), Some(position));
            continue;
        }
        let mut after_ant = initial.state().clone();
        after_ant.play_turn_from_position(ant, position).unwrap();
        let anchor = after_ant.board.position_of_piece(black_ant).unwrap();
        let near_available = after_ant.board.spawnable_positions(Color::Black)
            .any(|position| position.is_neighbor(anchor));
        assert_eq!(session.play(ant, position), Outcome::StepDone);
        assert_eq!(session.state().turn_color, Color::White);
        let reply = session.state().board.position_of_piece(grasshopper).unwrap();
        assert!(after_ant.board.spawnable(Color::Black, reply));
        if near_available { assert!(reply.is_neighbor(anchor)); } else { fallback += 1; }
        assert_eq!(session.highlights(), vec![target]);
        assert_eq!(session.play(queen, target), Outcome::LessonDone,
            "Queen route must stay open after Ant moves to {position:?}");
        accepted += 1;
    }
    assert!(accepted > 1);
    assert!(fallback > 0);
}

#[test]
fn freedom_hint_shows_a_legal_ant_move_after_two_retried_errors() {
    let mut session = Session::new(find_lesson("bugs.freedom_to_move").unwrap()).unwrap();
    let target = session.highlights()[0];
    let ant: Piece = "wA2".parse().unwrap();
    for count in 1..=2 {
        assert_eq!(session.play(ant, target), Outcome::Wrong);
        assert!(session.needs_retry());
        session.retry();
        assert_eq!(session.arrows().is_empty(), count < 2);
    }
    let arrows = session.arrows();
    assert_eq!(arrows.len(), 1);
    assert_eq!(arrows[0].0, session.state().board.position_of_piece(ant).unwrap());
    assert_ne!(arrows[0].1, target);
    assert_eq!(session.play(ant, arrows[0].1), Outcome::StepDone);
    assert!(session.arrows().is_empty());
}

#[test]
fn queen_and_ant_groups_have_the_requested_order() {
    let ids: Vec<_> = lessons().skip(8).take(6).map(|lesson| lesson.id).collect();
    assert_eq!(ids, [
        "bugs.queen", "bugs.freedom_to_move", "bugs.dont_break_the_hive",
        "bugs.ant_first_moves", "bugs.ant", "bugs.ant_attack",
    ]);
    assert_eq!(find_lesson("bugs.ant").unwrap().title, "Gate");
    assert!(find_lesson("bugs.ant_pin").is_none());
    assert_eq!(lessons().count(), 65);
    let queen = super::group_of("bugs.queen").unwrap();
    assert_eq!(queen.title, "The Queen Bee");
    for id in ["bugs.queen", "bugs.freedom_to_move", "bugs.dont_break_the_hive"] {
        assert_eq!(super::group_of(id).unwrap().id, queen.id);
    }
}

#[test]
fn blank_hive_only_allows_white_queen_and_has_no_opponent_responses() {
    let lesson = find_lesson("bugs.dont_break_the_hive").unwrap();
    assert!(lesson.uses_blank_tiles());
    let mut session = Session::new(lesson).unwrap();
    let initial = session.state().board.clone();
    let queen: Piece = "wQ".parse().unwrap();
    let target = hex(&initial, "bQ/").unwrap();
    let first = hex(&initial, "bL/").unwrap();
    assert_eq!(session.highlights(), vec![target]);
    for position in initial.all_taken_positions() {
        let piece = initial.top_piece(position).unwrap();
        assert_eq!(session.can_select(piece), piece == queen);
    }
    for (piece, position) in legal_moves(session.state()) {
        if piece != queen {
            let mut attempt = session.clone();
            assert_eq!(attempt.play(piece, position), Outcome::Illegal);
            assert!(attempt.state().board == initial);
        }
    }
    assert_eq!(session.play(queen, first), Outcome::Progress);
    assert_eq!(session.state().turn_color, Color::White);
    assert!(!session.is_finished());
    assert_eq!(session.play(queen, target), Outcome::LessonDone);
    assert!(Condition::Surrounded("bQ").holds(session.state()));
    for position in initial.all_taken_positions() {
        let piece = initial.top_piece(position).unwrap();
        if piece != queen {
            assert_eq!(session.state().board.position_of_piece(piece), Some(position));
        }
    }
    assert_eq!(session.state().turn_color, Color::White);
    assert_eq!(session.current_step().unwrap().prompt, "Well done!");
    assert_eq!(session.completion_result(), Some("White wins!"));
    assert!(session.current_step().unwrap().completion_tip.is_empty());
}

#[test]
fn blank_hive_illegal_moves_are_not_applied_and_hints_find_the_route() {
    let lesson = find_lesson("bugs.dont_break_the_hive").unwrap();
    let mut session = Session::new(lesson).unwrap();
    let queen: Piece = "wQ".parse().unwrap();
    let initial = session.state().board.clone();
    let from = initial.position_of_piece(queen).unwrap();
    let first = hex(&initial, "bL/").unwrap();
    let target = hex(&initial, "bQ/").unwrap();
    let red = session.forbidden();
    assert_eq!(red.len(), 1);
    assert!(!initial.occupied(red[0]));
    assert!(session.arrows().is_empty());
    for count in 1..=2 {
        assert_eq!(session.play(queen, red[0]), Outcome::Wrong);
        assert!(session.needs_retry());
        assert!(session.state().board == initial);
        session.retry();
        assert_eq!(session.arrows().is_empty(), count < 2);
    }
    assert_eq!(session.arrows(), vec![(from, first)]);
    assert_eq!(session.play(queen, first), Outcome::Progress);
    assert_eq!(session.arrows(), vec![(first, target)]);
    assert_eq!(session.play(queen, target), Outcome::LessonDone);
    assert!(session.arrows().is_empty());
}

#[test]
fn ant_sublessons_share_a_topic_without_changing_ids_or_numbering() {
    let group = super::group_of("bugs.ant").unwrap();
    assert_eq!(group.title, "The Ant");
    assert_eq!(group.lessons, &["bugs.ant_first_moves", "bugs.ant", "bugs.ant_attack"]);
    let chapter = super::chapter_of("bugs.ant").unwrap();
    for id in group.lessons {
        assert_eq!(super::group_of(id).unwrap().id, group.id);
        assert_eq!(super::chapter_of(id).unwrap().id, chapter.id);
    }
    assert_eq!(super::group_of("bugs.queen").unwrap().title, "The Queen Bee");
    assert_eq!(lessons().nth(12).unwrap().id, "bugs.ant");
}

#[test]
fn placing_crosses_cover_only_forbidden_empty_neighbours_of_black_ladybug() {
    let session = Session::new(find_lesson("basics.placing").unwrap()).unwrap();
    let anchor = hex(&session.state().board, "bL").unwrap();
    let crosses = session.placement_crosses();
    assert_eq!(crosses.len(), 5);
    for position in crosses {
        assert!(position.is_neighbor(anchor));
        assert!(!session.state().board.occupied(position));
        assert!(!session.state().board.spawnable(Color::White, position));
    }
}

#[test]
fn movement_lesson_continues_every_queen_placement_and_black_reply() {
    let queen_rule = Session::new(find_lesson("basics.queen_rule").unwrap()).unwrap();
    let next = find_lesson("basics.move_pieces").unwrap();
    let queen: Piece = "wQ".parse().unwrap();
    let placements: Vec<_> = queen_rule.state().board.spawnable_positions(Color::White).collect();
    assert!(!placements.is_empty());
    for at in placements {
        let mut previous = queen_rule.clone();
        assert_eq!(previous.play(queen, at), Outcome::LessonDone);
        let mut continued = Session::with_starting_state(next, Some(previous.state().clone())).unwrap();
        assert!(continued.state().board == previous.state().board);
        assert_eq!(continued.state().turn_color, Color::White);
        let (piece, target) = board_moves(continued.state()).into_iter().next().unwrap();
        assert_eq!(continued.play(piece, target), Outcome::LessonDone);
    }
}

#[test]
fn black_warning_marks_follow_actual_mobility_after_every_white_move() {
    let initial = Session::new(find_lesson("basics.one_hive").unwrap()).unwrap();
    let mut freed = false;
    let mut pinned = false;
    for (piece, target) in board_moves(initial.state()) {
        let mut session = initial.clone();
        assert_eq!(session.play(piece, target), Outcome::StepDone);
        assert_eq!(session.learner(), Color::Black);
        let marks = session.marks();
        let moves = board_moves(session.state());
        for name in ["bL", "bQ"] {
            let black: Piece = name.parse().unwrap();
            let at = session.state().board.position_of_piece(black).unwrap();
            let movable = moves.iter().any(|(piece, _)| *piece == black);
            assert_eq!(marks.iter().any(|mark| mark.position == at), !movable);
            freed |= movable;
            pinned |= !movable;
        }
        let (black, at) = moves.into_iter().next().expect("Black can move");
        assert_eq!(session.play(black, at), Outcome::LessonDone);
    }
    assert!(freed && pinned);
}

#[test]
fn draw_swaps_the_white_pillbug_and_first_ant() {
    let session = Session::new(find_lesson("basics.draw").unwrap()).unwrap();
    let original = hive_lib::hop::parse("A+A+MG+B+g!Qb+g3+P6-b+l-m7+a+L+q,w").unwrap();
    let ant: Piece = "wA1".parse().unwrap();
    let pillbug: Piece = "wP".parse().unwrap();
    assert_eq!(session.state().board.position_of_piece(ant), original.board.position_of_piece(pillbug));
    assert_eq!(session.state().board.position_of_piece(pillbug), original.board.position_of_piece(ant));
}

#[test]
fn first_ant_exercise_accepts_every_attack_and_rejects_other_moves() {
    let initial = Session::new(find_lesson("bugs.ant_first_moves").unwrap()).unwrap();
    let ant: Piece = "wA1".parse().unwrap();
    let queen = hex(&initial.state().board, "bQ").unwrap();
    let mut attacks = 0;
    let mut rejected = 0;
    for (piece, target) in legal_moves(initial.state()) {
        let mut session = initial.clone();
        if piece == ant && target.is_neighbor(queen) {
            assert_eq!(session.play(piece, target), Outcome::LessonDone);
            attacks += 1;
        } else {
            assert_eq!(session.play(piece, target), Outcome::Wrong);
            rejected += 1;
        }
    }
    assert_eq!(attacks, 4);
    assert!(rejected > 0);
}

fn board_moves(state: &State) -> Vec<(Piece, Position)> {
    state.board.moves(state.turn_color).into_iter()
        .flat_map(|((piece, _), targets)| targets.into_iter().map(move |target| (piece, target)))
        .collect()
}

#[test]
fn placing_accepts_every_legal_piece_at_every_white_placement() {
    let initial = Session::new(find_lesson("basics.placing").unwrap()).unwrap();
    let mut pieces = HashSet::new();
    for (piece, target) in legal_moves(initial.state()) {
        let mut session = initial.clone();
        assert_eq!(session.play(piece, target), Outcome::LessonDone);
        assert!(session.state().board.position_of_piece("bQ".parse().unwrap()).is_some());
        assert!(session.prompt().unwrap().contains("your piece"));
        pieces.insert(piece.bug());
    }
    assert_eq!(pieces.len(), 7);
    assert!(pieces.contains(&hive_lib::Bug::Queen));
    assert!(!pieces.contains(&hive_lib::Bug::Ladybug));
}

#[test]
fn one_hive_black_win_easter_egg_requires_an_actual_win() {
    let mut session = Session::new(find_lesson("basics.one_hive").unwrap()).unwrap();
    let queen: Piece = "wQ".parse().unwrap();
    let target = hex(&session.state().board, "\\bL").unwrap();
    assert_eq!(session.play(queen, target), Outcome::StepDone);
    let black_target = hex(&session.state().board, "\\wQ").unwrap();
    for name in ["bA1", "bP"] {
        let mut win = session.clone();
        assert_eq!(win.play(name.parse().unwrap(), black_target), Outcome::LessonDone);
        assert_eq!(win.state().game_status,
            hive_lib::GameStatus::Finished(hive_lib::GameResult::Winner(Color::Black)));
        assert_eq!(win.prompt(), Some("Good job, Black won!"));
    }
    for (piece, target) in board_moves(session.state()) {
        let mut attempt = session.clone();
        assert_eq!(attempt.play(piece, target), Outcome::LessonDone);
        let won = attempt.state().game_status ==
            hive_lib::GameStatus::Finished(hive_lib::GameResult::Winner(Color::Black));
        assert_eq!(attempt.prompt() == Some("Good job, Black won!"), won);
    }
}

#[test]
fn ant_attack_accepts_exactly_three_pins_and_has_no_black_reply() {
    let initial = Session::new(find_lesson("bugs.ant_attack").unwrap()).unwrap();
    let ant: Piece = "wA1".parse().unwrap();
    let queen: Piece = "bQ".parse().unwrap();
    let queen_at = initial.state().board.position_of_piece(queen).unwrap();
    let expected: HashSet<_> = ["/bQ", "bQ\\", "bQ-"].into_iter()
        .map(|notation| hex(&initial.state().board, notation).unwrap()).collect();
    let mut accepted = HashSet::new();
    for (piece, target) in legal_moves(initial.state()) {
        let mut session = initial.clone();
        if piece == ant && expected.contains(&target) {
            assert_eq!(session.play(piece, target), Outcome::LessonDone);
            assert!(target.is_neighbor(queen_at));
            assert!(session.state().board.is_pinned(queen));
            assert!(!session.state().board.moves(Color::Black).iter()
                .any(|((mover, _), targets)| *mover == queen && !targets.is_empty()));
            assert_eq!(session.state().history.moves.iter().filter(|(piece, _)| piece != "pass").count(), 1);
            assert_eq!(session.red_marks().len(), 1);
            assert_eq!(session.red_marks()[0].position, queen_at);
            accepted.insert(target);
        } else {
            assert!(matches!(session.play(piece, target), Outcome::Wrong | Outcome::Refuted));
            assert!(session.needs_retry());
        }
    }
    assert_eq!(accepted, expected);
}

#[test]
fn ant_attack_reveals_exactly_three_green_pins_after_two_errors() {
    let mut session = Session::new(find_lesson("bugs.ant_attack").unwrap()).unwrap();
    assert!(session.highlights().is_empty());
    assert_eq!(session.fit_targets().into_iter().collect::<HashSet<_>>().len(), 3);
    let wrong = legal_moves(session.state()).into_iter().find(|(piece, _)| piece.to_string() != "wA1").unwrap();
    for count in 1..=2 {
        assert!(matches!(session.play(wrong.0, wrong.1), Outcome::Wrong | Outcome::Refuted));
        session.retry();
        assert_eq!(session.highlights().len(), if count == 2 { 3 } else { 0 });
        assert!(session.arrows().is_empty());
    }
    for at in session.highlights() {
        let mut attempt = session.clone();
        assert_eq!(attempt.play("wA1".parse().unwrap(), at), Outcome::LessonDone);
    }
}

#[test]
fn gate_preserves_geometry_swaps_ant_ids_and_only_allows_queen_and_first_ant() {
    let session = Session::new(find_lesson("bugs.ant").unwrap()).unwrap();
    let original = hive_lib::hop::parse("A+PA+b!+lL+Q+A+p+q5-a,w").unwrap();
    assert_eq!(session.state().board.position_of_piece("wA1".parse().unwrap()),
        original.board.position_of_piece("wA3".parse().unwrap()));
    for at in session.state().board.all_taken_positions() {
        assert!(original.board.occupied(at));
        let piece = session.state().board.top_piece(at).unwrap();
        assert_eq!(session.can_select(piece), ["wQ", "wA1"].contains(&piece.to_string().as_str()));
    }
}

#[test]
fn gate_ant_exploration_is_benign_and_reveals_queen_help_after_two_ant_moves() {
    let mut session = Session::new(find_lesson("bugs.ant").unwrap()).unwrap();
    let queen: Piece = "wQ".parse().unwrap();
    let ant: Piece = "wA1".parse().unwrap();
    let ant_start = session.state().board.position_of_piece(ant).unwrap();
    let from = session.state().board.position_of_piece(queen).unwrap();
    let open = hex(&session.state().board, "-wL").unwrap();
    let goal = hex(&session.state().board, "wL\\").unwrap();
    assert_eq!(session.red_arrows(), vec![(ant_start, goal)]);
    assert!(session.arrows().is_empty());
    let away = session.destinations(ant).into_iter().find(|to| *to != goal).unwrap();
    assert_eq!(session.play(ant, away), Outcome::Progress);
    assert!(!session.needs_retry());
    assert_eq!(session.step_index(), 0);
    assert!(session.arrows().is_empty());
    assert_eq!(session.state().turn_color, Color::White);
    assert!(session.destinations(ant).contains(&ant_start));
    assert_eq!(session.play(ant, ant_start), Outcome::Progress);
    assert!(!session.needs_retry());
    assert_eq!(session.arrows(), vec![(from, open)]);
    assert_eq!(session.play(queen, open), Outcome::Progress);
    assert!(session.red_arrows().is_empty());
    assert!(session.destinations(ant).contains(&goal));
    assert_eq!(session.play(ant, goal), Outcome::LessonDone);
    assert_eq!(session.completion_result(), Some("White wins!"));
}

#[test]
fn gate_accepts_every_legal_ant_exploration_without_a_retry() {
    let initial = Session::new(find_lesson("bugs.ant").unwrap()).unwrap();
    let ant: Piece = "wA1".parse().unwrap();
    assert!(!initial.destinations(ant).is_empty());
    for target in initial.destinations(ant) {
        let mut session = initial.clone();
        assert_eq!(session.play(ant, target), Outcome::Progress);
        assert!(!session.needs_retry());
        assert_eq!(session.state().board.position_of_piece(ant), Some(target));
        assert!(session.accepts_moves());
    }
}

#[test]
fn black_queen_escapes_after_every_failed_pin_when_a_legal_queen_move_exists() {
    let initial = Session::new(find_lesson("bugs.ant_attack").unwrap()).unwrap();
    let black: Piece = "bQ".parse().unwrap();
    let before = initial.state().board.position_of_piece(black).unwrap();
    let mut replies = 0;
    for (piece, target) in legal_moves(initial.state()) {
        let mut attempt = initial.clone();
        let outcome = attempt.play(piece, target);
        if outcome == Outcome::LessonDone { continue; }
        if outcome == Outcome::Refuted {
            let after = attempt.state().board.position_of_piece(black).unwrap();
            assert_ne!(before, after);
            assert_eq!(attempt.state().history.moves.last().unwrap().0, "bQ");
            assert!(attempt.needs_retry());
            attempt.retry();
            assert_eq!(attempt.state().board.position_of_piece(black), Some(before));
            replies += 1;
        } else {
            assert_eq!(outcome, Outcome::Wrong);
            let mut after_white = initial.state().clone();
            after_white.play_turn_from_position(piece, target).unwrap();
            assert!(!after_white.board.moves(Color::Black).keys().any(|(piece, _)| *piece == black));
        }
    }
    assert!(replies > 0);
}

#[test]
fn spiders_are_lessons_fourteen_to_seventeen_and_all_are_ready() {
    let ids: Vec<_> = lessons().skip(14).take(4).map(|lesson| lesson.id).collect();
    assert_eq!(ids, ["bugs.spider", "bugs.spider_dead_end", "bugs.spider_door", "bugs.spider_power"]);
    let group = super::group_of("bugs.spider").unwrap();
    assert_eq!(group.title, "The Spider");
    for id in &ids { assert_eq!(super::group_of(id).unwrap().id, group.id); }
    assert!(!find_lesson("bugs.spider_dead_end").unwrap().is_pending());
    assert!(!find_lesson("bugs.spider_door").unwrap().is_pending());
}

#[test]
fn spider_lesson_accepts_both_three_step_destinations_and_hints_after_two_errors() {
    let initial = Session::new(find_lesson("bugs.spider").unwrap()).unwrap();
    let spider: Piece = "wS1".parse().unwrap();
    let expected: HashSet<_> = ["\\wA1", "\\bA2"].into_iter()
        .map(|notation| hex(&initial.state().board, notation).unwrap()).collect();
    let mut accepted = HashSet::new();
    for (piece, target) in legal_moves(initial.state()) {
        let mut attempt = initial.clone();
        if piece == spider {
            assert_eq!(attempt.play(piece, target), Outcome::LessonDone);
            assert!(!attempt.current_step().unwrap().completion_tip.is_empty());
            accepted.insert(target);
        } else {
            assert_eq!(attempt.play(piece, target), Outcome::Wrong);
        }
    }
    assert_eq!(accepted, expected);
    assert_eq!(initial.marks().len(), 1);
    assert!(initial.highlights().is_empty());
    let wrong = legal_moves(initial.state()).into_iter().find(|(piece, _)| *piece != spider).unwrap();
    let mut session = initial.clone();
    for count in 1..=2 {
        assert_eq!(session.play(wrong.0, wrong.1), Outcome::Wrong);
        session.retry();
        assert_eq!(session.highlights().len(), if count == 2 { 2 } else { 0 });
    }
    assert_eq!(session.highlights().into_iter().collect::<HashSet<_>>(), expected);
}

#[test]
fn spider_power_uses_both_spiders_and_only_normal_three_step_crawls() {
    let mut session = Session::new(find_lesson("bugs.spider_power").unwrap()).unwrap();
    let initial = session.state().board.clone();
    for at in initial.all_taken_positions() {
        let piece = initial.top_piece(at).unwrap();
        assert_eq!(session.can_select(piece), ["wS1", "wS2"].contains(&piece.to_string().as_str()));
    }
    for (index, (piece, notation)) in [
        ("wS2", "\\bA3"), ("wS2", "wL-"), ("wS1", "/bA2"),
        ("wS1", "bQ\\"), ("wS2", "\\bG1"),
    ].into_iter().enumerate() {
        let piece: Piece = piece.parse().unwrap();
        let target = hex(&session.state().board, notation).unwrap();
        let from = session.state().board.position_of_piece(piece).unwrap();
        assert!(hive_lib::Bug::normal_moves(from, &session.state().board).contains(&target));
        assert_eq!(session.play(piece, target),
            if index == 4 { Outcome::LessonDone } else { Outcome::Progress });
        assert!(!session.needs_retry());
    }
    assert!(Condition::Surrounded("bQ").holds(session.state()));
    assert!(Condition::Touching("wS1", "bQ").holds(session.state()));
    assert!(Condition::Touching("wS2", "bQ").holds(session.state()));
    assert_eq!(session.completion_result(), Some("White wins!"));
    for at in initial.all_taken_positions() {
        let piece = initial.top_piece(at).unwrap();
        if piece.bug() != hive_lib::Bug::Spider {
            assert_eq!(session.state().board.position_of_piece(piece), Some(at));
        }
    }
}

#[test]
fn blank_tiles_cannot_lend_their_pillbug_ability_to_a_spider() {
    let mut session = Session::new(find_lesson("bugs.spider_power").unwrap()).unwrap();
    let mut rejected = 0;
    for (piece, notation) in [
        ("wS2", "\\bA3"), ("wS2", "wL-"), ("wS1", "/bA2"),
        ("wS1", "bQ\\"), ("wS2", "\\bG1"),
    ] {
        for name in ["wS1", "wS2"] {
            let spider: Piece = name.parse().unwrap();
            let ordinary = session.destinations(spider);
            for (piece, to) in board_moves(session.state()) {
                if piece == spider && !ordinary.contains(&to) {
                    let mut attempt = session.clone();
                    assert_eq!(attempt.play(piece, to), Outcome::Illegal);
                    assert!(attempt.state().board == session.state().board);
                    rejected += 1;
                }
            }
        }
        let target = hex(&session.state().board, notation).unwrap();
        assert!(matches!(session.play(piece.parse().unwrap(), target),
            Outcome::Progress | Outcome::LessonDone));
    }
    assert!(rejected > 0, "the hidden Pillbug would otherwise lend an ability");
}

#[test]
fn repeated_ant_exploration_does_not_end_the_exercise_in_a_draw() {
    let mut session = Session::new(find_lesson("bugs.ant").unwrap()).unwrap();
    let ant: Piece = "wA1".parse().unwrap();
    let start = session.state().board.position_of_piece(ant).unwrap();
    let away = session.destinations(ant)[0];
    for _ in 0..6 {
        assert_eq!(session.play(ant, away), Outcome::Progress);
        assert_eq!(session.play(ant, start), Outcome::Progress);
        assert!(session.accepts_moves());
        assert!(!session.needs_retry());
    }
}

#[test]
fn backtracking_lesson_finishes_without_a_move_or_continue_button() {
    let session = Session::new(find_lesson("bugs.spider_dead_end").unwrap()).unwrap();
    let spider: Piece = "wS1".parse().unwrap();
    let from = session.state().board.position_of_piece(spider).unwrap();
    assert!(!session.state().board.is_pinned(spider));
    assert!(hive_lib::Bug::normal_moves(from, &session.state().board).is_empty());
    assert_eq!(session.marks().len(), 1);
    assert_eq!(session.marks()[0].position, from);
    assert!(session.is_finished());
    assert!(!session.awaits_continue());
    assert!(!session.accepts_moves());
    for at in session.state().board.all_taken_positions() {
        assert!(!session.can_select(session.state().board.top_piece(at).unwrap()));
    }
}

fn expected_arrow_count(session: &Session, step: &Step) -> usize {
    step.arrows.len() + step.legal_arrows
        .and_then(|spec| spec.parse::<Piece>().ok())
        .map_or(0, |piece| session.destinations(piece).len())
}

#[test]
fn corrected_door_position_has_a_two_move_route_and_arrows_follow_the_spider() {
    let mut session = Session::new(find_lesson("bugs.spider_door").unwrap()).unwrap();
    let spider: Piece = "wS1".parse().unwrap();
    let from = session.state().board.position_of_piece(spider).unwrap();
    assert_eq!(from, Position::new(17, 16));
    let goal = hex(&session.state().board, "bA1\\").unwrap();
    let queen = hex(&session.state().board, "bQ").unwrap();
    let ant = hex(&session.state().board, "bA1").unwrap();
    assert!(goal.is_neighbor(queen) && goal.is_neighbor(ant));
    assert!(!session.state().board.occupied(goal));
    assert!(!session.destinations(spider).contains(&goal));
    assert_eq!(session.highlights(), vec![goal]);
    assert!(session.current_step().unwrap().small_arrows);
    assert_eq!(session.arrows().len(), 4);
    for at in session.state().board.all_taken_positions() {
        let piece = session.state().board.top_piece(at).unwrap();
        assert_eq!(session.can_select(piece), piece == spider);
    }
    let ordinary: HashSet<_> = session.destinations(spider).into_iter().collect();
    assert!(session.arrows().iter().all(|(start, to)| *start == from && ordinary.contains(to)));
    for to in ordinary {
        let mut attempt = session.clone();
        assert_eq!(attempt.play(spider, to), Outcome::Progress);
        assert!(!attempt.needs_retry());
        assert!(attempt.arrows().iter().all(|(start, target)|
            *start == to && attempt.destinations(spider).contains(target)));
    }
    let via = hex(&session.state().board, "wA2\\").unwrap();
    assert_eq!(session.play(spider, via), Outcome::Progress);
    assert!(!session.is_finished());
    assert_eq!(session.play(spider, goal), Outcome::LessonDone);
    assert!(session.is_finished());
}

#[test]
fn beetle_group_precedes_grasshopper_and_spider_power_is_instruction_only() {
    let ids: Vec<_> = lessons().skip(18).take(4).map(|lesson| lesson.id).collect();
    assert_eq!(ids, ["bugs.beetle", "bugs.beetle_spawn", "bugs.beetle_gate", "bugs.grasshopper"]);
    let group = super::group_of("bugs.beetle").unwrap();
    assert_eq!(group.title, "The Beetle");
    assert_eq!(group.lessons, &["bugs.beetle", "bugs.beetle_spawn", "bugs.beetle_gate"]);
    assert_eq!(super::group_of("bugs.beetle_gate").unwrap().id, group.id);
    let power = find_lesson("bugs.spider_power").unwrap();
    assert!(power.intro.is_empty());
    assert!(!power.steps[0].prompt.contains("Blank tiles and Queens stay fixed"));
}

#[test]
fn climbing_finishes_only_after_black_covers_the_white_beetle() {
    let mut session = Session::new(find_lesson("bugs.beetle").unwrap()).unwrap();
    let beetle: Piece = "wB1".parse().unwrap();
    let start = session.state().board.position_of_piece(beetle).unwrap();
    let expected: HashSet<_> = hive_lib::Bug::normal_moves(start, &session.state().board).into_iter().collect();
    assert_eq!(session.destinations(beetle).into_iter().collect::<HashSet<_>>(), expected);
    assert_eq!(session.arrows().iter().map(|(_, to)| *to).collect::<HashSet<_>>(), expected);
    let engine_targets = session.state().board.moves(Color::White)
        .get(&(beetle, start)).cloned().unwrap();
    let assisted: Vec<_> = engine_targets.into_iter()
        .filter(|target| !expected.contains(target)).collect();
    assert!(!assisted.is_empty());
    for target in assisted {
        let before = session.state().board.clone();
        assert_eq!(session.play(beetle, target), Outcome::Illegal);
        assert!(session.state().board == before);
    }
    let gate = hex(&session.state().board, "-bP").unwrap();
    assert!(!session.state().board.occupied(gate));
    assert!(gate.is_neighbor(hex(&session.state().board, "wQ").unwrap()));
    assert!(gate.is_neighbor(hex(&session.state().board, "bQ").unwrap()));
    assert_eq!(session.red_arrows(), vec![(start, gate)]);
    assert!(!session.destinations(beetle).contains(&gate));
    assert_eq!(session.play(beetle, gate), Outcome::Illegal);
    let ladybug = hex(&session.state().board, "bL").unwrap();
    assert_eq!(session.play(beetle, ladybug), Outcome::StepDone);
    assert!(Condition::OnTop("wB1", "bL").holds(session.state()));
    let black_beetle = hex(&session.state().board, "bB1").unwrap();
    assert_eq!(black_beetle, hex(&session.state().board, "bQ\\").unwrap());
    assert!(black_beetle.is_neighbor(hex(&session.state().board, "bM").unwrap()));
    let incoming = session.black_reply_arrow().unwrap();
    assert_eq!(incoming.1, black_beetle);
    assert!(incoming.0.is_neighbor(incoming.1));
    assert!(!session.state().board.occupied(incoming.0));

    assert!(!session.is_finished());
    assert_eq!(session.state().turn_color, Color::White);
    let queen = hex(&session.state().board, "bQ").unwrap();
    assert_eq!(session.play(beetle, queen), Outcome::LessonDone);
    assert_eq!(session.state().board.level(queen), 3);
    assert_eq!(session.state().board.position_of_piece(beetle), Some(queen));
    assert_eq!(session.state().board.top_piece(queen), Some("bB1".parse().unwrap()));
    assert!(Condition::OnTop("bB1", "wB1").holds(session.state()));
    assert_eq!(session.black_reply_arrow(), Some((black_beetle, queen)));
    session.retry();
    assert!(session.black_reply_arrow().is_none());
    let ladybug = hex(&session.state().board, "bL").unwrap();
    assert_eq!(session.play(beetle, ladybug), Outcome::StepDone);
    assert_eq!(session.play(beetle, queen), Outcome::LessonDone);
    assert!(Condition::Stuck("wB1").holds(session.state()));
    assert!(Condition::Stuck("bQ").holds(session.state()));
    assert!(session.is_finished());
}

#[test]
fn climbing_hints_replace_all_other_arrows_after_two_errors_on_each_step() {
    let mut session = Session::new(find_lesson("bugs.beetle").unwrap()).unwrap();
    let beetle: Piece = "wB1".parse().unwrap();
    for stage in 0..2 {
        for attempt in 0..2 {
            if stage == 1 {
                let ladybug = hex(&session.state().board, "bL").unwrap();
                assert_eq!(session.play(beetle, ladybug), Outcome::StepDone);
            }
            let goal = hex(&session.state().board, if stage == 0 { "bL" } else { "bQ" }).unwrap();
            let wrong = session.destinations(beetle).into_iter().find(|to| *to != goal).unwrap();
            assert_eq!(session.play(beetle, wrong), Outcome::Wrong);
            assert!(session.arrows().is_empty());
            session.retry();
            if stage == 0 {
                assert_eq!(session.arrows().len(), if attempt == 1 { 1 } else { 5 });
                assert_eq!(session.red_arrows().len(), if attempt == 1 { 0 } else { 1 });
            }
        }
        if stage == 0 {
            let from = session.state().board.position_of_piece(beetle).unwrap();
            assert_eq!(session.arrows(), vec![(from, hex(&session.state().board, "bL").unwrap())]);
        } else {
            let ladybug = hex(&session.state().board, "bL").unwrap();
            assert_eq!(session.play(beetle, ladybug), Outcome::StepDone);
            assert_eq!(session.arrows(), vec![(ladybug, hex(&session.state().board, "bQ").unwrap())]);
            assert!(session.red_arrows().is_empty());
        }
    }
}

#[test]
fn spawn_points_only_accept_the_two_new_hexes_for_any_reserve_piece() {
    let mut session = Session::new(find_lesson("bugs.beetle_spawn").unwrap()).unwrap();
    let before: HashSet<_> = session.state().board.spawnable_positions(Color::White).collect();
    let target = hex(&session.state().board, "bP").unwrap();
    assert_eq!(session.play("wB1".parse().unwrap(), target), Outcome::StepDone);
    assert_eq!(session.state().board.top_piece(target), Some("wB1".parse().unwrap()));
    assert!(Condition::Placed("bA2").holds(session.state()));
    assert_eq!(session.state().turn_color, Color::White);
    assert!(!session.is_finished());
    let after: HashSet<_> = session.state().board.spawnable_positions(Color::White).collect();
    let expected: HashSet<_> = ["\\wG1", "wB1\\"].into_iter()
        .map(|name| hex(&session.state().board, name).unwrap()).collect();
    assert_eq!(session.highlights().into_iter().collect::<HashSet<_>>(), expected);
    let hopper = hex(&session.state().board, "wG1").unwrap();
    assert!(expected.iter().all(|at| after.contains(at) && !before.contains(at)
        && at.is_neighbor(target) && at.is_neighbor(hopper)));

    let mut accepted = 0;
    for (piece, at) in legal_moves(session.state()) {
        if session.state().board.position_of_piece(piece).is_some() { continue; }
        let mut attempt = session.clone();
        let correct = expected.contains(&at);
        assert_eq!(attempt.play(piece, at), if correct { Outcome::LessonDone } else { Outcome::Wrong });
        assert_eq!(attempt.is_finished(), correct);
        if correct { accepted += 1; }
    }
    assert!(accepted > 1);
}

#[test]
fn beetle_gates_has_only_beetle_graphics_and_both_routes_still_win() {
    let lesson = find_lesson("bugs.beetle_gate").unwrap();
    assert_eq!(lesson.blank_visible().unwrap(), &["wB1", "wB2", "bB1", "bB2"]);
    assert!(lesson.outro.is_empty());
    for obstacle in ["bB1", "bB2"] {
        let mut session = Session::new(lesson).unwrap();
        let beetle: Piece = "wB1".parse().unwrap();
        let from = session.state().board.position_of_piece(beetle).unwrap();
        let goal = hex(&session.state().board, "-bB1").unwrap();
        assert_eq!(session.red_arrows(), vec![(from, goal)]);
        assert!(!session.destinations(beetle).contains(&goal));
        assert!(session.marks().iter().all(|mark| mark.number.is_none()));
        assert!(!session.can_select("wQ".parse().unwrap()));
        let first = hex(&session.state().board, obstacle).unwrap();
        assert_eq!(session.play(beetle, first), Outcome::StepDone);
        assert!(session.black_reply_arrow().is_none());
        assert_eq!(session.play(beetle, goal), Outcome::LessonDone);
        assert!(Condition::Surrounded("bQ").holds(session.state()));
    }
}

#[test]
fn jump_lesson_swaps_the_mosquito_and_shows_two_legal_and_two_illegal_options() {
    let mut session = Session::new(find_lesson("bugs.grasshopper").unwrap()).unwrap();
    assert_eq!(session.lesson().title, "Jump!");
    let board = &session.state().board;
    assert!(board.position_of_piece("bP".parse().unwrap()).is_none());
    assert!(board.position_of_piece("bM".parse().unwrap()).is_some());
    let hopper: Piece = "wG1".parse().unwrap();
    let from = board.position_of_piece(hopper).unwrap();
    let legal: HashSet<_> = session.destinations(hopper).into_iter().collect();
    assert_eq!(legal.len(), 2);
    assert_eq!(session.arrows().iter().map(|(_, to)| *to).collect::<HashSet<_>>(), legal);
    let ant = hex(board, "bA3").unwrap();
    assert_eq!(session.red_arrows().len(), 2);
    for (start, to) in session.red_arrows() {
        assert_eq!(start, from);
        assert!(to.is_neighbor(ant));
        assert!(!board.occupied(to));
        assert!(!legal.contains(&to));
    }
    let goal = hex(board, "-bQ").unwrap();
    assert_eq!(session.play(hopper, goal), Outcome::StepDone);
    let reply = session.black_reply_arrow().unwrap();
    assert_eq!(reply.1, hex(&session.state().board, "bG1").unwrap());
    let exit = hex(&session.state().board, "-bA2").unwrap();
    assert_eq!(session.play(hopper, exit), Outcome::LessonDone);
    assert_eq!(session.black_reply_arrow(), Some(reply));
}

#[test]
fn grasshopper_group_occupies_lessons_twenty_one_to_twenty_three() {
    let ids: Vec<_> = lessons().skip(21).take(3).map(|lesson| lesson.id).collect();
    assert_eq!(ids, ["bugs.grasshopper", "bugs.grasshopper_place", "bugs.grasshopper_paths"]);
    let group = super::group_of("bugs.grasshopper").unwrap();
    assert_eq!(group.title, "The Grasshopper");
    assert_eq!(group.lessons, ids.as_slice());
}

#[test]
fn grasshopper_placement_only_accepts_the_winning_hex_and_black_refutes_wrong_placements() {
    let initial = Session::new(find_lesson("bugs.grasshopper_place").unwrap()).unwrap();
    let hopper: Piece = "wG3".parse().unwrap();
    let goal = hex(&initial.state().board, "/wA3").unwrap();
    assert!(goal.is_neighbor(hex(&initial.state().board, "wG1").unwrap()));
    assert!(goal.is_neighbor(hex(&initial.state().board, "wA3").unwrap()));
    let trap = hex(&initial.state().board, "wS1/").unwrap();
    let mut rejected = 0;
    for target in initial.state().board.spawnable_positions(Color::White) {
        let mut session = initial.clone();
        if target == goal {
            assert_eq!(session.play(hopper, target), Outcome::StepDone);
            assert!(!session.is_finished());
            assert!(session.black_reply_arrow().is_some());
            let win = hex(&session.state().board, "\\bQ").unwrap();
            assert_eq!(session.play(hopper, win), Outcome::LessonDone);
            assert!(Condition::Surrounded("bQ").holds(session.state()));
            assert_eq!(session.completion_result(), Some("White wins!"));
        } else {
            assert_eq!(session.play(hopper, target), Outcome::Refuted);
            assert!(session.needs_retry());
            assert!(session.black_reply_arrow().is_some());
            if target == trap {
                assert!(session.state().board.is_pinned(hopper));
                assert_eq!(session.black_reply_arrow().unwrap().1, hex(&session.state().board, "bS1").unwrap());
            } else {
                assert_eq!(session.state().turn_color, Color::White);
                assert!(session.state().board.last_moved.unwrap().0.is_color(Color::Black));
            }
            rejected += 1;
        }
    }
    assert!(rejected > 1);
    let mut session = initial;
    for attempt in 0..2 {
        assert_eq!(session.play(hopper, trap), Outcome::Refuted);
        session.retry();
        assert_eq!(session.highlights(), if attempt == 1 { vec![goal] } else { vec![] });
    }
}

#[test]
fn jumping_paths_only_moves_two_grasshoppers_and_requires_both_final_holes() {
    let mut session = Session::new(find_lesson("bugs.grasshopper_paths").unwrap()).unwrap();
    let initial = session.state().board.clone();
    let holes: HashSet<_> = hex(&initial, "bQ").unwrap().positions_around()
        .filter(|at| !initial.occupied(*at)).collect();
    assert_eq!(holes.len(), 2);
    assert_eq!(session.highlights().into_iter().collect::<HashSet<_>>(), holes);
    for at in initial.all_taken_positions() {
        let piece = initial.top_piece(at).unwrap();
        assert_eq!(session.can_select(piece), ["wG2", "wG3"].contains(&piece.to_string().as_str()));
    }
    for (i, (name, to)) in [
        ("wG2", "bS1\\"), ("wG2", "/bA1"), ("wG3", "bS1\\"),
        ("wG3", "\\bL"), ("wG2", "\\bA2"), ("wG3", "bB2\\"),
    ].into_iter().enumerate() {
        let piece: Piece = name.parse().unwrap();
        let target = hex(&session.state().board, to).unwrap();
        let from = session.state().board.position_of_piece(piece).unwrap();
        assert!(hive_lib::Bug::normal_moves(from, &session.state().board).contains(&target));
        assert_eq!(session.play(piece, target), if i == 5 { Outcome::LessonDone } else { Outcome::Progress });
        assert!(session.black_reply_arrow().is_none());
        assert!(!session.needs_retry());
    }
    assert!(Condition::Surrounded("bQ").holds(session.state()));
    assert!(Condition::Touching("wG2", "bQ").holds(session.state()));
    assert!(Condition::Touching("wG3", "bQ").holds(session.state()));
    assert_eq!(session.completion_result(), Some("White wins!"));
    for at in initial.all_taken_positions() {
        let piece = initial.top_piece(at).unwrap();
        if !["wG2", "wG3"].contains(&piece.to_string().as_str()) {
            assert_eq!(session.state().board.position_of_piece(piece), Some(at));
        }
    }
}

#[test]
fn initial_framing_includes_known_black_placements_and_their_incoming_arrows() {
    for (id, piece, to) in [
        ("bugs.beetle", "wB1", "bL"),
        ("bugs.beetle_spawn", "wB1", "bP"),
        ("bugs.queen", "wQ", "bA2\\"),
    ] {
        let mut session = Session::new(find_lesson(id).unwrap()).unwrap();
        let before = session.state().board.clone();
        let framing = session.fit_targets();
        assert!(session.state().board == before);
        let to = hex(&session.state().board, to).unwrap();
        assert_eq!(session.play(piece.parse().unwrap(), to), Outcome::StepDone);
        let (from, to) = session.black_reply_arrow().unwrap();
        assert!(framing.contains(&from), "{id}: incoming arrow begins outside initial framing");
        assert!(framing.contains(&to), "{id}: Black's new piece is outside initial framing");
    }
}


#[test]
fn piece_sections_put_ladybug_then_pillbug_then_mosquito_last() {
    let ids: Vec<_> = lessons().skip(24).take(8).map(|lesson| lesson.id).collect();
    assert_eq!(ids, [
        "bugs.ladybug", "bugs.ladybug_gate", "bugs.pillbug",
        "bugs.pillbug_save", "bugs.pillbug_enemy",
        "bugs.pillbug_gate", "bugs.pillbug_deactivate", "bugs.mosquito",
    ]);
    assert_eq!(super::group_of("bugs.ladybug_gate").unwrap().title, "The Ladybug");
    assert_eq!(super::group_of("bugs.pillbug").unwrap().lessons.len(), 5);
    assert!(find_lesson("bugs.pillbug_walk").is_none());
    for id in &ids[2..7] {
        assert!(!find_lesson(id).unwrap().is_pending());
    }
}

#[test]
fn grasshopper_random_reply_keeps_the_next_winning_jump_available() {
    let initial = Session::new(find_lesson("bugs.grasshopper_place").unwrap()).unwrap();
    assert_eq!(initial.red_marks()[0].position, hex(&initial.state().board, "bQ").unwrap());
    let hopper: Piece = "wG3".parse().unwrap();
    let start = hex(&initial.state().board, "/wA3").unwrap();
    for _ in 0..32 {
        let mut session = initial.clone();
        assert_eq!(session.play(hopper, start), Outcome::StepDone);
        assert!(session.black_reply_arrow().is_some());
        assert_eq!(session.red_marks()[0].position, hex(&session.state().board, "bQ").unwrap());
        let target = hex(&session.state().board, "\\bQ").unwrap();
        assert_eq!(session.play(hopper, target), Outcome::LessonDone);
        assert_eq!(session.completion_result(), Some("White wins!"));
    }
}

#[test]
fn jumping_path_hints_appear_after_ten_moves_and_retry_after_sixteen() {
    let mut session = Session::new(find_lesson("bugs.grasshopper_paths").unwrap()).unwrap();
    let hopper: Piece = "wG2".parse().unwrap();
    let a = session.state().board.position_of_piece(hopper).unwrap();
    let b = hex(&session.state().board, "bS1\\").unwrap();
    for i in 1..=16 {
        let target = if i % 2 == 1 { b } else { a };
        assert_eq!(session.play(hopper, target), if i == 16 { Outcome::Wrong } else { Outcome::Progress });
        if i < 10 { assert!(session.arrows().is_empty()); }
        if i == 10 {
            let arrows = session.arrows();
            assert!(!arrows.is_empty(), "route hint search found no solution");
            let mut solve = session.clone();
            for (from, to) in arrows {
                let piece = solve.state().board.top_piece(from).unwrap();
                assert!(["wG2", "wG3"].contains(&piece.to_string().as_str()));
                assert!(solve.destinations(piece).contains(&to));
                let result = solve.play(piece, to);
                assert!(matches!(result, Outcome::Progress | Outcome::LessonDone));
            }
            assert!(solve.is_finished());
        }
    }
    assert!(session.needs_retry());
    assert!(!session.is_finished());
    assert_eq!(session.state().board.position_of_piece(hopper), Some(a));
    session.retry();
    assert!(!session.needs_retry());
    assert!(session.arrows().is_empty());
}

#[test]
fn ladybug_gate_has_exactly_two_move_pinning_routes_and_hides_other_pieces() {
    let initial = Session::new(find_lesson("bugs.ladybug_gate").unwrap()).unwrap();
    let ladybug: Piece = "wL".parse().unwrap();
    let from = initial.state().board.position_of_piece(ladybug).unwrap();
    let blocked = hex(&initial.state().board, "\\bS2").unwrap();
    assert_eq!(initial.forbidden(), vec![blocked]);
    assert!(!initial.destinations(ladybug).contains(&blocked));
    assert_eq!(initial.red_arrows(), vec![(from, hex(&initial.state().board, "bG1").unwrap())]);
    assert!(!initial.destinations(ladybug).contains(&hex(&initial.state().board, "bG1").unwrap()));
    for at in initial.state().board.all_taken_positions() {
        let piece = initial.state().board.top_piece(at).unwrap();
        assert_eq!(initial.can_select(piece), piece == ladybug);
    }
    let mut good = 0;
    for to in initial.destinations(ladybug) {
        let mut session = initial.clone();
        let result = session.play(ladybug, to);
        assert!(!session.is_finished());
        if result == Outcome::StepDone {
            good += 1;
            assert!(!Condition::Pinned("bQ").holds(session.state()));
            let finish = hex(&session.state().board, "bQ-").unwrap();
            assert_eq!(session.play(ladybug, finish), Outcome::LessonDone);
            assert!(Condition::Pinned("bQ").holds(session.state()));
            assert!(Condition::Stuck("bQ").holds(session.state()));
        } else {
            assert_eq!(result, Outcome::Wrong);
            assert!(session.needs_retry());
        }
    }
    assert_eq!(good, 2);
    let visible = initial.lesson().blank_visible().unwrap();
    assert!(visible.contains(&"bQ") && visible.contains(&"wL"));
    assert!(!visible.contains(&"wQ"));
}

#[test]
fn pillbug_exercise_requires_the_lift_and_shows_all_three_drop_arrows() {
    let initial = Session::new(find_lesson("bugs.pillbug").unwrap()).unwrap();
    let beetle: Piece = "wB1".parse().unwrap();
    let from = initial.state().board.position_of_piece(beetle).unwrap();
    let ability = hex(&initial.state().board, "wP").unwrap();
    let target = hex(&initial.state().board, "wP\\").unwrap();
    assert!(!hive_lib::Bug::normal_moves(from, &initial.state().board).contains(&target));
    assert!(initial.destinations(beetle).contains(&target));
    assert_eq!(initial.arrows().len(), 4);
    assert!(initial.arrows().contains(&(from, ability)));
    for (_, to) in initial.arrows().into_iter().filter(|(start,_)| *start == ability) {
        assert!(hive_lib::Bug::can_throw(ability, from, to, &initial.state().board));
    }
    assert_eq!(initial.red_arrows(), vec![(from, target)]);
    let mut session = initial.clone();
    assert_eq!(session.play(beetle, target), Outcome::LessonDone);
    assert!(Condition::Touching("wB1", "bQ").holds(session.state()));
    for to in initial.destinations(beetle).into_iter().filter(|to| *to != target) {
        let mut wrong = initial.clone();
        assert_eq!(wrong.play(beetle, to), Outcome::Wrong);
        assert!(wrong.needs_retry());
        assert_eq!(wrong.state().board.position_of_piece(beetle), Some(to));
    }
}


#[test]
fn ladybug_example_has_two_occupied_steps_then_the_empty_pocket() {
    let session = Session::new(find_lesson("bugs.ladybug").unwrap()).unwrap();
    let arrows = session.arrows();
    assert_eq!(arrows.len(), 3);
    assert_eq!(session.marks()[0].position, arrows[0].0);
    assert_eq!(arrows[0].1, arrows[1].0);
    assert_eq!(arrows[1].1, arrows[2].0);
    assert!(arrows.iter().all(|(from, to)| from.is_neighbor(*to)));
    assert!(session.state().board.occupied(arrows[0].1));
    assert!(session.state().board.occupied(arrows[1].1));
    assert!(!session.state().board.occupied(arrows[2].1));
    assert!(session.destinations("wL".parse().unwrap()).contains(&arrows[2].1));
}


#[test]
fn pillbug_rescue_accepts_every_drop_resets_the_position_and_requires_both_parts() {
    let initial = Session::new(find_lesson("bugs.pillbug_save").unwrap()).unwrap();
    let queen: Piece = "wQ".parse().unwrap();
    let pillbug: Piece = "wP".parse().unwrap();
    let board = &initial.state().board;
    let queen_at = board.position_of_piece(queen).unwrap();
    assert_eq!(queen_at.positions_around().filter(|at| board.occupied(*at)).count(), 5);
    assert!(hive_lib::Bug::normal_moves(queen_at, board).is_empty());
    let drops = initial.destinations(queen);
    assert_eq!(drops.len(), 3);
    assert!(drops.iter().all(|at| initial.fit_targets().contains(at)));
    for target in drops {
        let mut rescued = initial.clone();
        assert_eq!(rescued.play(queen, target), Outcome::StepDone);
        assert!(!rescued.is_finished());
        assert_eq!(rescued.step_index(), 1);
        assert_eq!(rescued.state().board.position_of_piece(queen), Some(target));
        assert!(rescued.awaits_continue());
        assert_eq!(rescued.continue_label(), "Next");
        assert!(!rescued.accepts_moves());
        assert_eq!(rescued.advance(), Outcome::StepDone);
        assert!(rescued.state().board == initial.state().board);
        assert!(rescued.prompt().unwrap().contains("can also move by itself"));
        assert!(rescued.black_reply_arrow().is_none());
        let moves = rescued.destinations(pillbug);
        assert_eq!(moves.len(), 2);
        for to in moves {
            let mut second = rescued.clone();
            assert!(!to.is_neighbor(queen_at));
            assert_eq!(second.play(pillbug, to), Outcome::LessonDone);
            assert_eq!(second.state().board.position_of_piece(queen), Some(queen_at));
            assert_eq!(queen_at.positions_around().filter(|at| second.state().board.occupied(*at)).count(), 4);
        }
    }
}

#[test]
fn pillbug_lift_hint_goes_over_the_pillbug_after_two_errors() {
    let mut session = Session::new(find_lesson("bugs.pillbug_save").unwrap()).unwrap();
    let pillbug: Piece = "wP".parse().unwrap();
    let queen: Piece = "wQ".parse().unwrap();
    assert!(session.arrows().is_empty());
    for i in 0..2 {
        let wrong = session.destinations(pillbug)[0];
        assert_eq!(session.play(pillbug, wrong), Outcome::Wrong);
        assert!(session.needs_retry());
        session.retry();
        if i == 0 { assert!(session.arrows().is_empty()); }
    }
    let arrows = session.arrows();
    assert_eq!(arrows.len(), 2);
    let board = &session.state().board;
    assert_eq!(arrows[0].0, board.position_of_piece(queen).unwrap());
    assert_eq!(arrows[0].1, board.position_of_piece(pillbug).unwrap());
    assert_eq!(arrows[1].0, arrows[0].1);
    assert!(hive_lib::Bug::can_throw(arrows[0].1, arrows[0].0, arrows[1].1, board));
}

#[test]
fn enemy_ant_lesson_preserves_geometry_and_cannot_lift_the_last_moved_ant() {
    let initial = Session::new(find_lesson("bugs.pillbug_enemy").unwrap()).unwrap();
    let recent: Piece = "bA2".parse().unwrap();
    let movable: Piece = "bA1".parse().unwrap();
    assert_eq!(initial.state().board.last_moved.unwrap().0, recent);
    assert_eq!(initial.state().board.stunned, Some(recent));
    assert!(initial.destinations(recent).is_empty());
    assert!(initial.red_marks().is_empty());
    assert_eq!(initial.state().board.last_move.1, initial.state().board.position_of_piece(recent));
    assert!(initial.state().board.last_move.0.is_some());
    let drops = initial.destinations(movable);
    assert_eq!(drops.len(), 3);
    for to in drops {
        let mut session = initial.clone();
        assert_eq!(session.play(movable, to), Outcome::LessonDone);
        assert_eq!(session.state().board.position_of_piece(movable), Some(to));
    }
}

#[test]
fn pillbug_gate_rejects_both_covered_targets_and_refutes_every_wrong_beetle_move() {
    let initial = Session::new(find_lesson("bugs.pillbug_gate").unwrap()).unwrap();
    let good: Piece = "wB1".parse().unwrap();
    let wrong: Piece = "wB2".parse().unwrap();
    let queen: Piece = "wQ".parse().unwrap();
    let queen_at = initial.state().board.position_of_piece(queen).unwrap();
    let pill_at = hex(&initial.state().board, "wP").unwrap();
    assert!(initial.destinations(queen).is_empty());
    assert!(hive_lib::Bug::available_abilities(pill_at, &initial.state().board).is_empty());
    assert_eq!(initial.marks().len(), 2);
    for target in initial.destinations(wrong) {
        let mut session = initial.clone();
        assert_eq!(session.play(wrong, target), Outcome::Refuted);
        assert!(session.needs_retry());
        assert_eq!(session.state().board.top_piece(queen_at), Some("bB1".parse().unwrap()));
        assert!(session.black_reply_arrow().is_some());
    }
    let mut successes = 0;
    for target in initial.destinations(good) {
        let mut session = initial.clone();
        if [queen_at, pill_at].contains(&target) {
            assert_eq!(session.play(good, target), Outcome::Wrong);
        } else {
            assert_eq!(session.play(good, target), Outcome::StepDone);
            successes += 1;
            assert!(!session.needs_retry());
            assert!(session.black_reply_arrow().is_none());
            let drops = session.destinations(queen);
            assert!(!drops.is_empty());
            for to in drops {
                let mut rescue = session.clone();
                assert!(hive_lib::Bug::can_throw(pill_at, queen_at, to, &rescue.state().board));
                assert_eq!(rescue.play(queen, to), Outcome::LessonDone);
            }
        }
    }
    assert_eq!(successes, 4);
}

fn cover_black_pillbug(session: &mut Session) {
    for to in ["wA1", "bA1", "bP"] {
        let target = hex(&session.state().board, to).unwrap();
        assert_eq!(session.play("wB1".parse().unwrap(), target),
            Outcome::Progress);
    }
}

#[test]
fn neutralisation_starts_as_white_and_adds_a_fixed_tile_that_pins_the_spider() {
    let session = Session::new(find_lesson("bugs.pillbug_deactivate").unwrap()).unwrap();
    assert_eq!(session.learner(), Color::White);
    assert_eq!(session.state().turn_color, Color::White);
    let spider: Piece = "wS1".parse().unwrap();
    assert!(session.state().board.is_pinned(spider));
    let extra = hex(&session.state().board, "wS1/").unwrap();
    assert_eq!(session.state().board.top_piece(extra), Some("wG2".parse().unwrap()));
    for at in session.state().board.all_taken_positions() {
        let piece = session.state().board.top_piece(at).unwrap();
        assert_eq!(session.can_select(piece), ["wB1", "wB2"].contains(&piece.to_string().as_str()));
    }
    let visible = session.lesson().blank_visible().unwrap();
    assert!(visible.contains(&"wQ") && visible.contains(&"bQ") && visible.contains(&"wP") && visible.contains(&"bP"));
    assert!(!visible.contains(&"wS1") && !visible.contains(&"wG2"));
}

#[test]
fn neutralisation_accepts_different_winning_sequences_without_intermediate_parts() {
    for cover_queen in [false, true] {
        let mut session = Session::new(find_lesson("bugs.pillbug_deactivate").unwrap()).unwrap();
        assert_eq!(session.lesson().steps.len(), 2);
        cover_black_pillbug(&mut session);
        let mut route = Vec::new();
        if cover_queen { route.push(("wB1", "bQ")); }
        route.extend([("wB2", "wL"), ("wB2", "bQ\\"), ("wB1", "-bQ")]);
        let last = route.len() - 1;
        for (i, (name, target)) in route.into_iter().enumerate() {
            let to = hex(&session.state().board, target).unwrap();
            assert_eq!(session.play(name.parse().unwrap(), to),
                if i == last { Outcome::LessonDone } else { Outcome::Progress });
            assert!(session.black_reply_arrow().is_none());
        }
        assert_eq!(session.completion_result(), Some("White wins!"));
    }
}

#[test]
fn leaving_the_black_pillbug_available_lets_black_rescue_the_queen() {
    let mut session = Session::new(find_lesson("bugs.pillbug_deactivate").unwrap()).unwrap();
    let queen_at = hex(&session.state().board, "bQ").unwrap();
    let climb = hex(&session.state().board, "wL").unwrap();
    assert_eq!(session.play("wB2".parse().unwrap(), climb), Outcome::Progress);
    let hole = hex(&session.state().board, "bQ\\").unwrap();
    assert_eq!(session.play("wB2".parse().unwrap(), hole), Outcome::Refuted);
    assert!(session.needs_retry());
    assert_eq!(session.state().board.position_of_piece("wB2".parse().unwrap()), Some(hole));
    let escaped = hex(&session.state().board, "bQ").unwrap();
    assert_ne!(escaped, queen_at);
    assert_eq!(session.black_reply_arrow(), Some((queen_at, escaped)));
    session.retry();
    assert!(!session.needs_retry());
    assert_eq!(hex(&session.state().board, "bQ"), Some(queen_at));
}

#[test]
fn neutralisation_hints_do_not_uncover_the_pillbug_before_the_surround_is_ready() {
    let mut session = Session::new(find_lesson("bugs.pillbug_deactivate").unwrap()).unwrap();
    for _ in 0..2 {
        cover_black_pillbug(&mut session);
        let hole = hex(&session.state().board, "-bQ").unwrap();
        assert_eq!(session.play("wB1".parse().unwrap(), hole), Outcome::Refuted);
        session.retry();
    }
    cover_black_pillbug(&mut session);
    for _ in 0..12 {
        let arrows = session.arrows();
        assert_eq!(arrows.len(), 1);
        let (from,to) = arrows[0];
        let piece = session.state().board.top_piece(from).unwrap();
        let result = session.play(piece,to);
        assert!(matches!(result, Outcome::Progress | Outcome::LessonDone));
        if session.is_finished() { break; }
    }
    assert!(session.is_finished());
    assert_eq!(session.completion_result(), Some("White wins!"));
}


#[test]
fn rescue_uses_a_green_queen_outline_and_does_not_reset_before_next() {
    let mut session = Session::new(find_lesson("bugs.pillbug_save").unwrap()).unwrap();
    let queen = "wQ".parse().unwrap();
    assert!(session.red_marks().is_empty());
    assert_eq!(session.green_marks()[0].position, hex(&session.state().board, "wQ").unwrap());
    let to = session.destinations(queen)[0];
    session.play(queen, to);
    let position = session.state().board.clone();
    assert!(session.awaits_continue());
    assert_eq!(session.play("wP".parse().unwrap(), to), Outcome::Illegal);
    assert!(session.state().board == position);
    assert_eq!(session.continue_label(), "Next");
}

#[test]
fn ant_previous_marker_describes_a_legal_move_without_changing_the_position() {
    let session = Session::new(find_lesson("bugs.pillbug_enemy").unwrap()).unwrap();
    let ant = "bA2".parse().unwrap();
    let board = &session.state().board;
    let (Some(from), Some(to)) = board.last_move else { panic!("both move markers are needed"); };
    assert_eq!(board.position_of_piece(ant), Some(to));
    assert!(!board.occupied(from));
    assert!(session.fit_targets().contains(&from));
    let mut previous = board.clone();
    previous.remove(to);
    previous.insert(from, ant, false);
    previous.last_move = (None, None);
    previous.last_moved = None;
    previous.stunned = None;
    let mut previous = State::new_from_position(previous, session.state().game_type, Color::Black).unwrap();
    assert!(previous.play_turn_from_position(ant, to).is_ok());
    for at in board.all_taken_positions() {
        assert_eq!(previous.board.top_piece(at), board.top_piece(at));
    }
}

#[test]
fn pillbug_gate_starts_with_a_red_queen_to_pillbug_arrow() {
    let session = Session::new(find_lesson("bugs.pillbug_gate").unwrap()).unwrap();
    assert_eq!(session.red_arrows(), vec![(
        hex(&session.state().board, "wQ").unwrap(),
        hex(&session.state().board, "wP").unwrap(),
    )]);
}

#[test]
fn black_beetle_covers_an_exposed_white_beetle_and_retry_restores_the_lesson() {
    let mut session = Session::new(find_lesson("bugs.pillbug_deactivate").unwrap()).unwrap();
    let initial = session.state().board.clone();
    cover_black_pillbug(&mut session);
    let to = hex(&session.state().board, "wL").unwrap();
    assert_eq!(session.play("wB2".parse().unwrap(), to), Outcome::Progress);
    let to = hex(&session.state().board, "bL").unwrap();
    assert_eq!(session.play("wB2".parse().unwrap(), to), Outcome::Refuted);
    assert_eq!(session.state().board.top_piece(to), Some("bB1".parse().unwrap()));
    assert_eq!(session.state().board.position_of_piece("wB2".parse().unwrap()), Some(to));
    assert!(session.needs_retry());
    assert!(session.black_reply_arrow().is_some());
    session.retry();
    assert!(session.state().board == initial);
    assert_eq!(session.step_index(), 0);
}

#[test]
fn mosquito_category_has_four_playable_sublessons() {
    let group = super::group_of("bugs.mosquito").unwrap();
    assert_eq!(group.title, "The Mosquito");
    assert_eq!(group.lessons, ["bugs.mosquito", "bugs.mosquito_climb", "bugs.mosquito_ability", "bugs.mosquito_other"]);
    assert_eq!(lessons().nth(31).unwrap().id, "bugs.mosquito");
    assert!(!find_lesson("bugs.mosquito_ability").unwrap().is_pending());
    let initial = Session::new(find_lesson("bugs.mosquito").unwrap()).unwrap();
    let mosquito = "wM".parse().unwrap();
    let start = hex(&initial.state().board, "wM").unwrap();
    let neighbours: HashSet<_> = start.positions_around()
        .filter_map(|at| initial.state().board.top_piece(at)).collect();
    assert_eq!(initial.revealed_neighbours().into_iter().collect::<HashSet<_>>(), neighbours);
    let winning_firsts: HashSet<_> = [
        Position::new(18, 15), Position::new(16, 16),
        Position::new(18, 16), Position::new(18, 18),
    ].into_iter().collect();
    let mut actual = HashSet::new();
    for first in initial.destinations(mosquito) {
        let mut session = initial.clone();
        assert_eq!(session.play(mosquito, first), Outcome::Progress);
        assert!(!session.is_finished());
        assert_eq!(session.state().turn_color, Color::White);
        let neighbours: HashSet<_> = first.positions_around()
            .filter_map(|at| session.state().board.top_piece(at)).collect();
        assert_eq!(session.revealed_neighbours().into_iter().collect::<HashSet<_>>(), neighbours);
        for second in session.destinations(mosquito) {
            let mut attempt = session.clone();
            if attempt.play(mosquito, second) == Outcome::LessonDone {
                actual.insert(first);
                assert_eq!(second, Position::new(15, 18));
                assert_eq!(attempt.completion_result(), Some("White wins!"));
            } else {
                assert!(attempt.needs_retry());
                assert!(!attempt.is_finished());
            }
        }
    }
    assert_eq!(actual, winning_firsts);
}

#[test]
fn mosquito_hints_after_two_errors_solve_the_exercise_in_exactly_two_moves() {
    let mut session = Session::new(find_lesson("bugs.mosquito").unwrap()).unwrap();
    let mosquito = "wM".parse().unwrap();
    for _ in 0..2 {
        assert_eq!(session.play(mosquito, Position::new(16, 14)), Outcome::Progress);
        let wrong = session.destinations(mosquito)[0];
        assert_eq!(session.play(mosquito, wrong), Outcome::Wrong);
        session.retry();
    }
    for i in 0..2 {
        let arrows = session.arrows();
        assert_eq!(arrows.len(), 1);
        assert_eq!(session.play(mosquito, arrows[0].1),
            if i == 0 { Outcome::Progress } else { Outcome::LessonDone });
    }
}


#[test]
fn mosquito_winning_exercise_uses_thinner_frames() {
    let session = Session::new(find_lesson("bugs.mosquito").unwrap()).unwrap();
    assert!(session.current_step().unwrap().thin_frames);
    assert_eq!(session.green_marks().len(), 1);
    assert_eq!(session.red_marks().len(), 1);
}

#[test]
fn copying_enemy_pieces_accepts_all_three_pins_then_requires_both_climbs() {
    let initial = Session::new(find_lesson("bugs.mosquito_climb").unwrap()).unwrap();
    let mosquito = "wM".parse().unwrap();
    assert!(initial.highlights().is_empty());
    for first in [Position::new(16, 19), Position::new(17, 19), Position::new(18, 18)] {
        let mut session = initial.clone();
        assert_eq!(session.play(mosquito, first), Outcome::StepDone);
        assert!(Condition::Pinned("bB1").holds(session.state()));
        let placement = session.state().board.position_of_piece("bA1".parse().unwrap()).unwrap();
        assert!(placement.is_neighbor(hex(&session.state().board, "bS1").unwrap()));
        assert!(session.black_reply_arrow().is_some());
        let beetle = hex(&session.state().board, "bB1").unwrap();
        assert_eq!(session.play(mosquito, beetle), Outcome::StepDone);
        assert!(Condition::OnTop("wM", "bB1").holds(session.state()));
        assert!(!session.is_finished());
        let queen = hex(&session.state().board, "bQ").unwrap();
        assert_eq!(session.play(mosquito, queen), Outcome::LessonDone);
        assert!(Condition::OnTop("wM", "bQ").holds(session.state()));
    }
    let mut session = initial;
    for i in 0..2 {
        let (piece, to) = legal_moves(session.state()).into_iter()
            .find(|(piece, _)| *piece != mosquito).unwrap();
        assert_eq!(session.play(piece, to), Outcome::Wrong);
        session.retry();
        assert_eq!(session.highlights().len(), if i == 0 { 0 } else { 3 });
        assert!(session.arrows().is_empty());
    }
}

#[test]
fn copied_pillbug_ability_rejects_pins_with_an_explanation_and_accepts_both_targets() {
    let initial = Session::new(find_lesson("bugs.mosquito_ability").unwrap()).unwrap();
    let mosquito = "wM".parse().unwrap();
    for first in [Position::new(14, 20), Position::new(15, 20)] {
        let mut session = initial.clone();
        assert_eq!(session.play(mosquito, first), Outcome::Wrong);
        assert!(session.hint().unwrap().contains("One Hive Rule"));
        assert!(session.hint().unwrap().contains("split the hive"));
        assert_eq!(session.red_marks()[0].position, hex(&session.state().board, "bP").unwrap());
        assert!(session.needs_retry());
        session.retry();
        assert!(session.red_marks().is_empty());
        assert!(!session.hint().unwrap().contains("One Hive Rule"));
    }
    let mut lifted = HashSet::new();
    for first in [Position::new(14, 19), Position::new(16, 19)] {
        let mut session = initial.clone();
        assert_eq!(session.play(mosquito, first), Outcome::StepDone);
        assert!(!Condition::Pinned("bP").holds(session.state()));
        assert_eq!(session.state().turn_color, Color::White);
        for name in ["bP", "bQ"] {
            let piece = name.parse().unwrap();
            for to in session.destinations(piece) {
                let mut attempt = session.clone();
                assert_eq!(attempt.play(piece, to), Outcome::LessonDone);
                assert_eq!(attempt.state().board.position_of_piece(piece), Some(to));
                assert_eq!(attempt.state().board.position_of_piece(mosquito), Some(first));
                lifted.insert(name);
            }
        }
    }
    assert_eq!(lifted, HashSet::from(["bP", "bQ"]));
}

#[test]
fn another_mosquito_does_not_lend_beetle_movement_but_an_ant_can_rescue_it() {
    let initial = Session::new(find_lesson("bugs.mosquito_other").unwrap()).unwrap();
    let mosquito = "wM".parse().unwrap();
    let ant = "wA1".parse().unwrap();
    for first in [Position::new(16, 20), Position::new(17, 19)] {
        let mut trapped = initial.clone();
        assert_eq!(trapped.play(mosquito, first), Outcome::StepDone);
        let beetle = hex(&trapped.state().board, "bB1").unwrap();
        assert_eq!(trapped.state().board.top_piece(beetle), Some("bM".parse().unwrap()));
        assert!(trapped.destinations(mosquito).is_empty());
        let mut correct = 0;
        for to in trapped.destinations(ant) {
            let mut once = trapped.clone();
            let outcome = once.play(ant, to);
            if outcome != Outcome::StepDone {
                assert_eq!(outcome, Outcome::Wrong);
                continue;
            }
            correct += 1;
            for _ in 0..8 {
                let mut session = trapped.clone();
                assert_eq!(session.play(ant, to), Outcome::StepDone);
                assert!(Condition::Touching("wA1", "wM").holds(session.state()));
                assert!(!session.destinations(mosquito).is_empty());
                let (piece, at) = session.state().board.last_moved.unwrap();
                assert_eq!(piece.color(), Color::Black);
                assert!(at.is_neighbor(hex(&session.state().board, "bP").unwrap()));
                assert!(initial.state().board.position_of_piece(piece).is_none());
                assert_eq!(session.state().turn_color, Color::White);
                for destination in session.destinations(mosquito) {
                    let mut finish = session.clone();
                    assert_eq!(finish.play(mosquito, destination), Outcome::LessonDone);
                }
            }
        }
        assert_eq!(correct, 2);
    }
}

#[test]
fn pass_lesson_is_last_in_basics_and_starts_after_a_real_forced_black_pass() {
    let lesson = find_lesson("basics.pass").unwrap();
    assert_eq!(CHAPTERS[0].lessons.last().unwrap().id, lesson.id);
    assert_eq!(lessons().nth(7).unwrap().id, lesson.id);
    let initial = Session::new(lesson).unwrap();
    assert_eq!(initial.state().turn_color, Color::White);
    assert_eq!(initial.state().history.moves.last().unwrap().0, "pass");
    assert!(initial.state().board.moves(Color::Black).is_empty());
    assert_eq!(initial.state().board.spawnable_positions(Color::Black).count(), 0);
    let crosses = initial.placement_crosses();
    assert!(!crosses.is_empty());
    for at in crosses {
        assert!(!initial.state().board.occupied(at));
        assert!(!initial.state().board.spawnable(Color::Black, at));
        assert!(at.positions_around().any(|p| initial.state().board.top_piece(p)
            .is_some_and(|piece| piece.color() == Color::Black)));
        assert!(initial.fit_targets().contains(&at));
    }
    let mut white = initial.state().clone();
    assert!(white.play_turn_from_history("pass", "").is_err());
    let mut movements = 0;
    let mut placements = 0;
    for (piece, to) in legal_moves(initial.state()) {
        let mut session = initial.clone();
        if initial.state().board.position_of_piece(piece).is_some() { movements += 1; }
        else { placements += 1; }
        assert_eq!(session.play(piece, to), Outcome::LessonDone);
        if session.state().board.is_shutout(Color::Black, session.state().game_type) {
            assert_eq!(session.state().turn_color, Color::White);
            assert_eq!(session.state().history.moves.last().unwrap().0, "pass");
        } else {
            assert_eq!(session.state().turn_color, Color::Black);
        }
    }
    assert!(movements > 0 && placements > 0);
    assert!(lesson.brief.contains("TODO part two"));
}
