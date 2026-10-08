use crate::{
    functions::game_evals::request_game_eval,
    hooks::history_nav::should_ignore_history_key_event,
    providers::{
        analysis::AnalysisContext,
        game_eval::{wordings, GameEvalContext},
        game_state::{GameStateStore, GameStateStoreFields},
        AuthContext,
    },
};
use leptos::{ev::keydown, prelude::*, task::spawn_local};
use leptos_use::{use_event_listener, use_window};
use shared_types::{eval_unavailable_reason, EvalResult, GameEvalView};
use web_sys::KeyboardEvent;

fn wait_text(secs: u64) -> String {
    match secs {
        0..=59 => "under a minute".to_string(),
        60..=119 => "about a minute".to_string(),
        s => format!("about {} minutes", s.div_ceil(60)),
    }
}

#[component]
pub fn Evals() -> impl IntoView {
    let evals = expect_context::<GameEvalContext>();
    let game_state = expect_context::<GameStateStore>();
    let auth = expect_context::<AuthContext>();
    let game_response = game_state.game_response();
    let error = RwSignal::new(None::<String>);
    let requesting = RwSignal::new(false);

    let unavailable = Memo::new(move |_| {
        game_response.with(|game| {
            game.as_ref().map(|game| {
                eval_unavailable_reason(
                    game.finished,
                    &game.game_type.to_string(),
                    game.tournament_queen_rule,
                    game.history.len(),
                )
            })
        })
    });
    let logged_in = Memo::new(move |_| auth.user.with(Option::is_some));

    let request = move |_| {
        let Some(game_id) = game_response.with_untracked(|g| g.as_ref().map(|g| g.game_id.clone()))
        else {
            return;
        };
        requesting.set(true);
        error.set(None);
        spawn_local(async move {
            match request_game_eval(game_id).await {
                Ok(view) => evals.view.set(Some(view)),
                Err(e) => error.set(Some(server_message(&e))),
            }
            requesting.set(false);
        });
    };

    let request_button = move |label: &'static str| {
        view! {
            <Show
                when=move || logged_in.get()
                fallback=|| {
                    view! {
                        <p class="text-sm text-gray-600 dark:text-gray-300">
                            "Log in to request a computer eval of this game."
                        </p>
                    }
                }
            >
                <button
                    type="button"
                    class="self-start ui-button ui-button-primary ui-button-sm"
                    disabled=move || requesting.get()
                    on:click=request
                >
                    {label}
                </button>
            </Show>
        }
    };

    let body = move || {
        match evals.view.get() {
        None => match unavailable.get() {
            None => view! {
                <p class="text-sm text-gray-600 dark:text-gray-300">
                    "Open a finished game to see its eval."
                </p>
            }
            .into_any(),
            Some(_) => view! { <p class="text-sm text-gray-500">"Loading…"</p> }.into_any(),
        },
        Some(GameEvalView::NotRequested) => match unavailable.get().flatten() {
            Some(reason) => {
                view! { <p class="text-sm text-gray-600 dark:text-gray-300">{reason}</p> }
                    .into_any()
            }
            None => view! {
                <div class="flex flex-col gap-2">
                    <p class="text-sm text-gray-600 dark:text-gray-300">
                        "The engine marks inaccuracies, mistakes and blunders and shows a better move for each."
                    </p>
                    {request_button("Request computer eval")}
                </div>
            }
            .into_any(),
        },
        Some(GameEvalView::Queued {
            position,
            queue_len,
            wait_secs,
        }) => {
            let place = format!("Waiting in line: #{position} of {queue_len}.");
            view! {
                <p class="text-sm">
                    {match wait_secs {
                        Some(secs) => format!("{place} Ready in {}.", wait_text(secs)),
                        None => {
                            format!(
                                "{place} The eval engine is not running right now; this game will be evaluated once it is back.",
                            )
                        }
                    }}
                </p>
            }
            .into_any()
        }
        Some(GameEvalView::Running {
            progress_pct,
            wait_secs,
        }) => view! {
            <div class="flex flex-col gap-1">
                <p class="text-sm">
                    {format!(
                        "The engine is looking at this game. Ready in {}.",
                        wait_text(wait_secs),
                    )}
                </p>
                <div
                    class="overflow-hidden h-2 rounded bg-black/10 dark:bg-white/10"
                    role="progressbar"
                    aria-valuenow=progress_pct
                    aria-valuemin="0"
                    aria-valuemax="100"
                >
                    <div
                        class="h-full bg-pillbug-teal"
                        style:width=format!("{progress_pct}%")
                    ></div>
                </div>
            </div>
        }
        .into_any(),
        Some(GameEvalView::Failed) => view! {
            <div class="flex flex-col gap-2">
                <p class="text-sm">"The eval of this game could not be completed."</p>
                {request_button("Try again")}
            </div>
        }
        .into_any(),
        Some(GameEvalView::Done(result)) => view! { <EvalDone result /> }.into_any(),
    }
    };

    // Fills the sidebar so only the move list scrolls, keeping the description above it in view.
    view! {
        <div class="flex flex-col flex-1 gap-3 min-h-0">
            {body}
            {move || {
                error
                    .get()
                    .map(|message| {
                        view! { <p class="text-sm text-red-700 dark:text-red-400">{message}</p> }
                    })
            }}
        </div>
    }
}

