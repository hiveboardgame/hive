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
    BlankPosition(&'static str),
    BlankExercise {
        position: &'static str,
        movable: &'static [&'static str],
        swap: Option<(&'static str, &'static str)>,
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
    pub win_prompt: Option<(hive_lib::Color, &'static str)>,
    pub announce_winner: bool,
    pub red_arrows: &'static [(&'static str, &'static str)],
    pub hint_moves: Option<(&'static str, u8)>,
    pub wrong_reply: Option<&'static str>,
}

pub enum Reply {
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
            win_prompt: None,
            announce_winner: false,
            red_arrows: &[],
            hint_moves: None,
            wrong_reply: None,
        }
    }

    pub const fn play(prompt: &'static str, goal: Goal, hint: &'static str) -> Self {
        Self {
            prompt,
            goal,
            marks: &[],
            number_marks: true,
            refutations: &[],
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
            win_prompt: None,
            announce_winner: false,
            red_arrows: &[],
            hint_moves: None,
            wrong_reply: None,
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

    pub const fn reply_on_mistake(mut self, piece: &'static str) -> Self {
        self.wrong_reply = Some(piece);
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
        matches!(self.setup, Setup::Position(_) | Setup::BlankPosition(_) | Setup::BlankExercise { .. } | Setup::AfterLesson { .. } | Setup::SwapPieces { .. } | Setup::Clocked { .. })
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
