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
    pub refutations: &'static [(&'static str, &'static str)],
    pub highlights: &'static [&'static str],
    pub arrows: &'static [(&'static str, &'static str)],
    pub hint: &'static str,
    pub then: Reply,
    pub grid: bool,
    pub history: bool,
}

pub enum Reply {
    LearnerAgain,
    Opponent {
        piece: &'static str,
        to: &'static str,
    },
}

impl Step {
    pub const fn read(prompt: &'static str, marks: &'static [&'static str]) -> Self {
        Self {
            prompt,
            goal: Goal::Continue,
            marks,
            refutations: &[],
            highlights: &[],
            arrows: &[],
            hint: "",
            then: Reply::LearnerAgain,
            grid: false,
            history: false,
        }
    }

    pub const fn play(prompt: &'static str, goal: Goal, hint: &'static str) -> Self {
        Self {
            prompt,
            goal,
            marks: &[],
            refutations: &[],
            highlights: &[],
            arrows: &[],
            hint,
            then: Reply::LearnerAgain,
            grid: false,
            history: false,
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

    pub const fn reply(mut self, piece: &'static str, to: &'static str) -> Self {
        self.then = Reply::Opponent { piece, to };
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
        matches!(self.setup, Setup::Position(_) | Setup::Clocked { .. })
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
