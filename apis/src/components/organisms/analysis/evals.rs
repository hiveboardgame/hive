use super::sidebar::use_set_query;
use crate::{
    components::atoms::grade_mark::{grade_fill_class, grade_name, GradeMark},
    functions::game_evals::{cancel_game_eval, request_game_eval},
    hooks::history_nav::should_ignore_history_key_event,
    providers::{
        analysis::{AnalysisContext, AnalysisPreviewSnapshot},
        game_eval::{line_words, wordings, GameEvalContext},
        game_state::{GameStateStore, GameStateStoreFields},
        AuthContext,
        Config,
    },
};
use leptos::{
    ev::{keydown, MouseEvent},
    prelude::*,
    reactive::effect::batch,
    task::spawn_local,
};
use leptos_router::hooks::use_query_map;
use leptos_use::{use_event_listener, use_window};
use shared_types::{eval_unavailable_reason, EvalResult, GameEvalView, Grade};
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

fn wait_text(secs: u64) -> String {
    match secs {
        0..=59 => "under a minute".to_string(),
        60..=119 => "about a minute".to_string(),
        s => format!("about {} minutes", s.div_ceil(60)),
    }
}

/// Where Tailwind's `lg` layout puts the board beside the panel instead of above it.
const SIDE_BY_SIDE_WIDTH: f64 = 1024.0;

const LEAVE_NOTE: &str =
    "You can leave this page: the eval carries on and is here when you come back.";

#[component]
fn Spinner() -> impl IntoView {
    view! {
        <span
            class="inline-block w-3.5 h-3.5 rounded-full border-2 animate-spin shrink-0 border-pillbug-teal border-t-transparent"
            aria-hidden="true"
        ></span>
    }
}