/// Server function errors arrive prefixed with their kind; the user only needs the reason.
fn server_message(error: &ServerFnError) -> String {
    match error {
        ServerFnError::ServerError(message) => message.clone(),
        _ => "Could not reach the server. Try again.".to_string(),
    }
}

/// Puts the board on the position before move `ply`, adds the engine's line there as a
/// variation and selects its first move, so the arrow keys step through the rest.
fn play_line(analysis: AnalysisContext, game_state: GameStateStore, ply: usize, line: &[String]) {
    analysis.reset_preview(game_state);
    if !analysis.store.select_main_ply(Some(ply), game_state) {
        return;
    }
    let appended: Vec<((String, String), u64)> = game_state
        .state()
        .try_maybe_update(|state| {
            let start = state.history.moves.len();
            for mv in line {
                let (piece, position) = mv.split_once(' ').unwrap_or((mv.as_str(), ""));
                if state.play_turn_from_history(piece, position).is_err() {
                    break;
                }
            }
            let appended: Vec<_> = state.history.moves[start..]
                .iter()
                .cloned()
                .zip(state.hashes[start..].iter().copied())
                .collect();
            (!appended.is_empty(), appended)
        })
        .unwrap_or_default();
    let Some(steps_back) = appended.len().checked_sub(1) else {
        return;
    };
    analysis.store.append_moves(appended, game_state);
    for _ in 0..steps_back {
        match analysis.store.previous_history_target_node_id() {
            Some(parent) => {
                analysis.select_node(parent, game_state);
            }
            None => break,
        }
    }
    analysis.sync_reserve_from_game_state(game_state);
}

/// Puts the board on the position the graded move `ply` was played from.
fn select_graded(analysis: AnalysisContext, game_state: GameStateStore, ply: usize) {
    analysis.reset_preview(game_state);
    if analysis.store.select_main_ply(Some(ply), game_state) {
        analysis.sync_reserve_from_game_state(game_state);
    }
}

/// The graded move after (or before) the board's position. Off the list, e.g. on the game's
/// final position where the page opens, it wraps round, so the first press always lands.
fn neighbour(graded: &[usize], current: Option<usize>, forward: bool) -> Option<usize> {
    let next = match (current, forward) {
        (Some(at), true) => graded.iter().copied().find(|ply| *ply > at),
        (Some(at), false) => graded.iter().rev().copied().find(|ply| *ply < at),
        (None, _) => None,
    };
    let on_graded = current.is_some_and(|at| graded.contains(&at));
    match (next, forward) {
        (None, true) if !on_graded => graded.first().copied(),
        (None, false) if !on_graded => graded.last().copied(),
        _ => next,
    }
}

fn scroll_eval_into_view(ply: usize) {
    let row = use_window()
        .as_ref()
        .and_then(|window| window.document())
        .and_then(|document| {
            document
                .query_selector(&format!("[data-eval-ply='{ply}']"))
                .ok()
        })
        .flatten();
    if let Some(row) = row {
        row.scroll_into_view_with_bool(false);
    }
}

