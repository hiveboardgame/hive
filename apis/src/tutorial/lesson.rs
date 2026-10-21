use super::goal::Goal;

pub struct Chapter {
    pub id: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub lessons: &'static [Lesson],
}

pub struct Lesson {
    pub id: &'static str,
    pub title: &'static str,
    pub intro: &'static [&'static str],
    /// The authoring spec for `hop`: what the position must show for the steps to work.
    pub brief: &'static str,
    pub setup: Setup,
    pub steps: &'static [Step],
    pub outro: &'static str,
}

pub enum Setup {
    Position(&'static str),
    ForcedPass { position: &'static str, color: hive_lib::Color },
    BlankPosition(&'static str),
    BlankExercise {
        position: &'static str,
        movable: &'static [&'static str],
        swap: Option<(&'static str, &'static str)>,
        visible: Option<&'static [&'static str]>,
        start_as: Option<hive_lib::Color>,
        extras: &'static [(&'static str, &'static str)],
        reveal_neighbours: Option<&'static str>,
    },
    AfterLesson {
        lesson: &'static str,
        fallback: &'static str,
    },
    SwapPieces {
        position: &'static str,
        first: &'static str,
        second: &'static str,
    },
    Clocked {
        moves: &'static str,
        base_minutes: u64,
        increment_seconds: u64,
    },
    Pending,
    TextOnly,
}

pub struct Step {
    pub prompt: &'static str,
    pub goal: Goal,
    pub marks: &'static [&'static str],
    pub number_marks: bool,
    pub refutations: &'static [(&'static str, &'static str)],
    pub conditional_refutation: Option<(&'static super::goal::Condition, &'static str, &'static str)>,
    pub highlights: &'static [&'static str],
    pub arrows: &'static [(&'static str, &'static str)],
    pub hint: &'static str,
    pub completion_tip: &'static str,
    pub then: Reply,
    pub grid: bool,
    pub history: bool,
    pub hint_after: Option<u8>,
    pub your_turn: bool,
    pub forbidden: &'static [&'static str],
    pub hint_highlights: &'static [&'static str],
    pub placement_crosses: Option<&'static str>,
    pub immobile_marks: bool,
    pub red_marks: &'static [&'static str],
    pub green_marks: &'static [&'static str],
    pub previous_move_from: Option<&'static str>,
    pub continue_label: &'static str,
    pub thin_frames: bool,
    pub conditional_hint: Option<(&'static super::goal::Condition, &'static str)>,
    pub conditional_red_marks: Option<(&'static super::goal::Condition, &'static [&'static str])>,
    pub crosses_for: Option<hive_lib::Color>,
    pub win_prompt: Option<(hive_lib::Color, &'static str)>,
    pub announce_winner: bool,
    pub red_arrows: &'static [(&'static str, &'static str)],
    pub hint_moves: Option<(&'static str, u8)>,
    pub wrong_reply: Option<&'static str>,
    pub complete_on_load: bool,
    pub small_arrows: bool,
    pub legal_arrows: Option<&'static str>,
    pub normal_moves_only: bool,
    pub random_wrong_reply: bool,
    pub route_hint_after: Option<u8>,
    pub retry_after_moves: Option<u8>,
    pub escape_reply: Option<&'static str>,
    pub cover_reply: Option<(&'static [&'static str], &'static [&'static str])>,
}

pub enum Reply {
    InitialPosition,
    RandomPlacementNear { near: &'static str },
    RandomPreservingWin {
        piece: &'static str,
        to: &'static str,
    },
    LearnerAgain,
    NextPlayer(hive_lib::Color),
    Opponent {
        piece: &'static str,
        to: &'static str,
    },
    OpponentPlacement {
        piece: &'static str,
        near: Option<&'static str>,
    },
}

impl Step {
    pub const fn read(prompt: &'static str, marks: &'static [&'static str]) -> Self {
        Self {
            prompt,
            goal: Goal::Continue,
            marks,
            number_marks: true,
            refutations: &[],
            conditional_refutation: None,
            highlights: &[],
            arrows: &[],
            hint: "",
            completion_tip: "",
            then: Reply::LearnerAgain,
            grid: false,
            history: false,
            hint_after: None,
            your_turn: false,
            forbidden: &[],
            hint_highlights: &[],
            placement_crosses: None,
            immobile_marks: false,
            red_marks: &[],
            green_marks: &[],
            previous_move_from: None,
            continue_label: "Continue",
            thin_frames: false,
            conditional_hint: None,
            conditional_red_marks: None,
            crosses_for: None,
            win_prompt: None,
            announce_winner: false,
            red_arrows: &[],
            hint_moves: None,
            wrong_reply: None,
            complete_on_load: false,
            small_arrows: false,
            legal_arrows: None,
            normal_moves_only: false,
            random_wrong_reply: false,
            route_hint_after: None,
            retry_after_moves: None,
            escape_reply: None,
            cover_reply: None,
        }
    }

    pub const fn play(prompt: &'static str, goal: Goal, hint: &'static str) -> Self {
        Self {
            prompt,
            goal,
            marks: &[],
            number_marks: true,
            refutations: &[],
            conditional_refutation: None,
            highlights: &[],
            arrows: &[],
            hint,
            completion_tip: "",
            then: Reply::LearnerAgain,
            grid: false,
            history: false,
            hint_after: None,
            your_turn: false,
            forbidden: &[],
            hint_highlights: &[],
            placement_crosses: None,
            immobile_marks: false,
            red_marks: &[],
            green_marks: &[],
            previous_move_from: None,
            continue_label: "Continue",
            thin_frames: false,
            conditional_hint: None,
            conditional_red_marks: None,
            crosses_for: None,
            win_prompt: None,
            announce_winner: false,
            red_arrows: &[],
            hint_moves: None,
            wrong_reply: None,
            complete_on_load: false,
            small_arrows: false,
            legal_arrows: None,
            normal_moves_only: false,
            random_wrong_reply: false,
            route_hint_after: None,
            retry_after_moves: None,
            escape_reply: None,
            cover_reply: None,
        }
    }

    pub const fn marked(mut self, marks: &'static [&'static str]) -> Self {
        self.marks = marks;
        self
    }

    pub const fn highlighted(mut self, highlights: &'static [&'static str]) -> Self {
        self.highlights = highlights;
        self
    }

    pub const fn without_mark_numbers(mut self) -> Self {
        self.number_marks = false;
        self
    }

    pub const fn with_arrows(mut self, arrows: &'static [(&'static str, &'static str)]) -> Self {
        self.arrows = arrows;
        self
    }

    pub const fn punished_by(
        mut self,
        refutations: &'static [(&'static str, &'static str)],
    ) -> Self {
        self.refutations = refutations;
        self
    }

    pub const fn punished_when(
        mut self,
        condition: &'static super::goal::Condition,
        piece: &'static str,
        to: &'static str,
    ) -> Self {
        self.conditional_refutation = Some((condition, piece, to));
        self
    }

    pub const fn with_grid(mut self) -> Self {
        self.grid = true;
        self
    }

    pub const fn with_history(mut self) -> Self {
        self.history = true;
        self
    }

    pub const fn with_completion_tip(mut self, completion_tip: &'static str) -> Self {
        self.completion_tip = completion_tip;
        self
    }

    pub const fn reply(mut self, piece: &'static str, to: &'static str) -> Self {
        self.then = Reply::Opponent { piece, to };
        self
    }

    /// Begin the next part from the lesson's initial diagram, keeping progress.
    pub const fn restart_position(mut self) -> Self {
        self.then = Reply::InitialPosition;
        self
    }

    pub const fn hint_after(mut self, attempts: u8) -> Self {
        self.hint_after = Some(attempts);
        self
    }

    pub const fn forbidden(mut self, hexes: &'static [&'static str]) -> Self {
        self.forbidden = hexes;
        self
    }

    pub const fn highlight_hint_after(
        mut self,
        attempts: u8,
        hexes: &'static [&'static str],
    ) -> Self {
        self.hint_after = Some(attempts);
        self.hint_highlights = hexes;
        self
    }

    pub const fn next_player(mut self, color: hive_lib::Color) -> Self {
        self.then = Reply::NextPlayer(color);
        self.your_turn = true;
        self
    }

    pub const fn placement_crosses(mut self, around: &'static str) -> Self {
        self.placement_crosses = Some(around);
        self
    }

    pub const fn marked_when_immobile(mut self, pieces: &'static [&'static str]) -> Self {
        self.marks = pieces;
        self.number_marks = false;
        self.immobile_marks = true;
        self
    }

    pub const fn outlined_red(mut self, pieces: &'static [&'static str]) -> Self {
        self.red_marks = pieces;
        self
    }

    pub const fn with_win_prompt(mut self, color: hive_lib::Color, prompt: &'static str) -> Self {
        self.win_prompt = Some((color, prompt));
        self
    }

    pub const fn announce_winner(mut self) -> Self {
        self.announce_winner = true;
        self
    }

    pub const fn with_red_arrows(mut self, arrows: &'static [(&'static str, &'static str)]) -> Self {
        self.red_arrows = arrows;
        self
    }

    pub const fn hint_after_moves(mut self, piece: &'static str, moves: u8) -> Self {
        self.hint_moves = Some((piece, moves));
        self
    }

    pub const fn random_reply_preserving_win(mut self, piece: &'static str, to: &'static str) -> Self {
        self.then = Reply::RandomPreservingWin { piece, to };
        self
    }

    pub const fn random_reply_on_mistake(mut self) -> Self {
        self.random_wrong_reply = true;
        self
    }

    pub const fn retry_after_moves(mut self, limit: u8) -> Self {
        self.retry_after_moves = Some(limit);
        self
    }

    pub const fn guided_exploration(mut self, hint_after: u8, retry_after: u8) -> Self {
        self.route_hint_after = Some(hint_after);
        self.retry_after_moves = Some(retry_after);
        self
    }

    pub const fn outlined_green(mut self, pieces: &'static [&'static str]) -> Self {
        self.green_marks = pieces;
        self
    }

    /// Supply an authored previous hex when HOP only records the last mover.
    pub const fn previous_move_from(mut self, from: &'static str) -> Self {
        self.previous_move_from = Some(from);
        self
    }

    pub const fn thin_frames(mut self) -> Self {
        self.thin_frames = true;
        self
    }

    pub const fn explain_when(
        mut self,
        condition: &'static super::goal::Condition,
        hint: &'static str,
        marks: &'static [&'static str],
    ) -> Self {
        self.conditional_hint = Some((condition, hint));
        self.conditional_red_marks = Some((condition, marks));
        self
    }

    pub const fn placement_crosses_for(mut self, color: hive_lib::Color) -> Self {
        self.crosses_for = Some(color);
        self
    }

    pub const fn random_placement_near(mut self, near: &'static str) -> Self {
        self.then = Reply::RandomPlacementNear { near };
        self
    }

    pub const fn next_button(mut self) -> Self {
        self.continue_label = "Next";
        self
    }

    /// Punish a move if an opponent Beetle can legally cover a named piece.
    pub const fn punish_cover(
        mut self,
        beetles: &'static [&'static str],
        targets: &'static [&'static str],
    ) -> Self {
        self.cover_reply = Some((beetles, targets));
        self
    }

    /// Let an opponent piece escape immediately when a learner move frees it.
    pub const fn punish_escape(mut self, piece: &'static str) -> Self {
        self.escape_reply = Some(piece);
        self
    }

    pub const fn reply_on_mistake(mut self, piece: &'static str) -> Self {
        self.wrong_reply = Some(piece);
        self
    }

    pub const fn observation(mut self) -> Self {
        self.complete_on_load = true;
        self
    }

    pub const fn with_small_arrows(mut self, arrows: &'static [(&'static str, &'static str)]) -> Self {
        self.arrows = arrows;
        self.small_arrows = true;
        self
    }

    /// Show the named piece's current legal destinations; a move hint replaces these arrows.
    pub const fn with_legal_arrows(mut self, piece: &'static str) -> Self {
        self.legal_arrows = Some(piece);
        self
    }

    pub const fn normal_moves_only(mut self) -> Self {
        self.normal_moves_only = true;
        self
    }

    pub const fn your_turn(mut self) -> Self {
        self.your_turn = true;
        self
    }

    pub const fn reply_placement_near(mut self, piece: &'static str, near: &'static str) -> Self {
        self.then = Reply::OpponentPlacement {
            piece,
            near: Some(near),
        };
        self
    }

    pub const fn reply_placement(mut self, piece: &'static str) -> Self {
        self.then = Reply::OpponentPlacement { piece, near: None };
        self
    }
}