#[component]
pub fn Evals() -> impl IntoView {
    let evals = expect_context::<GameEvalContext>();
    let game_state = expect_context::<GameStateStore>();
    let auth = expect_context::<AuthContext>();
    let game_response = game_state.game_response();
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    // A click before the page has finished loading reaches no handler and is lost, so the
    // buttons wait for it.
    let hydrated = RwSignal::new(false);
    Effect::new(move |_| hydrated.set(true));
    evals.panels.update(|n| *n += 1);
    on_cleanup(move || evals.panels.update(|n| *n = n.saturating_sub(1)));

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
    let game_id = move || game_response.with_untracked(|g| g.as_ref().map(|g| g.game_id.clone()));

    let request = move |_| {
        let Some(game_id) = game_id() else {
            return;
        };
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            match request_game_eval(game_id).await {
                Ok(view) => evals.view.set(Some(view)),
                Err(e) => error.set(Some(server_message(&e))),
            }
            busy.set(false);
        });
    };
    let cancel = move |_| {
        let Some(game_id) = game_id() else {
            return;
        };
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            match cancel_game_eval(game_id).await {
                Ok(view) => evals.view.set(Some(view)),
                Err(e) => error.set(Some(server_message(&e))),
            }
            busy.set(false);
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
                    disabled=move || busy.get() || !hydrated.get()
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
            Some(_) => view! {
                <p class="flex gap-2 items-center text-sm text-gray-500">
                    <Spinner />
                    "Loading…"
                </p>
            }
            .into_any(),
        },
        Some(GameEvalView::NotRequested) => match unavailable.get().flatten() {
            Some(reason) => {
                view! { <p class="text-sm text-gray-600 dark:text-gray-300">{reason}</p> }
                    .into_any()
            }
            None => view! {
                <div class="flex flex-col gap-2">
                    <p class="text-sm text-gray-600 dark:text-gray-300">
                        "The engine replays the game, marks inaccuracies (?!), mistakes (?) and blunders (??), and shows a better move for each."
                    </p>
                    {request_button("Request computer eval")}
                </div>
            }
            .into_any(),
        },
        Some(GameEvalView::Queued {
            position,
            queue_len,
            mine,
            wait_secs,
        }) => {
            let place = format!("Waiting in line: #{position} of {queue_len}.");
            let status = match wait_secs {
                Some(secs) => format!("{place} Ready in {}.", wait_text(secs)),
                None => {
                    format!(
                        "{place} The eval engine is not running right now; this game will be evaluated once it is back.",
                    )
                }
            };
            view! {
                <div class="flex flex-col gap-2 text-sm" role="status">
                    <p class="flex gap-2 items-center">
                        <Spinner />
                        {status}
                    </p>
                    <p class="text-xs text-gray-600 dark:text-gray-300">{LEAVE_NOTE}</p>
                    <Show when=move || mine>
                        <button
                            type="button"
                            class="self-start ui-button ui-button-sm"
                            disabled=move || busy.get() || !hydrated.get()
                            on:click=cancel
                        >
                            "Cancel request"
                        </button>
                    </Show>
                </div>
            }
            .into_any()
        }
        Some(GameEvalView::Running {
            progress_pct,
            wait_secs,
        }) => view! {
            <div class="flex flex-col gap-1.5" role="status">
                <p class="flex gap-2 items-center text-sm">
                    <Spinner />
                    {format!(
                        "The engine is looking at this game ({progress_pct}% done). Ready in {}.",
                        wait_text(wait_secs),
                    )}
                </p>
                <div
                    class="overflow-hidden h-2 rounded bg-black/10 dark:bg-white/10"
                    role="progressbar"
                    aria-label="Eval progress"
                    aria-valuenow=progress_pct
                    aria-valuemin="0"
                    aria-valuemax="100"
                >
                    <div
                        class="h-full transition-all bg-pillbug-teal"
                        style:width=format!("{progress_pct}%")
                    ></div>
                </div>
                <p class="text-xs text-gray-600 dark:text-gray-300">{LEAVE_NOTE}</p>
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
        <div class="flex flex-col flex-1 gap-3 min-h-0" data-eval-panel="">
            {body}
            {move || {
                error
                    .get()
                    .map(|message| {
                        view! {
                            <p class="text-sm text-red-700 dark:text-red-400" role="alert">
                                {message}
                            </p>
                        }
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

/// Puts the board on the position the graded move `ply` was played from.
fn select_graded(analysis: AnalysisContext, game_state: GameStateStore, ply: usize) {
    analysis.reset_preview(game_state);
    if analysis.store.select_main_ply(Some(ply), game_state) {
        analysis.sync_reserve_from_game_state(game_state);
    }
}

/// Shows the board `steps` moves into the engine's line after the graded move `ply`, as a
/// preview over the real position: the analysis tree is never touched, and any navigation
/// drops the preview. Returns how many of the moves could be played.
fn preview_line(
    analysis: AnalysisContext,
    game_state: GameStateStore,
    ply: usize,
    line: &[String],
    steps: usize,
) -> usize {
    analysis.reset_preview(game_state);
    if untrack(|| analysis.store.selected_game_ply()) != Some(ply) {
        select_graded(analysis, game_state, ply);
    }
    if steps == 0 {
        return 0;
    }
    let base = game_state.state().get_untracked();
    let mut shown = base.clone();
    let mut played = 0;
    for mv in line.iter().take(steps) {
        let (piece, position) = mv.split_once(' ').unwrap_or((mv.as_str(), ""));
        if shown.play_turn_from_history(piece, position).is_err() {
            break;
        }
        played += 1;
    }
    analysis.preview.set(Some(AnalysisPreviewSnapshot {
        node_id: analysis.store.selected_node_id_untracked(),
        state: base,
        generation: analysis.store.document_generation_untracked(),
    }));
    batch(move || {
        game_state.state().set(shown);
        game_state.move_info().update(|move_info| move_info.reset());
    });
    played
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

fn grade_plural(grade: Grade, count: usize) -> String {
    let name = grade_name(grade);
    match (count, name.strip_suffix('y')) {
        (1, _) => name.to_string(),
        (_, Some(stem)) => format!("{stem}ies"),
        (_, None) => format!("{name}s"),
    }
}

fn severity(grade: Grade) -> u8 {
    match grade {
        Grade::Inaccuracy => 0,
        Grade::Mistake => 1,
        Grade::Blunder => 2,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Filter {
    white: Option<bool>,
    worst_from: Grade,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            white: None,
            worst_from: Grade::Inaccuracy,
        }
    }
}

impl Filter {
    fn shows(&self, ply: usize, grade: Grade) -> bool {
        self.white
            .is_none_or(|white| ply.is_multiple_of(2) == white)
            && severity(grade) >= severity(self.worst_from)
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
    let Some(row) = row.and_then(|row| row.dyn_into::<web_sys::HtmlElement>().ok()) else {
        return;
    };
    // Scrolls the list, not the page: on a phone the board must stay where it is.
    let Some(list) = row
        .closest("ul")
        .ok()
        .flatten()
        .and_then(|list| list.dyn_into::<web_sys::HtmlElement>().ok())
    else {
        return;
    };
    let (top, bottom) = (row.offset_top(), row.offset_top() + row.offset_height());
    let (view_top, view_height) = (list.scroll_top() + list.offset_top(), list.client_height());
    if top < view_top {
        list.set_scroll_top(top - list.offset_top());
    } else if bottom > view_top + view_height {
        list.set_scroll_top(bottom - list.offset_top() - view_height);
    }
}

/// On a phone the board sits above the panel, out of sight while picking from the list.
fn bring_board_into_view() {
    let Some(window) = use_window().as_ref().cloned() else {
        return;
    };
    let narrow = window
        .inner_width()
        .ok()
        .and_then(|width| width.as_f64())
        .is_some_and(|width| width < SIDE_BY_SIDE_WIDTH);
    if !narrow {
        return;
    }
    if let Some(board) = window
        .document()
        .and_then(|document| document.query_selector("[data-main-board]").ok().flatten())
    {
        board.scroll_into_view_with_bool(true);
    }
}

fn curve_points(curve: &[f32], width: f32, height: f32) -> Vec<(f32, f32)> {
    let steps = curve.len().saturating_sub(1).max(1) as f32;
    curve
        .iter()
        .enumerate()
        .map(|(i, chance)| {
            (
                i as f32 / steps * width,
                height - chance.clamp(0.0, 100.0) / 100.0 * height,
            )
        })
        .collect()
}

/// The judged move nearest to a click `fraction` of the way along the graph, where move `ply`
/// is drawn at `ply + 1` of `moves` steps.
fn nearest_mark(marks: &[usize], moves: usize, fraction: f64) -> Option<usize> {
    let at = fraction * moves.max(1) as f64 - 1.0;
    marks
        .iter()
        .copied()
        .min_by(|a, b| (*a as f64 - at).abs().total_cmp(&(*b as f64 - at).abs()))
}

const GRAPH_WIDTH: f32 = 300.0;
const GRAPH_HEIGHT: f32 = 64.0;
const TICK_HALF: f32 = 9.0;

#[component]
fn WinGraph(
    curve: Vec<f32>,
    marks: Vec<(usize, Grade)>,
    selected: Memo<Option<usize>>,
    on_pick: Callback<usize>,
) -> impl IntoView {
    let moves = curve.len().saturating_sub(1);
    let points = curve_points(&curve, GRAPH_WIDTH, GRAPH_HEIGHT);
    let line = points
        .iter()
        .map(|(x, y)| format!("{x:.2},{y:.2}"))
        .collect::<Vec<_>>()
        .join(" ");
    let area = format!("M0,{GRAPH_HEIGHT} L{line} L{GRAPH_WIDTH},{GRAPH_HEIGHT} Z");
    let x_of = move |ply: usize| (ply + 1) as f32 / moves.max(1) as f32 * GRAPH_WIDTH;
    let mark_plies = StoredValue::new(marks.iter().map(|(ply, _)| *ply).collect::<Vec<_>>());
    let grades = StoredValue::new(marks.clone());
    let curve = StoredValue::new(curve);
    // The position after this many moves, under the pointer.
    let hovered = RwSignal::new(None::<usize>);
    let fraction_of = |event: &MouseEvent| {
        let target = event
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())?;
        let rect = target.get_bounding_client_rect();
        (rect.width() > 0.0).then(|| (event.client_x() as f64 - rect.left()) / rect.width())
    };
    let pick = move |event: MouseEvent| {
        if let Some(ply) = fraction_of(&event).and_then(|fraction| {
            mark_plies.with_value(|marks| nearest_mark(marks, moves, fraction))
        }) {
            on_pick.run(ply);
        }
    };
    let hover = move |event: MouseEvent| {
        hovered.set(
            fraction_of(&event)
                .map(|fraction| ((fraction * moves as f64).round().max(0.0) as usize).min(moves)),
        );
    };
    let keys = move |event: web_sys::KeyboardEvent| {
        let forward = match event.key().as_str() {
            "ArrowRight" => true,
            "ArrowLeft" => false,
            _ => return,
        };
        event.prevent_default();
        let next =
            mark_plies.with_value(|marks| neighbour(marks, selected.get_untracked(), forward));
        if let Some(ply) = next {
            on_pick.run(ply);
        }
    };
    let readout = move || match hovered.get() {
        None => "Hover the graph for the win chances after any move.".to_string(),
        Some(after) => {
            let white = curve.with_value(|curve| curve.get(after).copied().unwrap_or(50.0));
            let mark = after
                .checked_sub(1)
                .and_then(|ply| grades.with_value(|g| g.iter().find(|(p, _)| *p == ply).copied()))
                .map(|(_, grade)| {
                    format!(
                        " · move {after} was {} {}",
                        grade_name(grade),
                        grade.glyph()
                    )
                })
                .unwrap_or_default();
            let when = if after == 0 {
                "Start".to_string()
            } else {
                format!("After move {after}")
            };
            format!(
                "{when}: White {:.0}% · Black {:.0}%{mark}",
                white,
                100.0 - white
            )
        }
    };
    let ticks = marks
        .into_iter()
        .map(|(ply, grade)| {
            let x = x_of(ply);
            let y = points.get(ply + 1).map_or(GRAPH_HEIGHT / 2.0, |(_, y)| *y);
            let colour = match grade {
                Grade::Inaccuracy => "#facc15",
                Grade::Mistake => "#f97316",
                Grade::Blunder => "#dc2626",
            };
            view! {
                <line
                    x1=x
                    x2=x
                    y1=(y - TICK_HALF).max(0.0)
                    y2=(y + TICK_HALF).min(GRAPH_HEIGHT)
                    stroke=colour
                    stroke-width="3"
                    stroke-linecap="round"
                    vector-effect="non-scaling-stroke"
                />
            }
        })
        .collect_view();
    let marker = move |ply: Option<usize>, colour: &'static str, width: &'static str| {
        ply.map(|x| {
            view! {
                <line
                    x1=x as f32 / moves.max(1) as f32 * GRAPH_WIDTH
                    x2=x as f32 / moves.max(1) as f32 * GRAPH_WIDTH
                    y1="0"
                    y2=GRAPH_HEIGHT
                    stroke=colour
                    stroke-width=width
                    vector-effect="non-scaling-stroke"
                />
            }
        })
    };
    view! {
        <div class="flex flex-col gap-0.5 shrink-0">
            <p class="flex justify-between text-gray-600 dark:text-gray-300 text-[11px]">
                <span>"White's win chance: up is White ahead, down is Black ahead"</span>
            </p>
            <div class="relative">
                <svg
                    class="w-full h-24 bg-gray-700 rounded cursor-pointer dark:bg-gray-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-pillbug-teal"
                    viewBox=format!("0 0 {GRAPH_WIDTH} {GRAPH_HEIGHT}")
                    preserveAspectRatio="none"
                    tabindex="0"
                    data-own-arrows=""
                    role="img"
                    aria-label="White's win chance over the game. Use the left and right arrow keys to step through the marked moves."
                    on:click=pick
                    on:mousemove=hover
                    on:mouseleave=move |_| hovered.set(None)
                    on:keydown=keys
                >
                    <path d=area fill="#f3f4f6" />
                    <line
                        x1="0"
                        x2=GRAPH_WIDTH
                        y1=GRAPH_HEIGHT / 2.0
                        y2=GRAPH_HEIGHT / 2.0
                        stroke="#9ca3af"
                        stroke-width="1"
                        stroke-dasharray="3 3"
                        vector-effect="non-scaling-stroke"
                    />
                    {ticks}
                    {move || marker(hovered.get(), "#9ca3af", "1")}
                    {move || marker(selected.get().map(|ply| ply + 1), "#0d9488", "3")}
                </svg>
            </div>
            <p
                class="hidden text-xs tabular-nums text-gray-600 lg:block dark:text-gray-300"
                aria-hidden="true"
            >
                {readout}
            </p>
        </div>
    }
}

/// The picked move's verdict right under the board, for phones, where the Evals panel is a
/// screen further down. It takes its own room: laid over the board it hid the very move.
#[component]
pub fn EvalBoardNote() -> impl IntoView {
    let evals = expect_context::<GameEvalContext>();
    let step = move |forward: bool| {
        if let Some(step) = evals.step.get_untracked() {
            step.run(forward);
        }
    };
    let to_list = move |_| {
        if let Some(panel) = use_window()
            .as_ref()
            .and_then(|window| window.document())
            .and_then(|document| document.query_selector("[data-eval-panel]").ok().flatten())
        {
            panel.scroll_into_view_with_bool(true);
        }
    };
    move || {
        evals.note.get().map(|note| {
            let in_line = evals.in_line.get();
            let at_end = in_line.is_some_and(|(at, len)| at >= len);
            view! {
                <div class="flex flex-col gap-1 p-2 mx-1 text-xs rounded border dark:bg-gray-900 shrink-0 border-orange-twilight bg-orange-twilight/30">
                    <p class="line-clamp-3">{note}</p>
                    <div class="flex gap-1.5 justify-end">
                        <button
                            type="button"
                            class="ui-button ui-button-sm"
                            on:click=move |_| step(false)
                        >
                            {if in_line.is_some() { "◀" } else { "◀ Previous" }}
                        </button>
                        <button
                            type="button"
                            class="ui-button ui-button-sm"
                            disabled=at_end
                            on:click=move |_| step(true)
                        >
                            {if in_line.is_some() { "▶" } else { "Next ▶" }}
                        </button>
                        {if in_line.is_some() {
                            view! {
                                <button
                                    type="button"
                                    class="ui-button ui-button-primary ui-button-sm"
                                    on:click=move |_| {
                                        if let Some(back) = evals.back.get_untracked() {
                                            back.run(());
                                        }
                                    }
                                >
                                    "Back to the game"
                                </button>
                            }
                                .into_any()
                        } else {
                            view! {
                                <button
                                    type="button"
                                    class="ui-button ui-button-primary ui-button-sm"
                                    on:click=to_list
                                >
                                    "List ↓"
                                </button>
                            }
                                .into_any()
                        }}
                    </div>
                </div>
            }
        })
    }
}

#[component]
fn SideHex(white: bool) -> impl IntoView {
    let name = if white { "White" } else { "Black" };
    view! {
        <svg
            class="inline-block w-2.5 h-3 shrink-0"
            viewBox="0 0 10 11.5"
            role="img"
            aria-label=name
        >
            <title>{name}</title>
            <polygon
                points="5,0.5 9.5,3.1 9.5,8.4 5,11 0.5,8.4 0.5,3.1"
                fill=if white { "#ffffff" } else { "#1f2937" }
                stroke="#6b7280"
                stroke-width="1"
                stroke-linejoin="round"
            />
        </svg>
    }
}

#[component]
fn EvalDone(result: EvalResult) -> impl IntoView {
    let analysis = expect_context::<AnalysisContext>();
    let game_state = expect_context::<GameStateStore>();
    let evals = expect_context::<GameEvalContext>();
    let game_response = game_state.game_response();
    let selected_ply = Memo::new(move |_| analysis.store.selected_game_ply());
    let filter = RwSignal::new(Filter::default());
    let Config(config, set_config) = expect_context::<Config>();
    let notation = move || config.with(|c| c.eval_notation);
    let set_notation = move |on: bool| {
        set_config.update(|c| {
            if let Some(c) = c {
                c.eval_notation = on;
            }
        })
    };
    // The graded move whose engine line is on the board, and how far into it.
    let line_at = RwSignal::new(None::<(usize, usize)>);
    Effect::watch(
        move || selected_ply.get(),
        move |selected, _, _| {
            if line_at
                .get_untracked()
                .is_some_and(|(ply, _)| Some(ply) != *selected)
            {
                line_at.set(None);
            }
        },
        false,
    );
    // Navigating away (History, arrow keys) undoes the preview; the line player follows.
    Effect::new(move |_| {
        if analysis.preview.with(Option::is_none)
            && line_at.get_untracked().is_some_and(|(_, step)| step > 0)
        {
            line_at.set(None);
        }
    });

    let names = game_response.with_untracked(|game| {
        game.as_ref()
            .map(|g| {
                (
                    g.white_player.username.clone(),
                    g.black_player.username.clone(),
                )
            })
            .unwrap_or_else(|| ("White".to_string(), "Black".to_string()))
    });
    let graded: Vec<(usize, shared_types::MoveEval, Grade)> = result
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
    let graded_index: StoredValue<Vec<(usize, Grade)>> = StoredValue::new(
        graded
            .iter()
            .map(|(ply, _, grade)| (*ply, *grade))
            .collect(),
    );
    let shown_plies = move || {
        let filter = filter.get_untracked();
        graded_index.with_value(|all| {
            all.iter()
                .filter(|(ply, grade)| filter.shows(*ply, *grade))
                .map(|(ply, _)| *ply)
                .collect::<Vec<_>>()
        })
    };
    let pick = move |ply: usize| {
        line_at.set(None);
        select_graded(analysis, game_state, ply);
        scroll_eval_into_view(ply);
    };
    // `?eval=43` names the judged move the way the panel does ("Move 43"); `?move=` counts
    // positions, so a judged move would read one off.
    let set_query = use_set_query();
    let linked = use_query_map().with_untracked(|query| {
        query
            .get("eval")
            .and_then(|n| n.parse::<usize>().ok())
            .and_then(|n| n.checked_sub(1))
    });
    Effect::watch(
        move || selected_ply.get(),
        move |selected, _, _| {
            let judged = selected.filter(|ply| evals.grade(*ply).is_some());
            set_query.run(vec![
                ("tab", Some("evals".to_string())),
                ("eval", judged.map(|ply| (ply + 1).to_string())),
                ("move", None),
            ]);
        },
        false,
    );
    Effect::new(move |_| {
        if let Some(ply) = linked.filter(|ply| evals.grade(*ply).is_some()) {
            pick(ply);
        }
    });
    let step = move |forward: bool| {
        let current = analysis.store.selected_game_ply();
        if let Some(ply) = neighbour(&shown_plies(), current, forward) {
            pick(ply);
        }
    };
    let words = StoredValue::new(game_response.with_untracked(|game| {
        game.as_ref()
            .map(|g| wordings(g.game_type, g.tournament_queen_rule, &g.history, &result))
            .unwrap_or_default()
    }));
    let wording = move |ply: usize| words.with_value(|all| all.get(ply).copied().flatten());
    let show_line_step = move |ply: usize, steps: usize| {
        let Some(eval) = evals.move_eval(ply) else {
            return;
        };
        let played = preview_line(analysis, game_state, ply, &eval.line, steps);
        line_at.set(Some((ply, played)));
        bring_board_into_view();
    };
    let line_len_of = move |ply: usize| evals.move_eval(ply).map_or(0, |eval| eval.line.len());
    let leave_line = move || {
        if let Some((ply, _)) = line_at.get_untracked() {
            show_line_step(ply, 0);
            line_at.set(None);
        }
    };
    // In the engine's line the steps go through the line, anywhere else through the marks.
    let step_any = move |forward: bool| match line_at.get_untracked() {
        Some((ply, at)) if at > 0 => {
            let to = if forward {
                (at + 1).min(line_len_of(ply))
            } else {
                at - 1
            };
            show_line_step(ply, to);
        }
        _ => step(forward),
    };
    evals.step.set(Some(Callback::new(step_any)));
    evals.back.set(Some(Callback::new(move |()| leave_line())));
    // The board's own ← and → would leave the line; while it is open they step through it.
    Effect::new(move |_| {
        let open = line_at.with(|line| line.is_some_and(|(_, at)| at > 0));
        evals.in_line.set(
            line_at
                .get()
                .filter(|(_, at)| *at > 0)
                .map(|(ply, at)| (at, line_len_of(ply))),
        );
        if let Some(body) = use_window().document().body() {
            if open {
                _ = body.set_attribute("data-own-arrows", "");
            } else {
                _ = body.remove_attribute("data-own-arrows");
            }
        }
    });
    Effect::new(move |_| {
        let note = match line_at.get() {
            Some((ply, at)) if at > 0 => game_response.with_untracked(|game| {
                let game = game.as_ref()?;
                let line = evals.move_eval(ply)?.line;
                let words = line_words(
                    game.game_type,
                    game.tournament_queen_rule,
                    &game.history,
                    ply,
                    &line,
                );
                Some(format!(
                    "Engine's line instead of move {}, {at} of {}: {}",
                    ply + 1,
                    words.len(),
                    words.get(at - 1).cloned().unwrap_or_default()
                ))
            }),
            _ => selected_ply.get().and_then(|ply| {
                let grade = evals.move_eval(ply)?.grade()?;
                Some(
                    wording(ply)
                        .map(|w| w.headline(ply + 1, grade))
                        .unwrap_or_else(|| format!("Move {}: {}", ply + 1, grade.glyph())),
                )
            }),
        };
        evals.note.set(note);
    });
    on_cleanup(move || {
        evals.note.set(None);
        evals.step.set(None);
        evals.back.set(None);
        evals.in_line.set(None);
        if let Some(body) = use_window().document().body() {
            _ = body.remove_attribute("data-own-arrows");
        }
    });
    // Only while this tab is shown: Up and Down mean nothing elsewhere on the page.
    _ = use_event_listener(
        use_window().document().body(),
        keydown,
        move |evt: KeyboardEvent| {
            let in_line = line_at.get_untracked().is_some_and(|(_, at)| at > 0);
            let forward = match evt.key().as_str() {
                "ArrowDown" => true,
                "ArrowUp" => false,
                "ArrowRight" if in_line => true,
                "ArrowLeft" if in_line => false,
                _ => return,
            };
            let modified = evt.alt_key() || evt.ctrl_key() || evt.meta_key() || evt.shift_key();
            // The line marks the body as having its own arrows, so the shared check would
            // refuse every key while it is open.
            if modified || (!in_line && should_ignore_history_key_event(&evt)) {
                return;
            }
            evt.prevent_default();
            if in_line && matches!(evt.key().as_str(), "ArrowLeft" | "ArrowRight") {
                step_any(forward);
            } else {
                leave_line();
                step(forward);
            }
        },
    );

    let counts = [result.marks(Some(true)), result.marks(Some(false))];
    let losses = [result.average_loss(true), result.average_loss(false)];
    let curve = result.white_curve();
    let side = {
        let names = names.clone();
        move |white: bool| {
            let name = if white {
                names.0.clone()
            } else {
                names.1.clone()
            };
            let side_index = usize::from(!white);
            let counts = counts[side_index];
            let loss = losses[side_index];
            let chips = [Grade::Blunder, Grade::Mistake, Grade::Inaccuracy]
                .into_iter()
                .map(|grade| {
                    let count = counts[severity(grade) as usize];
                    let label = format!("{count} {}", grade_plural(grade, count));
                    let active = move || {
                        filter.with(|f| f.white == Some(white) && f.worst_from == grade)
                    };
                    view! {
                        <button
                            type="button"
                            class="inline-flex gap-1 items-center py-1 px-1.5 rounded transition-colors lg:py-0.5 disabled:opacity-40 dark:hover:bg-white/10 hover:bg-black/10"
                            class=("ring-1", active)
                            class=("ring-pillbug-teal", active)
                            title=format!(
                                "Show only {}'s {}{}",
                                if white { "White" } else { "Black" },
                                grade_plural(grade, 2),
                                if grade == Grade::Blunder { "" } else { " and worse" },
                            )
                            aria-pressed=move || active().to_string()
                            disabled=count == 0
                            on:click=move |_| {
                                filter
                                    .update(|f| {
                                        *f = if f.white == Some(white) && f.worst_from == grade {
                                            Filter::default()
                                        } else {
                                            Filter {
                                                white: Some(white),
                                                worst_from: grade,
                                            }
                                        };
                                    })
                            }
                        >
                            <span class=format!(
                                "inline-block w-2 h-2 rounded-full {}",
                                grade_fill_class(grade),
                            )></span>
                            <span class="tabular-nums">{count}</span>
                            <GradeMark grade />
                            <span class="sr-only">{label}</span>
                        </button>
                    }
                })
                .collect_view();
            view! {
                <div class="flex flex-col gap-0.5 text-sm">
                    <span class="flex gap-1.5 items-center min-w-0 font-semibold">
                        <SideHex white />
                        <span class="truncate">{name}</span>
                    </span>
                    <span class="flex flex-wrap gap-1 items-center text-xs">
                        {chips}
                        {loss
                            .map(|loss| {
                                view! {
                                    <span
                                        class="ml-1 tabular-nums whitespace-nowrap cursor-help"
                                        title="Win chance this player gave away per move, on average. Lower is better."
                                    >
                                        {format!("avg. loss {loss:.1}%")}
                                    </span>
                                }
                            })}
                    </span>
                </div>
            }
        }
    };
    let overview = view! {
        <div class="flex flex-col gap-1.5 pb-2 border-b shrink-0 border-black/10 dark:border-white/10">
            {side(true)} {side(false)} <p class="text-xs text-gray-600 dark:text-gray-300">
                <GradeMark grade=Grade::Inaccuracy />
                " inaccuracy: lost 5–10% win chance · "
                <GradeMark grade=Grade::Mistake />
                " mistake: 10–15% · "
                <GradeMark grade=Grade::Blunder />
                " blunder: 15% or more · avg. loss: win chance given away per move"
            </p>
        </div>
    };

    let graph = view! {
        <WinGraph
            curve=curve
            marks=graded.iter().map(|(ply, _, grade)| (*ply, *grade)).collect()
            selected=selected_ply
            on_pick=Callback::new(move |ply| {
                pick(ply);
                bring_board_into_view();
            })
        />
    };

    let filters = move || {
        let names = names.clone();
        let side_button = move |white: Option<bool>, label: String| {
            let active = move || filter.with(|f| f.white == white);
            view! {
                <button
                    type="button"
                    class="py-1 px-2 rounded transition-colors lg:py-0.5"
                    class=("ui-segmented-active", active)
                    class=("hover:bg-black/10", move || !active())
                    class=("dark:hover:bg-white/10", move || !active())
                    aria-pressed=move || active().to_string()
                    on:click=move |_| filter.update(|f| f.white = white)
                >
                    {label}
                </button>
            }
        };
        let grade_button = move |worst_from: Grade, label: &'static str| {
            let active = move || filter.with(|f| f.worst_from == worst_from);
            view! {
                <button
                    type="button"
                    class="py-1 px-2 rounded transition-colors lg:py-0.5"
                    class=("ui-segmented-active", active)
                    class=("hover:bg-black/10", move || !active())
                    class=("dark:hover:bg-white/10", move || !active())
                    aria-pressed=move || active().to_string()
                    on:click=move |_| filter.update(|f| f.worst_from = worst_from)
                >
                    {label}
                </button>
            }
        };
        view! {
            <div class="flex flex-wrap gap-y-1 gap-x-3 text-xs" role="group" aria-label="Filter">
                <span class="flex gap-0.5">
                    {side_button(None, "Both".to_string())}
                    {side_button(Some(true), names.0.clone())}
                    {side_button(Some(false), names.1.clone())}
                </span>
                <span class="flex gap-0.5">
                    {grade_button(Grade::Inaccuracy, "All")}
                    {grade_button(Grade::Mistake, "? and ??")}
                    {grade_button(Grade::Blunder, "?? only")}
                </span>
                <span class="flex gap-0.5 ml-auto" role="group" aria-label="Show moves as">
                    <button
                        type="button"
                        class="py-1 px-2 rounded transition-colors lg:py-0.5"
                        class=("ui-segmented-active", move || !notation())
                        class=("hover:bg-black/10", notation)
                        aria-pressed=move || (!notation()).to_string()
                        on:click=move |_| set_notation(false)
                    >
                        "Words"
                    </button>
                    <button
                        type="button"
                        class="py-1 px-2 rounded transition-colors lg:py-0.5"
                        class=("ui-segmented-active", notation)
                        class=("hover:bg-black/10", move || !notation())
                        aria-pressed=move || notation().to_string()
                        on:click=move |_| set_notation(true)
                    >
                        "Notation"
                    </button>
                </span>
            </div>
        }
    };

    // The banner always takes the same room, so picking a move never shifts the list.
    let hint = move || {
        view! {
            <p class="text-sm text-gray-700 dark:text-gray-200">
                "Pick a move in the list or on the graph to see it on the board: "
                <span class="font-semibold text-red-700 dark:text-red-400">"✗ red"</span>
                " is what was played, "
                <span class="font-semibold text-green-700 dark:text-green-400">"✓ green"</span>
                " is better."
                <span class="hidden lg:inline">" ↑ and ↓ step through them."</span>
            </p>
        }
    };
    let banner = move || {
        let Some(ply) = selected_ply.get() else {
            return hint().into_any();
        };
        let Some((eval, grade)) = evals
            .move_eval(ply)
            .and_then(|eval| eval.grade().map(|grade| (eval, grade)))
            .filter(|(_, grade)| filter.with(|f| f.shows(ply, *grade)))
        else {
            return hint().into_any();
        };
        let headline = wording(ply)
            .map(|w| w.headline(ply + 1, grade))
            .unwrap_or_else(|| format!("Move {}: {} {}", ply + 1, eval.played, grade.glyph()));
        let line_text = StoredValue::new({
            let words = game_response.with_untracked(|game| {
                game.as_ref()
                    .map(|g| {
                        line_words(
                            g.game_type,
                            g.tournament_queen_rule,
                            &g.history,
                            ply,
                            &eval.line,
                        )
                    })
                    .unwrap_or_default()
            });
            words
                .into_iter()
                .zip(eval.line.iter().cloned())
                .collect::<Vec<_>>()
        });
        let line_len = line_text.with_value(Vec::len);
        let show_step = move |steps: usize| show_line_step(ply, steps);
        let in_line = move || line_at.get().map(|(_, step)| step);
        view! {
            <div class="flex flex-col gap-1.5">
                <p class:hidden=notation>{headline}</p>
                <p class="font-mono" class:hidden=move || !notation()>
                    {format!(
                        "{}. {} {} {}  ·  better {}",
                        ply + 1,
                        if ply % 2 == 0 { "White" } else { "Black" },
                        eval.played,
                        grade.glyph(),
                        eval.best,
                    )}
                </p>
                <Show
                    when=move || in_line().is_some()
                    fallback=move || {
                        view! {
                            <div class="flex flex-wrap gap-1.5 items-center">
                                <button
                                    type="button"
                                    class="ui-button ui-button-sm"
                                    on:click=move |_| step(false)
                                >
                                    "◀ Previous"
                                </button>
                                <button
                                    type="button"
                                    class="ui-button ui-button-sm"
                                    on:click=move |_| step(true)
                                >
                                    "Next ▶"
                                </button>
                                <Show when=move || { line_len > 0 }>
                                    <button
                                        type="button"
                                        class="ui-button ui-button-primary ui-button-sm"
                                        on:click=move |_| show_step(1)
                                    >
                                        {format!(
                                            "Play the engine's line ({line_len} {})",
                                            if line_len == 1 { "move" } else { "moves" },
                                        )}
                                    </button>
                                </Show>
                            </div>
                        }
                    }
                >
                    {move || {
                        let at = in_line().unwrap_or(0);
                        let at_end = at >= line_len;
                        let steps = line_text
                            .with_value(|words| {
                                words
                                    .iter()
                                    .enumerate()
                                    .map(|(i, (text, mv))| {
                                        let n = i + 1;
                                        let (text, mv) = (text.clone(), mv.clone());
                                        view! {
                                            <li>
                                                <button
                                                    type="button"
                                                    class="py-0.5 px-1.5 w-full text-xs text-left rounded transition-colors dark:hover:bg-white/10 hover:bg-black/10"
                                                    class=("ui-segmented-active", n == at)
                                                    aria-current=(n == at).then_some("step")
                                                    on:click=move |_| show_step(n)
                                                >
                                                    {format!("{}. ", ply + n)}
                                                    <span class:hidden=notation>{text}</span>
                                                    <span class="font-mono" class:hidden=move || !notation()>
                                                        {mv}
                                                    </span>
                                                </button>
                                            </li>
                                        }
                                    })
                                    .collect_view()
                            });
                        view! {
                            <div class="flex flex-col gap-1.5">
                                <p class="text-xs text-gray-600 dark:text-gray-300">
                                    {format!(
                                        "How the engine would play on instead of move {}. Step through it on the board ({at} of {line_len} shown):",
                                        ply + 1,
                                    )}
                                </p>
                                <ol class="flex overflow-y-auto flex-col max-h-40">{steps}</ol>
                                <div class="flex flex-wrap gap-1.5 items-center">
                                    <button
                                        type="button"
                                        class="ui-button ui-button-sm"
                                        aria-label="Back one move in the line"
                                        on:click=move |_| show_step(at.saturating_sub(1))
                                    >
                                        "◀"
                                    </button>
                                    <button
                                        type="button"
                                        class="ui-button ui-button-sm"
                                        aria-label="Forward one move in the line"
                                        // Named, not inline: in `disabled=at >= line_len` the view
                                        // macro ends the tag at `>` and prints the rest as text.
                                        disabled=at_end
                                        on:click=move |_| show_step((at + 1).min(line_len))
                                    >
                                        "▶"
                                    </button>
                                    <button
                                        type="button"
                                        class="ui-button ui-button-primary ui-button-sm"
                                        on:click=move |_| {
                                            show_step(0);
                                            line_at.set(None);
                                        }
                                    >
                                        "Back to the game"
                                    </button>
                                </div>
                            </div>
                        }
                    }}
                </Show>
            </div>
        }
        .into_any()
    };

    let rows = graded
        .into_iter()
        .map(|(ply, eval, grade)| {
            let (played_words, best_words) = match wording(ply) {
                Some(w) => (w.played_short(), w.best_short()),
                None => (eval.played.clone(), eval.best.clone()),
            };
            let selected = move || selected_ply.get() == Some(ply);
            let white = ply % 2 == 0;
            view! {
                <li data-eval-ply=ply class:hidden=move || !filter.with(|f| f.shows(ply, grade))>
                    <button
                        type="button"
                        class="flex flex-col gap-0.5 py-1.5 px-2 w-full text-left rounded transition-colors dark:hover:bg-pillbug-teal/15 hover:bg-blue-light/70"
                        class=("bg-orange-twilight/40", selected)
                        aria-current=move || selected().then_some("true")
                        on:click=move |_| {
                            pick(ply);
                            bring_board_into_view();
                        }
                    >
                        <span class="flex flex-wrap gap-x-1.5 items-baseline text-sm">
                            <span class="self-center">
                                <SideHex white />
                            </span>
                            <span class="font-semibold">
                                {format!("{}. ", ply + 1)}
                                <span class:hidden=notation>{played_words}</span>
                                <span class="font-mono" class:hidden=move || !notation()>
                                    {eval.played.clone()}
                                </span> " " <GradeMark grade />
                            </span>
                        </span>
                        <span class="text-sm">
                            "Better: "
                            <span
                                class="font-semibold text-green-700 dark:text-green-400"
                                class:hidden=notation
                            >
                                {best_words}
                            </span>
                            <span
                                class="font-mono font-semibold text-green-700 dark:text-green-400"
                                class:hidden=move || !notation()
                            >
                                {eval.best.clone()}
                            </span>
                        </span>
                    </button>
                </li>
            }
        })
        .collect_view();
    let nothing_shown = move || !nothing_graded && filter.with(|_| shown_plies().is_empty());

    view! {
        {overview}
        {graph}
        <Show
            when=move || !nothing_graded
            fallback=|| {
                view! {
                    <p class="text-sm">"The engine found no inaccuracies, mistakes or blunders."</p>
                }
            }
        >
            {filters.clone()()}
            <div
                class="p-2 text-sm rounded border shrink-0 min-h-[9.5rem] border-orange-twilight bg-orange-twilight/20"
                aria-live="polite"
            >
                {banner}
            </div>
        </Show>
        <ul class="flex overflow-y-auto flex-col flex-1 min-h-0" aria-label="Marked moves">
            {rows}
            <Show when=nothing_shown>
                <li class="p-2 text-sm text-gray-600 dark:text-gray-300">
                    "No moves match these filters."
                </li>
            </Show>
        </ul>
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

    #[test]
    fn filters_pick_a_side_and_a_worst_grade() {
        let all = Filter::default();
        assert!(all.shows(3, Grade::Inaccuracy));
        let white_mistakes = Filter {
            white: Some(true),
            worst_from: Grade::Mistake,
        };
        assert!(white_mistakes.shows(2, Grade::Blunder));
        assert!(white_mistakes.shows(2, Grade::Mistake));
        assert!(!white_mistakes.shows(2, Grade::Inaccuracy));
        assert!(
            !white_mistakes.shows(3, Grade::Blunder),
            "an odd ply is Black's"
        );
    }

    #[test]
    fn the_curve_spans_the_box_with_white_ahead_at_the_top() {
        let points = curve_points(&[50.0, 100.0, 0.0], 300.0, 64.0);
        assert_eq!(points, vec![(0.0, 32.0), (150.0, 0.0), (300.0, 64.0)]);
    }

    #[test]
    fn a_click_on_the_graph_picks_the_nearest_marked_move() {
        // Ten moves: move ply 4 is drawn at step 5, half way along.
        assert_eq!(nearest_mark(&[1, 4, 8], 10, 0.5), Some(4));
        assert_eq!(nearest_mark(&[1, 4, 8], 10, 0.95), Some(8));
        assert_eq!(nearest_mark(&[1, 4, 8], 10, 0.0), Some(1));
        assert_eq!(nearest_mark(&[], 10, 0.5), None);
    }
}
