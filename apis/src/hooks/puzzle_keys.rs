use crate::providers::puzzle::{PuzzleContext, PuzzlePhase};
use leptos::{ev::keydown, prelude::*};
use leptos_use::{use_event_listener, use_window};
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, KeyboardEvent};

/// Puzzle shortcuts: `n` next (or skip an untouched puzzle), `h` hint, `r` try again, `s` show the
/// solution, arrows step through a shown solution, `Esc` drops a selection or unconfirmed move.
pub fn use_puzzle_keys(puzzle: PuzzleContext, next: Callback<()>) {
    let signed_in = crate::providers::puzzle::is_signed_in();
    let costs_rating = move || puzzle.costs_rating(signed_in.get_untracked());
    let body = use_window().document().body();
    _ = use_event_listener(body, keydown, move |evt: KeyboardEvent| {
        if evt.alt_key() || evt.ctrl_key() || evt.meta_key() || typing(&evt) {
            return;
        }
        let phase = puzzle.phase.get_untracked();
        match evt.key().as_str() {
            "n" | "N" if phase.is_over() || phase == PuzzlePhase::Empty => next.run(()),
            "n" | "N" if phase == PuzzlePhase::Solving && puzzle.can_skip() => puzzle.skip(next),
            "n" | "N" if phase == PuzzlePhase::Solving && !costs_rating() => next.run(()),
            "ArrowLeft" if phase == PuzzlePhase::Revealed => {
                let step = puzzle.reveal_step.get_untracked();
                puzzle.show_solution_step(step.saturating_sub(1), true);
            }
            "ArrowRight" if phase == PuzzlePhase::Revealed => {
                let step = puzzle.reveal_step.get_untracked();
                puzzle.show_solution_step((step + 1).min(puzzle.solution_steps()), true);
            }
            "h" | "H" => puzzle.hint(costs_rating()),
            "r" | "R" if phase == PuzzlePhase::Failed => puzzle.retry(),
            "r" | "R" if phase.is_over() => puzzle.restart(),
            "s" | "S" if phase == PuzzlePhase::Solving => puzzle.request_reveal(costs_rating()),
            "s" | "S" if phase == PuzzlePhase::Failed => puzzle.reveal(),
            "Escape" => {
                puzzle.game_state.clear_selection();
                puzzle.notice.set(None);
                puzzle.reveal_armed.set(false);
            }
            _ => return,
        }
        evt.prevent_default();
    });
}

fn typing(evt: &KeyboardEvent) -> bool {
    evt.target()
        .and_then(|target| target.dyn_into::<HtmlElement>().ok())
        .is_some_and(|element| {
            matches!(element.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                || element.is_content_editable()
        })
}