impl Lesson {
    pub fn is_pending(&self) -> bool {
        matches!(self.setup, Setup::Pending)
    }

    pub fn is_text_only(&self) -> bool {
        matches!(self.setup, Setup::TextOnly)
    }

    pub fn has_board(&self) -> bool {
        matches!(self.setup, Setup::ForcedPass { .. } | Setup::Position(_) | Setup::BlankPosition(_) | Setup::BlankExercise { .. } | Setup::AfterLesson { .. } | Setup::SwapPieces { .. } | Setup::Clocked { .. })
    }

    pub fn uses_blank_tiles(&self) -> bool {
        matches!(self.setup, Setup::BlankPosition(_) | Setup::BlankExercise { .. })
    }

    pub fn blank_movers(&self) -> Option<&'static [&'static str]> {
        match self.setup {
            Setup::BlankPosition(_) => Some(&["wQ"]),
            Setup::BlankExercise { movable, .. } => Some(movable),
            _ => None,
        }
    }

    pub fn blank_visible(&self) -> Option<&'static [&'static str]> {
        match self.setup {
            Setup::BlankExercise { visible, .. } => visible,
            _ => None,
        }
    }

    pub fn clock(&self) -> Option<(u64, u64)> {
        match self.setup {
            Setup::Clocked {
                base_minutes,
                increment_seconds,
                ..
            } => Some((base_minutes, increment_seconds)),
            _ => None,
        }
    }
}
