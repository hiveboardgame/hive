use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy)]
pub enum EvalStatus {
    Queued,
    Running,
    Done,
    Failed,
}

impl fmt::Display for EvalStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            EvalStatus::Queued => "queued",
            EvalStatus::Running => "running",
            EvalStatus::Done => "done",
            EvalStatus::Failed => "failed",
        };
        write!(f, "{s}")
    }
}

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum EvalStatusError {
    #[error("{found} is not a valid EvalStatus")]
    InvalidEvalStatus { found: String },
}

impl FromStr for EvalStatus {
    type Err = EvalStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(EvalStatus::Queued),
            "running" => Ok(EvalStatus::Running),
            "done" => Ok(EvalStatus::Done),
            "failed" => Ok(EvalStatus::Failed),
            s => Err(EvalStatusError::InvalidEvalStatus {
                found: s.to_string(),
            }),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy, PartialOrd, Ord)]
pub enum Grade {
    Inaccuracy,
    Mistake,
    Blunder,
}

impl Grade {
    /// Lichess's thresholds, converted from its [-1, 1] winning-chances scale to win percent.
    pub fn from_loss(loss: f32) -> Option<Grade> {
        if loss >= 15.0 {
            Some(Grade::Blunder)
        } else if loss >= 10.0 {
            Some(Grade::Mistake)
        } else if loss >= 5.0 {
            Some(Grade::Inaccuracy)
        } else {
            None
        }
    }

    pub fn glyph(&self) -> &'static str {
        match self {
            Grade::Inaccuracy => "?!",
            Grade::Mistake => "?",
            Grade::Blunder => "??",
        }
    }
}

/// The engine's judgement of one played move. Win chances are percentages from the mover's
/// point of view.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct MoveEval {
    pub played: String,
    pub best: String,
    /// The engine's expected continuation, starting with `best`.
    pub line: Vec<String>,
    pub before: f32,
    pub after: f32,
    pub sims: u32,
}

impl MoveEval {
    pub fn loss(&self) -> f32 {
        if self.played == self.best {
            0.0
        } else {
            (self.before - self.after).max(0.0)
        }
    }

    pub fn grade(&self) -> Option<Grade> {
        Grade::from_loss(self.loss())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct EvalResult {
    /// One entry per move of the game, in order. `None` for a move the engine could not judge:
    /// one made from a position with no legal alternative.
    pub moves: Vec<Option<MoveEval>>,
}

pub const MAX_EVAL_LINE: usize = 12;

/// Why a game cannot get an eval, or None if it can. The net only knows Base+MLP under the
/// tournament queen rule, and only finished games are evaluated so an eval can't help a player
/// mid-game.
pub fn eval_unavailable_reason(
    finished: bool,
    game_type: &str,
    tournament_queen_rule: bool,
    moves: usize,
) -> Option<&'static str> {
    if !finished {
        Some("Evals are available once the game is over.")
    } else if game_type != hive_lib::GameType::MLP.to_string() || !tournament_queen_rule {
        Some("Evals are only available for Base+MLP games with the tournament queen rule.")
    } else if moves == 0 {
        Some("This game has no moves to evaluate.")
    } else {
        None
    }
}

/// The moves of a stored game history (`"wL ;bG1 wL/;"`), as UHP move strings.
pub fn history_moves(history: &str) -> Vec<String> {
    history
        .split(';')
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_string)
        .collect()
}
const MAX_MOVE_LEN: usize = 16;

#[derive(Error, Debug, PartialEq)]
pub enum EvalResultError {
    #[error("expected {expected} moves, got {found}")]
    WrongLength { expected: usize, found: usize },
    #[error("move {ply}: played {found}, the game has {expected}")]
    WrongMove {
        ply: usize,
        expected: String,
        found: String,
    },
    #[error("move {ply}: {reason}")]
    Invalid { ply: usize, reason: String },
}

