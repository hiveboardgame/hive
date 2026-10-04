use super::{
    curriculum::{find_lesson, lessons, neighbours, CHAPTERS},
    goal::{spec_pieces, Condition, Goal},
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
        assert!(!lesson.intro.is_empty(), "{} has no intro", lesson.id);
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
                Goal::Move { to, .. } => {
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
                Goal::Continue | Goal::MoveAny | Goal::Reach(_) => {}
            }
            if let Reply::Opponent { piece, to } = step.then {
                assert!(
                    !piece.is_empty() && !to.is_empty(),
                    "{}: scripted reply not filled in",
                    lesson.id
                );
            }
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
        assert_eq!(
            session.highlights().len(),
            step.highlights.len(),
            "{}: a highlight does not resolve",
            lesson.id
        );
        assert_eq!(
            session.arrows().len(),
            step.arrows.len(),
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
fn a_wrong_move_is_undone() {
    let mut session = Session::new(&PLACE_QUEEN).expect("test lesson loads");
    let (queen, position) = spawn_of(&session, |piece| piece.bug() == hive_lib::Bug::Queen);
    assert_eq!(session.play(queen, position), Outcome::StepDone);
    let before = session.state().board.clone();
    let (piece, position) = spawn_of(&session, |piece| piece.bug() == hive_lib::Bug::Ant);

    assert_eq!(session.play(piece, position), Outcome::Wrong);
    assert!(session.state().board == before);
    assert_eq!(session.step_index(), 1);
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
                || session.arrows().len() != current.arrows.len()
                || session.marks().len() != current.marks.len()
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
        if session.marks().len() != closing.marks.len() {
            return Err("the closing step points at a hex that does not exist".to_string());
        }
        if session.state().turn_color != session.learner() {
            return Err("the scripted reply before the closing step failed".to_string());
        }
    }
    Ok(())
}

fn search(session: &Session, step: usize, depth: usize) -> Option<Session> {
    if depth == 0 {
        return None;
    }
    let mut continuations = Vec::new();
    for (piece, position) in legal_moves(session.state()) {
        let mut next = session.clone();
        match next.play(piece, position) {
            Outcome::StepDone | Outcome::LessonDone => return Some(next),
            Outcome::Visited => continuations.push(next),
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
    match &step.goal {
        Goal::Continue | Goal::MoveAny => {}
        Goal::Move { piece, .. } | Goal::Visit { piece, .. } => specs.push(*piece),
        Goal::Reach(condition) => condition_specs(condition, &mut specs),
    }
    specs
}

fn condition_specs(condition: &Condition, specs: &mut Vec<&'static str>) {
    match condition {
        Condition::AnyPlaced(_) => {}
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
        | Condition::ShutOut(_) => {}
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
fn the_learner_never_moves_twice_in_a_row() {
    let mut offenders = Vec::new();
    for lesson in lessons() {
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