#[component]
fn EvalDone(result: EvalResult) -> impl IntoView {
    let analysis = expect_context::<AnalysisContext>();
    let game_state = expect_context::<GameStateStore>();
    let game_response = game_state.game_response();
    let selected_ply = Memo::new(move |_| analysis.store.selected_game_ply());
    let graded: Vec<_> = result
        .moves
        .iter()
        .enumerate()
        .filter_map(|(ply, eval)| {
            let eval = eval.clone()?;
            let grade = eval.grade()?;
            Some((ply, eval, grade))
        })
        .collect();
    let nothing_graded = graded.is_empty();
    let graded_plies: Vec<usize> = graded.iter().map(|(ply, ..)| *ply).collect();
    // Only while this tab is shown: Up and Down mean nothing elsewhere on the page.
    _ = use_event_listener(
        use_window().document().body(),
        keydown,
        move |evt: KeyboardEvent| {
            let forward = match evt.key().as_str() {
                "ArrowDown" => true,
                "ArrowUp" => false,
                _ => return,
            };
            if should_ignore_history_key_event(&evt) {
                return;
            }
            evt.prevent_default();
            let current = analysis.store.selected_game_ply();
            if let Some(ply) = neighbour(&graded_plies, current, forward) {
                select_graded(analysis, game_state, ply);
                scroll_eval_into_view(ply);
            }
        },
    );
    let words = StoredValue::new(game_response.with_untracked(|game| {
        game.as_ref()
            .map(|g| wordings(g.game_type, g.tournament_queen_rule, &g.history, &result))
            .unwrap_or_default()
    }));
    let wording = move |ply: usize| words.with_value(|all| all.get(ply).copied().flatten());
    let evals = expect_context::<GameEvalContext>();
    let decision = move || {
        let ply = selected_ply.get()?;
        let eval = evals.move_eval(ply)?;
        let grade = eval.grade()?;
        let headline = wording(ply)
            .map(|w| w.headline(ply + 1, ply % 2 == 0, grade))
            .unwrap_or_else(|| format!("Move {}: {} {}", ply + 1, eval.played, grade.glyph()));
        Some(view! {
            <div class="flex flex-col gap-1 p-2 text-sm rounded border border-orange-twilight bg-orange-twilight/20">
                <span>{headline}</span>
                <span class="font-mono text-xs text-gray-600 dark:text-gray-300">
                    {format!("{} {}  ·  better {}", eval.played, grade.glyph(), eval.best)}
                </span>
            </div>
        })
    };

    let rows = graded
        .into_iter()
        .map(|(ply, eval, grade)| {
            let jump = move |_| select_graded(analysis, game_state, ply);
            let (played_words, best_words) = match wording(ply) {
                Some(w) => (w.played_short(), w.best_short()),
                None => (eval.played.clone(), eval.best.clone()),
            };
            let line = eval.line.clone();
            let show_line = move |event: leptos::ev::MouseEvent| {
                event.stop_propagation();
                play_line(analysis, game_state, ply, &line);
            };
            view! {
                <li data-eval-ply=ply>
                    <button
                        type="button"
                        class="flex flex-col gap-0.5 py-1.5 px-2 w-full text-left rounded transition-colors dark:hover:bg-pillbug-teal/15 hover:bg-blue-light/70"
                        class=("bg-orange-twilight/40", move || selected_ply.get() == Some(ply))
                        on:click=jump
                    >
                        <span class="text-sm">
                            <span class="font-semibold">
                                {format!("{}. {} {}", ply + 1, played_words, grade.glyph())}
                            </span>
                            <span class="ml-1.5 font-mono text-xs text-gray-500 dark:text-gray-400">
                                {eval.played.clone()}
                            </span>
                        </span>
                        <span class="text-sm">
                            "Better: "
                            <span class="font-semibold text-green-700 dark:text-green-400">
                                {best_words}
                            </span>
                            <span class="ml-1.5 font-mono text-xs text-gray-500 dark:text-gray-400">
                                {eval.best.clone()}
                            </span>
                        </span>
                    </button>
                    <Show when=move || selected_ply.get() == Some(ply)>
                        <button
                            type="button"
                            class="mx-2 mb-1.5 ui-button ui-button-sm"
                            on:click=show_line.clone()
                        >
                            "Show line"
                        </button>
                    </Show>
                </li>
            }
        })
        .collect_view();

    view! {
        {decision}
        <Show
            when=move || !nothing_graded
            fallback=|| {
                view! {
                    <p class="text-sm">"The engine found no inaccuracies, mistakes or blunders."</p>
                }
            }
        >
            <p class="text-xs text-gray-600 dark:text-gray-300">
                "Pick a move, or step through them with ↑ and ↓, to see it on the board: red is what was played, green is better."
            </p>
        </Show>
        <ul class="flex overflow-y-auto flex-col flex-1 min-h-0">{rows}</ul>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_step_between_graded_moves() {
        let graded = [3, 4, 8, 15];
        assert_eq!(neighbour(&graded, Some(4), true), Some(8));
        assert_eq!(neighbour(&graded, Some(4), false), Some(3));
        assert_eq!(
            neighbour(&graded, Some(5), true),
            Some(8),
            "from between two, the next one"
        );
        assert_eq!(
            neighbour(&graded, Some(15), true),
            None,
            "no wrapping past the last"
        );
        assert_eq!(
            neighbour(&graded, Some(3), false),
            None,
            "nor before the first"
        );
        assert_eq!(
            neighbour(&graded, Some(40), true),
            Some(3),
            "from the game's end, the first"
        );
        assert_eq!(neighbour(&graded, Some(40), false), Some(15));
        assert_eq!(
            neighbour(&graded, Some(0), false),
            Some(15),
            "from the start, the last"
        );
        assert_eq!(neighbour(&graded, None, false), Some(15));
        assert_eq!(
            neighbour(&graded, None, true),
            Some(3),
            "from a variation, the first"
        );
        assert_eq!(neighbour(&[], Some(2), true), None);
    }

    #[test]
    fn waits_are_rounded_up_to_whole_minutes() {
        assert_eq!(wait_text(30), "under a minute");
        assert_eq!(wait_text(90), "about a minute");
        assert_eq!(wait_text(121), "about 3 minutes");
    }
}