impl EvalResult {
    /// Checks a worker's result against the game it claims to describe before it is stored.
    pub fn validate(&self, history: &[String]) -> Result<(), EvalResultError> {
        if self.moves.len() != history.len() {
            return Err(EvalResultError::WrongLength {
                expected: history.len(),
                found: self.moves.len(),
            });
        }
        for (ply, (eval, played)) in self.moves.iter().zip(history).enumerate() {
            let Some(eval) = eval else { continue };
            if &eval.played != played {
                return Err(EvalResultError::WrongMove {
                    ply,
                    expected: played.clone(),
                    found: eval.played.clone(),
                });
            }
            let invalid = |reason: &str| EvalResultError::Invalid {
                ply,
                reason: reason.to_string(),
            };
            if !(0.0..=100.0).contains(&eval.before) || !(0.0..=100.0).contains(&eval.after) {
                return Err(invalid("win chance outside 0..=100"));
            }
            if eval.line.len() > MAX_EVAL_LINE {
                return Err(invalid("line too long"));
            }
            if eval.line.first().is_some_and(|first| first != &eval.best) {
                return Err(invalid("line does not start with the best move"));
            }
            if std::iter::once(&eval.best)
                .chain(&eval.line)
                .any(|m| m.is_empty() || m.len() > MAX_MOVE_LEN)
            {
                return Err(invalid("malformed move"));
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct EvalJob {
    pub eval_id: uuid::Uuid,
    pub game_type: String,
    pub moves: Vec<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct EvalClaim {
    pub worker: String,
    /// Whether the site may hand out a game of its own choosing when no user is waiting.
    /// Defaults to yes, so a worker built before this field existed keeps working.
    #[serde(default = "auto_by_default")]
    pub auto: bool,
}

fn auto_by_default() -> bool {
    true
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct EvalProgress {
    pub worker: String,
    pub progress_pct: u8,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct EvalSubmission {
    pub worker: String,
    /// Engine build, net digest and search scheme, so a stored eval says what produced it.
    pub engine: String,
    pub result: EvalResult,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct EvalFailure {
    pub worker: String,
    pub error: String,
}

/// A finished eval as the front page lists it.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct RecentEval {
    pub game_id: crate::GameId,
    pub white: String,
    pub black: String,
    /// Ratings when the game was played.
    pub white_rating: Option<f64>,
    pub black_rating: Option<f64>,
    /// `GameStatus` and `Conclusion` as stored, e.g. "Finished(1-0)" and "Board".
    pub game_status: String,
    pub conclusion: String,
    pub moves: i32,
    pub tournament: Option<String>,
    /// None once the requester's account is gone, or for an automatic eval.
    pub requested_by: Option<String>,
    /// Started by the site itself on an idle queue.
    pub automatic: bool,
    pub finished_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct RecentEvals {
    pub recent: Vec<RecentEval>,
    /// Evals queued or running right now.
    pub in_line: usize,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum GameEvalView {
    NotRequested,
    /// `wait_secs` is None while no eval worker is running.
    Queued {
        position: usize,
        /// Evals waiting in all, this one included.
        queue_len: usize,
        wait_secs: Option<u64>,
    },
    Running {
        progress_pct: u8,
        wait_secs: u64,
    },
    Done(EvalResult),
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(played: &str, best: &str, before: f32, after: f32) -> MoveEval {
        MoveEval {
            played: played.to_string(),
            best: best.to_string(),
            line: vec![best.to_string()],
            before,
            after,
            sims: 800,
        }
    }

    #[test]
    fn history_moves_drops_the_first_moves_padding_and_the_trailing_separator() {
        assert_eq!(
            history_moves("wL ;bG1 wL/;wM /wL;"),
            vec!["wL", "bG1 wL/", "wM /wL"]
        );
        assert_eq!(history_moves(""), Vec::<String>::new());
    }

    #[test]
    fn only_finished_mlp_tournament_rule_games_get_evals() {
        assert_eq!(eval_unavailable_reason(true, "Base+MLP", true, 40), None);
        assert!(eval_unavailable_reason(false, "Base+MLP", true, 40).is_some());
        assert!(eval_unavailable_reason(true, "Base+MLP", false, 40).is_some());
        assert!(eval_unavailable_reason(true, "Base", true, 40).is_some());
        assert!(eval_unavailable_reason(true, "Base+MLP", true, 0).is_some());
    }

    #[test]
    fn a_claim_from_a_worker_without_the_auto_field_still_takes_auto_evals() {
        let claim: EvalClaim = serde_json::from_str(r#"{"worker":"prod-1"}"#).unwrap();
        assert!(claim.auto);
    }

    #[test]
    fn grade_thresholds_follow_lichess() {
        assert_eq!(Grade::from_loss(4.9), None);
        assert_eq!(Grade::from_loss(5.0), Some(Grade::Inaccuracy));
        assert_eq!(Grade::from_loss(10.0), Some(Grade::Mistake));
        assert_eq!(Grade::from_loss(15.0), Some(Grade::Blunder));
    }

    #[test]
    fn playing_the_best_move_costs_nothing_even_if_the_evals_disagree() {
        // Two separate searches can rate the same line differently; the move is still best.
        assert_eq!(eval("wQ wL\\", "wQ wL\\", 60.0, 40.0).grade(), None);
    }

    #[test]
    fn a_move_that_gains_is_not_negative_loss() {
        assert_eq!(eval("wQ wL\\", "wA1 -wL", 40.0, 55.0).loss(), 0.0);
    }

    #[test]
    fn status_round_trips_through_its_db_spelling() {
        for status in [
            EvalStatus::Queued,
            EvalStatus::Running,
            EvalStatus::Done,
            EvalStatus::Failed,
        ] {
            assert_eq!(status.to_string().parse::<EvalStatus>().unwrap(), status);
        }
    }

    #[test]
    fn validate_accepts_a_result_matching_the_game() {
        let history = vec!["wL".to_string(), "bG1 wL/".to_string()];
        let result = EvalResult {
            moves: vec![
                Some(eval("wL", "wP", 65.0, 62.0)),
                Some(eval("bG1 wL/", "bP wL/", 37.0, 35.0)),
            ],
        };
        assert_eq!(result.validate(&history), Ok(()));
    }

    #[test]
    fn validate_rejects_a_result_for_another_game() {
        let history = vec!["wL".to_string()];
        let result = EvalResult {
            moves: vec![Some(eval("wP", "wP", 65.0, 65.0))],
        };
        assert!(matches!(
            result.validate(&history),
            Err(EvalResultError::WrongMove { ply: 0, .. })
        ));
    }

    #[test]
    fn validate_rejects_a_truncated_result() {
        let history = vec!["wL".to_string(), "bG1 wL/".to_string()];
        let result = EvalResult {
            moves: vec![Some(eval("wL", "wP", 65.0, 62.0))],
        };
        assert_eq!(
            result.validate(&history),
            Err(EvalResultError::WrongLength {
                expected: 2,
                found: 1
            })
        );
    }

    #[test]
    fn validate_rejects_out_of_range_win_chances() {
        let history = vec!["wL".to_string()];
        let result = EvalResult {
            moves: vec![Some(eval("wL", "wP", 165.0, 62.0))],
        };
        assert!(matches!(
            result.validate(&history),
            Err(EvalResultError::Invalid { ply: 0, .. })
        ));
    }

    #[test]
    fn validate_rejects_a_line_that_does_not_start_with_the_best_move() {
        let history = vec!["wL".to_string()];
        let mut bad = eval("wL", "wP", 65.0, 62.0);
        bad.line = vec!["wG1".to_string()];
        let result = EvalResult {
            moves: vec![Some(bad)],
        };
        assert!(matches!(
            result.validate(&history),
            Err(EvalResultError::Invalid { ply: 0, .. })
        ));
    }
}
