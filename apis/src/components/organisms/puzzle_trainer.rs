use crate::{
    common::{CurrentConfirm, MoveConfirm},
    components::{
        layouts::base_layout::OrientationSignal,
        molecules::{
            piece_glows::{GlowColor, PieceGlows},
            puzzle_nav::PuzzleNav,
            puzzle_vote::PuzzleVote,
        },
        organisms::{
            board::Board,
            reserve::{Alignment, Reserve},
        },
    },
    functions::puzzles::{get_puzzle, my_puzzle_rating, puzzle_sets},
    hiveground::selected_history_board,
    hooks::puzzle_keys::use_puzzle_keys,
    providers::{
        game_state::{GameStateStore, GameStateStoreFields},
        puzzle::{
            is_signed_in,
            puzzle_hiveground_interaction,
            DailyStatus,
            PuzzleContext,
            PuzzlePhase,
        },
        Config,
    },
    responses::{PuzzleResponse, PuzzleVoteResponse},
};
use hive_lib::{Bug, Color, Piece};
use leptos::{prelude::*, task::spawn_local};
use shared_types::{GameSpeed, PuzzleDifficulty, PuzzleGameTypeFilter, PuzzleLength};

#[derive(Clone, Copy)]
struct PuzzleNext(Callback<()>);

/// The puzzle board with its side panel. With a `nanoid` it opens that puzzle; without one it
/// serves puzzles near the player's rating, one after another.
#[component]
pub fn PuzzleTrainer(
    #[prop(into)] nanoid: Signal<Option<String>>,
    #[prop(optional)] daily: Option<DailyStatus>,
) -> impl IntoView {
    let query = leptos_router::hooks::use_query_map();
    let game_state = GameStateStore::new();
    provide_context(game_state);
    let config = expect_context::<Config>().0;
    let puzzle = PuzzleContext::new(game_state);
    puzzle.daily.set(daily);
    provide_context(puzzle);
    let queens = Signal::derive(move || {
        puzzle.puzzle.track();
        let solver = puzzle.solver();
        game_state.state().with(|state| {
            [
                (solver.opposite_color(), GlowColor::Red),
                (solver, GlowColor::Green),
            ]
            .into_iter()
            .filter_map(|(color, glow)| {
                state
                    .board
                    .position_of_piece(Piece::new_from(Bug::Queen, color, 0))
                    .map(|at| (at, glow))
            })
            .collect::<Vec<_>>()
        })
    });
    provide_context(PieceGlows(queens));
    let confirm = Memo::new(move |_| {
        if puzzle.one_click.get() {
            return MoveConfirm::Single;
        }
        config.with(|config| {
            let mode = config
                .confirm_mode
                .get(&GameSpeed::Puzzle)
                .or_else(|| config.confirm_mode.get(&GameSpeed::Untimed))
                .cloned()
                .unwrap_or_default();
            // The clock confirm is a real-time idea; puzzles have no clock.
            if mode == MoveConfirm::Clock {
                MoveConfirm::Double
            } else {
                mode
            }
        })
    });
    provide_context(CurrentConfirm(confirm));
    let interaction = puzzle_hiveground_interaction(puzzle);
    // A shared puzzle link opens one puzzle; "next" then continues with rated training.
    let pathname = leptos_router::hooks::use_location().pathname;
    let navigate = leptos_router::hooks::use_navigate();
    let next = Callback::new(move |()| {
        if pathname.get_untracked() != "/puzzles" && !puzzle.training.get_value() {
            let target = if untrack(|| daily_result(puzzle)) == Some(true) {
                "/puzzles?after=daily".to_string()
            } else {
                format!("/puzzles{}", puzzle.focus_query())
            };
            navigate(&target, Default::default());
        } else {
            puzzle.load_next();
        }
    });
    provide_context(PuzzleNext(next));
    let shared = RwSignal::new(ShareState::Idle);
    provide_context(SharedLink(shared));
    Effect::watch(
        move || {
            puzzle
                .puzzle
                .with(|current| current.as_ref().map(|p| p.nanoid.clone()))
        },
        move |_, _, _| shared.set(ShareState::Idle),
        false,
    );
    let annotations = crate::providers::annotations::AnnotationsSignal::play(game_state);
    provide_context(annotations);
    crate::hooks::puzzle_marks::use_puzzle_marks(puzzle, annotations);
    use_puzzle_keys(puzzle, next);
    let history_board = selected_history_board(game_state);
    let vertical = expect_context::<OrientationSignal>().orientation_vertical;
    let rating = RwSignal::new(None::<i32>);
    Effect::watch(
        move || {
            puzzle.puzzle.with(|current| {
                current
                    .as_ref()
                    .map(|p| (p.nanoid.clone(), p.rating, p.attempted))
            })
        },
        move |current, _, _| {
            let Some((_, difficulty, attempted)) = current else {
                return;
            };
            let Some(mine) = rating.get_untracked() else {
                return;
            };
            let wanted = puzzle.difficulty.get_untracked();
            let target = f64::from(mine) + wanted.rating_offset();
            if !attempted
                && wanted != PuzzleDifficulty::Normal
                && (f64::from(*difficulty) - target).abs() > 400.0
            {
                puzzle.pending_notice.set_value(Some(
                    "No closer match left; this is the nearest.".to_string(),
                ));
            }
        },
        false,
    );
    let signed_in = is_signed_in();

    Effect::new(move |_| {
        if signed_in.get() {
            spawn_local(async move {
                if let Ok(Some(response)) = my_puzzle_rating().await {
                    let _ = rating.try_set(Some(response.rating));
                }
            });
        }
    });
    Effect::new(move |_| {
        if let Some(after) = puzzle
            .attempt
            .get()
            .and_then(|attempt| attempt.rating_after)
        {
            rating.set(Some(after));
        }
    });

    Effect::new(move |_| {
        puzzle.restore_preferences();
        match nanoid.get() {
            Some(nanoid) => {
                let (theme, collection) = query.with(|query| {
                    (
                        query.get("theme").filter(|theme| !theme.is_empty()),
                        query
                            .get("collection")
                            .filter(|collection| !collection.is_empty()),
                    )
                });
                puzzle.set_focus(theme, collection);
                puzzle.training.set_value(false);
                puzzle.phase.set(PuzzlePhase::Loading);
                spawn_local(async move {
                    match get_puzzle(nanoid).await {
                        Ok(found) => puzzle.load(found),
                        Err(_) => {
                            let _ = puzzle.phase.try_set(PuzzlePhase::Empty);
                        }
                    }
                });
            }
            None => {
                let (theme, collection) = query.with(|query| {
                    (
                        query.get("theme").filter(|theme| !theme.is_empty()),
                        query
                            .get("collection")
                            .filter(|collection| !collection.is_empty()),
                    )
                });
                let length = query.with(|query| match query.get("length").as_deref() {
                    Some("1") => Some(PuzzleLength::One),
                    Some("2") => Some(PuzzleLength::Two),
                    Some("3") => Some(PuzzleLength::ThreePlus),
                    _ => None,
                });
                if let Some(length) = length {
                    puzzle.set_length(length);
                }
                if query.with(|query| query.get("after").as_deref() == Some("daily")) {
                    puzzle
                        .one_off_difficulty
                        .set_value(Some(PuzzleDifficulty::Harder));
                    puzzle.pending_notice.set_value(Some(
                        "You solved today's puzzle, so here is a harder one.".to_string(),
                    ));
                }
                puzzle.training.set_value(true);
                puzzle.set_focus(theme, collection);
                puzzle.load_next();
            }
        }
    });

    let solver = Memo::new(move |_| {
        puzzle.puzzle.track();
        puzzle.solver()
    });
    let opponent = Signal::derive(move || solver.get().opposite_color());
    let solver_signal = Signal::derive(move || solver.get());
    let has_puzzle = move || puzzle.puzzle.with(Option::is_some);

    view! {
        <div class=move || {
            if vertical() {
                "flex flex-col ui-board-page-surface"
            } else {
                "grid min-h-[100dvh] max-h-[100dvh] grid-cols-[minmax(0,1fr)_clamp(320px,26vw,440px)] grid-rows-1 pr-1 ui-board-page-surface standalone:min-h-[var(--app-height)] standalone:max-h-[var(--app-height)]"
            }
        }>
            <Show
                when=vertical
                fallback=move || {
                    view! {
                        <div class="grid relative grid-cols-8 col-start-1 grid-rows-6 row-start-1 min-w-0 min-h-0">
                            <PuzzleBoard interaction history_board />
                            <PhaseBadge results_only=true />
                        </div>
                        <div class="flex flex-col col-start-2 row-start-1 gap-2 pt-10 my-1 mr-1 min-h-0">
                            <div class="flex overflow-y-auto flex-col flex-1 gap-3 p-3 min-h-0 text-sm select-none ui-board-side-panel">
                                <PuzzleNav />
                                <PuzzlePrompt />
                                <PuzzleControls />
                                <Show when=has_puzzle>
                                    <div class="flex flex-col gap-1 py-1 px-2 rounded border border-black/5 bg-odd-light/70 dark:border-white/10 dark:bg-surface-muted">
                                        <HandRow color=opponent interaction history_board />
                                        <HandRow color=solver_signal interaction history_board />
                                    </div>
                                </Show>
                                <PuzzleAfter />
                                <PuzzleFooter rating nav_elsewhere=true />
                            </div>
                        </div>
                    }
                }
            >
                <div class="flex flex-col min-h-0 h-[calc(100svh-2.5rem)]">
                    <div class="flex flex-col shrink-0 ui-board-side-panel">
                        <div class="flex flex-col gap-1 px-3 pt-2 pb-1">
                            <PuzzlePrompt compact=true />
                            <PuzzleControls compact=true />
                        </div>
                        <TopCardTray opponent interaction history_board />
                    </div>
                    <div class="flex relative min-h-0 grow">
                        <PuzzleBoard interaction history_board />
                    </div>
                    <BottomBar solver=solver_signal interaction history_board />
                </div>
                <div class="flex flex-col gap-3 p-3 ui-board-side-panel">
                    <PuzzleAfter />
                    <PuzzleFooter rating />
                </div>
            </Show>
        </div>
    }
}

const PUZZLE_RESERVE_VIEWBOX: &str = "-42 -38 438 76";

fn color_name(color: Color) -> &'static str {
    match color {
        Color::White => "White",
        Color::Black => "Black",
    }
}

pub fn moves_label(moves: i32) -> String {
    if moves == 1 {
        "1 move".to_string()
    } else {
        format!("{moves} moves")
    }
}

/// The board, rebuilt for every puzzle so it frames each new position afresh.
#[component]
fn PuzzleBoard(
    interaction: crate::hiveground::HivegroundInteraction,
    history_board: Memo<hive_lib::Board>,
) -> impl IntoView {
    let puzzle = expect_context::<PuzzleContext>();
    let key = Memo::new(move |_| {
        puzzle.puzzle.with(|current| {
            current
                .as_ref()
                .map(|p| p.nanoid.clone())
                .unwrap_or_default()
        })
    });
    view! {
        <Show
            when=move || puzzle.phase.get() != PuzzlePhase::Empty
            fallback=|| {
                view! {
                    <div class="flex justify-center items-center p-6 w-full h-full">
                        <EmptyMessage />
                    </div>
                }
            }
        >
            <For
                each=move || std::iter::once(key.get())
                key=|key| key.clone()
                children=move |_| view! { <Board interaction history_board fit_hive=true /> }
            />
        </Show>
    }
}

fn next_available(puzzle: PuzzleContext) -> bool {
    let phase = puzzle.phase.get();
    let in_set = puzzle.theme.with(Option::is_some) || puzzle.collection.with(Option::is_some);
    phase.is_over()
        || (phase == PuzzlePhase::Empty
            && puzzle.game_type.get() == PuzzleGameTypeFilter::Any
            && !in_set)
}

/// On phones, once the puzzle is over the player's hand gives way to the buttons that move on,
/// in the same thumb-reach strip and at the same height, so nothing shifts.
#[component]
fn BottomBar(
    solver: Signal<Color>,
    interaction: crate::hiveground::HivegroundInteraction,
    history_board: Memo<hive_lib::Board>,
) -> impl IntoView {
    let puzzle = expect_context::<PuzzleContext>();
    let next = expect_context::<PuzzleNext>().0;
    view! {
        <Show
            when=move || next_available(puzzle)
            fallback=move || {
                view! { <MobileHand color=solver framed=true interaction history_board /> }
            }
        >
            <div class="flex gap-2 items-center px-3 w-full h-20 shrink-0 ui-board-reserve">
                <Show when=move || puzzle.phase.get() == PuzzlePhase::Failed>
                    <button
                        type="button"
                        class="h-12 text-base ui-button ui-button-secondary grow basis-0"
                        on:click=move |_| puzzle.retry()
                    >
                        "Retry"
                    </button>
                </Show>
                <button
                    type="button"
                    class="h-12 text-base font-bold text-white shadow ui-button bg-pillbug-teal grow basis-0"
                    on:click=move |_| next.run(())
                >
                    {move || after_daily_label(puzzle).unwrap_or("Next puzzle →")}
                </button>
                <TrainerVote large=true />
            </div>
        </Show>
    }
}

#[component]
fn TopCardTray(
    opponent: Signal<Color>,
    interaction: crate::hiveground::HivegroundInteraction,
    history_board: Memo<hive_lib::Board>,
) -> impl IntoView {
    let puzzle = expect_context::<PuzzleContext>();
    let signed_in = is_signed_in();
    let guest_done = move || !signed_in.get() && puzzle.phase.get().is_over();
    let remember_return = use_remember_return();
    view! {
        <Show
            when=guest_done
            fallback=move || {
                view! { <MobileHand color=opponent interaction history_board /> }
            }
        >
            <a
                href="/login"
                class="flex items-center px-3 h-14 text-sm font-semibold border-t shrink-0 border-black/10 ui-text-link dark:border-white/10"
                on:click=move |_| remember_return()
            >
                "Sign in to get a puzzle rating and vote on puzzles →"
            </a>
        </Show>
    }
}

/// 👍 / 👎 for the current puzzle, offered once it is over.
#[component]
fn TrainerVote(#[prop(optional)] large: bool) -> impl IntoView {
    let puzzle = expect_context::<PuzzleContext>();
    let signed_in = is_signed_in();
    let my_vote = RwSignal::new(None::<bool>);
    Effect::new(move |_| {
        my_vote.set(
            puzzle
                .puzzle
                .with(|current| current.as_ref().and_then(|p| p.my_vote)),
        );
    });
    let nanoid = Signal::derive(move || {
        puzzle.puzzle.with(|current| {
            current
                .as_ref()
                .map(|p| p.nanoid.clone())
                .unwrap_or_default()
        })
    });
    let on_voted = Callback::new(move |response: PuzzleVoteResponse| {
        puzzle.puzzle.update(|current| {
            if let Some(current) = current {
                current.my_vote = response.my_vote;
                current.likes = response.likes;
                current.dislikes = response.dislikes;
            }
        });
    });
    view! {
        <Show when=move || signed_in.get() && puzzle.phase.get().is_over()>
            <PuzzleVote nanoid my_vote on_voted large />
        </Show>
    }
}

#[component]
fn MobileHand(
    color: Signal<Color>,
    #[prop(optional)] framed: bool,
    interaction: crate::hiveground::HivegroundInteraction,
    history_board: Memo<hive_lib::Board>,
) -> impl IntoView {
    let puzzle = expect_context::<PuzzleContext>();
    let state = expect_context::<GameStateStore>().state();
    let in_hand = move || {
        if puzzle.puzzle.with(Option::is_none) {
            return false;
        }
        let color = color.get();
        state.with(|state| !state.board.reserve(color, state.game_type).is_empty())
    };
    let empty_label = if framed {
        "No pieces left in your hand"
    } else {
        "No pieces left in their hand"
    };
    let has_puzzle = move || puzzle.puzzle.with(Option::is_some);
    view! {
        <div class=if framed {
            "flex items-center justify-between px-2 w-full h-20 max-h-20 shrink-0 ui-board-reserve"
        } else {
            "flex items-center justify-between w-full h-14 max-h-14 shrink-0 border-t border-black/10 dark:border-white/10"
        }>
            <Show
                when=in_hand
                fallback=move || {
                    view! {
                        <Show when=has_puzzle>
                            <span class="px-3 text-sm text-gray-500 dark:text-gray-400">
                                {empty_label}
                            </span>
                        </Show>
                    }
                }
            >
                <Reserve
                    alignment=Alignment::SingleRow
                    color
                    viewbox_str=PUZZLE_RESERVE_VIEWBOX
                    interaction
                    history_board
                />
            </Show>
        </div>
    }
}

#[component]
fn HandRow(
    color: Signal<Color>,
    interaction: crate::hiveground::HivegroundInteraction,
    history_board: Memo<hive_lib::Board>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let puzzle = expect_context::<PuzzleContext>();
    let state = game_state.state();
    let hand_viewbox = Memo::new(move |_| {
        let kinds = state.with(|state| {
            [Color::White, Color::Black]
                .into_iter()
                .map(|color| state.board.reserve(color, state.game_type).len())
                .max()
                .unwrap_or(0)
        });
        match kinds {
            0..=3 => "-42 -38 230 76",
            4..=5 => "-42 -38 330 76",
            _ => "-42 -38 438 76",
        }
    });
    let in_hand = move || {
        let color = color.get();
        state.with(|state| !state.board.reserve(color, state.game_type).is_empty())
    };
    let label = move || {
        let color = color.get();
        let whose = if color == puzzle.solver() {
            "Your pieces in hand"
        } else {
            "Opponent's pieces in hand"
        };
        format!("{whose} ({})", color_name(color))
    };
    view! {
        <div class="flex flex-col">
            <span class="font-semibold tracking-wide text-gray-600 uppercase dark:text-gray-400 text-[11px]">
                {label}
            </span>
            <Show
                when=in_hand
                fallback=|| {
                    view! { <span class="py-1 text-xs text-gray-500">"None left"</span> }
                }
            >
                <div class="flex h-24">
                    {move || {
                        let viewbox_str = hand_viewbox.get();
                        view! {
                            <Reserve
                                alignment=Alignment::SingleRow
                                color
                                viewbox_str
                                interaction
                                history_board
                            />
                        }
                    }}
                </div>
            </Show>
        </div>
    }
}

/// A short, always-visible status over the board, so the result reads without looking away.
#[component]
fn PhaseBadge(#[prop(optional)] results_only: bool) -> impl IntoView {
    let puzzle = expect_context::<PuzzleContext>();
    let confirm = expect_context::<CurrentConfirm>().0;
    let awaiting_confirm = move || {
        confirm.get() == MoveConfirm::Double
            && puzzle
                .game_state
                .move_info()
                .with(|info| info.target_position.is_some() && info.active.is_some())
    };
    let text = move || match puzzle.phase.get() {
        PuzzlePhase::Solving if results_only => None,
        PuzzlePhase::Solving if puzzle.notice.with(Option::is_some) => Some((
            puzzle.notice.get().unwrap_or_default(),
            "bg-orange-twilight text-white",
        )),
        PuzzlePhase::Solving if awaiting_confirm() => Some((
            "Select the same spot again to play".to_string(),
            "bg-pillbug-teal text-white",
        )),
        PuzzlePhase::Loading => Some(("Loading…".to_string(), "bg-gray-700/80 text-white")),
        PuzzlePhase::Setup => Some((
            "Opponent's last move".to_string(),
            "bg-gray-700/80 text-white",
        )),
        PuzzlePhase::Replying => {
            Some(("✓ Correct!".to_string(), "bg-grasshopper-green text-white"))
        }
        PuzzlePhase::Solved if puzzle.replaying.get() => Some((
            "✓ Solved (practice)".to_string(),
            "bg-grasshopper-green text-white",
        )),
        PuzzlePhase::Solved if puzzle.hinted.get() => Some((
            "✓ Solved with a hint".to_string(),
            "bg-orange-twilight text-white",
        )),
        PuzzlePhase::Solved => Some(("✓ Solved!".to_string(), "bg-grasshopper-green text-white")),
        PuzzlePhase::Failed if puzzle.played_out.get() > 0 => None,
        PuzzlePhase::Failed => Some(("✗ Not the move".to_string(), "bg-ladybug-red text-white")),
        PuzzlePhase::Revealed => Some(("Solution".to_string(), "bg-pillbug-teal text-white")),
        PuzzlePhase::Empty => Some((
            if puzzle.game_type.get() == PuzzleGameTypeFilter::Any {
                "Puzzle not found".to_string()
            } else {
                "No puzzles left in this set".to_string()
            },
            "bg-gray-700/80 text-white",
        )),
        PuzzlePhase::Solving => None,
    };
    view! {
        <ShowLet some=text let:badge>
            <div
                role="status"
                aria-live="polite"
                class=format!(
                    "absolute bottom-4 left-1/2 z-20 px-5 py-2 max-w-[90%] text-center text-lg font-bold rounded-2xl shadow-lg -translate-x-1/2 pointer-events-none {}",
                    badge.1,
                )
            >
                {badge.0}
            </div>
        </ShowLet>
    }
}

/// The set being trained, with a way back to mixed puzzles.
#[component]
fn FocusChip() -> impl IntoView {
    let puzzle = expect_context::<PuzzleContext>();
    let set_key = move || {
        puzzle
            .collection
            .get()
            .map(|collection| format!("collection:{collection}"))
            .or_else(|| puzzle.theme.get())
    };
    let progress = Resource::new(
        move || {
            (
                set_key(),
                puzzle.puzzle.with(|p| p.as_ref().map(|p| p.nanoid.clone())),
            )
        },
        |(key, _)| async move {
            let key = key?;
            let sets = puzzle_sets().await.ok()?;
            sets.into_iter()
                .find(|set| set.key == key)
                .map(|set| (set.solved, set.total))
        },
    );
    let signed_in = is_signed_in();
    let focus = move || {
        puzzle
            .collection
            .get()
            .map(|collection| collection_label(&collection))
            .or_else(|| puzzle.theme.get().map(|theme| theme_label(&theme)))
    };
    let counter = move || {
        let (solved, total) = progress.get().flatten()?;
        Some(if signed_in.get() {
            format!(" · {solved}/{total} solved")
        } else {
            format!(" · {total} puzzles")
        })
    };
    view! {
        <ShowLet some=focus let:focus>
            <span class="inline-flex items-center self-start pl-3 text-xs font-semibold rounded-full bg-pillbug-teal/15">
                {format!("Training: {focus}")} <Transition>{counter}</Transition>
                <button
                    type="button"
                    class="grid place-items-center -my-3 w-11 h-11 rounded-full hover:text-ladybug-red"
                    title="Leave this set; later puzzles are mixed again"
                    aria-label="Leave this set"
                    on:click=move |_| puzzle.leave_focus()
                >
                    "✕"
                </button>
            </span>
        </ShowLet>
    }
}

#[component]
fn PuzzlePrompt(#[prop(optional)] compact: bool) -> impl IntoView {
    let puzzle = expect_context::<PuzzleContext>();
    let signed_in = is_signed_in();
    let rating_note = move || {
        puzzle
            .attempt
            .get()
            .and_then(|attempt| Some((attempt.rating_change?, attempt.rating_after?)))
            .map(|(change, after)| format!("{},\u{a0}now\u{a0}{after}", signed(change)))
            .unwrap_or_default()
    };
    let confirm = expect_context::<CurrentConfirm>().0;
    let details = move || {
        puzzle.puzzle.with(|current| {
            current
                .as_ref()
                .map(|p| (p.solver_moves, p.rating, p.title.clone(), p.attempted))
        })
    };
    let solver = move || {
        puzzle.puzzle.track();
        puzzle.solver()
    };
    let swatch = move || match solver() {
        Color::White => "bg-hive-white border border-black/30",
        Color::Black => "bg-hive-black border border-white/30",
    };
    let move_number = move || {
        let total = details().map(|d| d.0).unwrap_or(0);
        let done = puzzle.progress.with(|progress| progress.index / 2) as i32;
        (done.min(total - 1).max(0) + 1, total)
    };
    let awaiting_confirm = move || {
        confirm.get() == MoveConfirm::Double
            && puzzle
                .game_state
                .move_info()
                .with(|info| info.target_position.is_some() && info.active.is_some())
    };
    // The task stays put while playing; transient status (hints, notices, results) gets its own
    // line underneath, so the opponent's move and the goal never vanish mid-thought.
    let daily_summary = move || daily_summary(puzzle);
    let goal = move || match puzzle.phase.get() {
        PuzzlePhase::Loading => "Finding a puzzle…".to_string(),
        PuzzlePhase::Empty => String::new(),
        PuzzlePhase::Setup | PuzzlePhase::Replying => "Your opponent is moving…".to_string(),
        phase if phase.is_over() && daily_summary().is_some() => {
            daily_summary().unwrap_or_default()
        }
        PuzzlePhase::Revealed | PuzzlePhase::Solved => String::new(),
        _ => {
            let (number, total) = move_number();
            let task = if total > 1 && puzzle.phase.get() == PuzzlePhase::Solving {
                format!("Move {number} of {total}.")
            } else {
                String::new()
            };
            match puzzle.opponent_move.get() {
                Some(played) => format!("{played} {task}"),
                None => task,
            }
        }
    };
    let tried_before =
        move || details().is_some_and(|details| details.3) && !puzzle.replaying.get();
    let with_note = move |text: &str| {
        let note = rating_note();
        let stop = if text.ends_with('!') { "" } else { "." };
        if note.is_empty() && tried_before() {
            format!("{text}{stop} Not rated: you tried this puzzle before.")
        } else if compact || note.is_empty() {
            format!("{text}{stop}")
        } else {
            format!("{text}{stop} Rating {note}.")
        }
    };
    let message = move || match puzzle.phase.get() {
        PuzzlePhase::Solving => {
            if let Some(notice) = puzzle.notice.get() {
                notice
            } else if puzzle.reveal_armed.get() {
                "Press again to confirm (counts as a miss).".to_string()
            } else if awaiting_confirm() {
                let prompt = if compact {
                    "Tap again to play."
                } else {
                    "Click again to play (Esc cancels)."
                };
                match puzzle.hint_text.get() {
                    Some(hint) => format!("{hint} {prompt}"),
                    None => prompt.to_string(),
                }
            } else if let Some(hint) = puzzle.hint_text.get() {
                hint
            } else if let Some(summary) = daily_summary() {
                summary
            } else if puzzle.solver_to_move() {
                if puzzle.progress.with(|progress| progress.index) > 0 {
                    "✓ Good so far. Your move.".to_string()
                } else if tried_before() {
                    "Not rated: you tried this puzzle before. Your move.".to_string()
                } else {
                    "Your move.".to_string()
                }
            } else {
                String::new()
            }
        }
        PuzzlePhase::Replying => "✓ Correct!".to_string(),
        PuzzlePhase::Solved if puzzle.replaying.get() => {
            if puzzle.solved_before_replay.get() {
                "✓ Solved again.".to_string()
            } else {
                "✓ Solved.".to_string()
            }
        }
        PuzzlePhase::Solved if puzzle.hinted.get() && puzzle.first_result.get() != Some(false) => {
            if signed_in.get() {
                with_note("Solved with a hint")
            } else {
                "Solved with a hint.".to_string()
            }
        }
        PuzzlePhase::Solved if puzzle.first_result.get() == Some(false) => {
            if signed_in.get() {
                with_note("✓ Solved. Your first try counted")
            } else {
                "✓ Solved.".to_string()
            }
        }
        PuzzlePhase::Solved => with_note("✓ Solved!"),
        PuzzlePhase::Failed if puzzle.played_out.get() > 0 => {
            let opponent = color_name(solver().opposite_color());
            let total = details().map(|details| details.0).unwrap_or(1);
            let status = puzzle
                .game_state
                .state()
                .with(|state| state.game_status.clone());
            match status {
                hive_lib::GameStatus::Finished(hive_lib::GameResult::Winner(winner))
                    if winner == solver() =>
                {
                    format!(
                        "You surrounded the Queen, but not within {}.",
                        moves_label(total)
                    )
                }
                hive_lib::GameStatus::Finished(hive_lib::GameResult::Winner(_)) => {
                    format!("{opponent} surrounded your Queen.")
                }
                hive_lib::GameStatus::Finished(_) => {
                    "Both Queens are surrounded: a draw.".to_string()
                }
                _ if puzzle.playing_out.get() => {
                    format!("Playing it out, not rated. {opponent} answers with its best defence.")
                }
                _ => format!("{opponent} is answering…"),
            }
        }
        PuzzlePhase::Failed if puzzle.refuted.get() => {
            let opponent = color_name(solver().opposite_color());
            let lost = puzzle.game_state.state().with(|state| {
                matches!(
                    state.game_status,
                    hive_lib::GameStatus::Finished(hive_lib::GameResult::Winner(winner))
                        if winner != solver()
                )
            });
            let play_on = if puzzle.playing_out.get() {
                " Play on to see why."
            } else {
                ""
            };
            if lost {
                with_note(&format!("✗ Not the move. {opponent} answers and wins"))
            } else {
                let total = details().map(|details| details.0).unwrap_or(1);
                format!(
                    "{}{play_on}",
                    with_note(&format!(
                        "✗ Not the move. After {opponent}'s answer it cannot be won in {} anymore",
                        moves_label(total)
                    ))
                )
            }
        }
        PuzzlePhase::Failed => with_note("✗ Not the move"),
        PuzzlePhase::Revealed => {
            let step = puzzle.reveal_step.get();
            if compact && step > 0 {
                solution_step_label(puzzle, step)
            } else if compact {
                "Solution".to_string()
            } else {
                with_note("Solution")
            }
        }
        _ => String::new(),
    };
    let daily_label = move || {
        puzzle
            .puzzle
            .with(|p| p.as_ref().and_then(|p| p.daily_day))
            .map(|day| format!("Puzzle of the day · {}", day.format("%A %-d %B")))
    };
    let message_class = move || {
        match puzzle.phase.get() {
        PuzzlePhase::Solved
            if (puzzle.first_result.get() == Some(false) || puzzle.hinted.get())
                && !puzzle.replaying.get() =>
        {
            "text-orange-twilight font-semibold rounded px-1 bg-orange-twilight/15"
        }
        PuzzlePhase::Solved => {
            "text-base text-green-700 dark:text-green-400 font-bold rounded px-1.5 py-0.5 bg-grasshopper-green/20"
        }
        PuzzlePhase::Replying => {
            "text-green-700 dark:text-green-400 font-semibold rounded px-1 bg-grasshopper-green/15"
        }
        PuzzlePhase::Failed => {
            "text-red-700 dark:text-red-400 font-semibold rounded px-1 bg-ladybug-red/15"
        }
        PuzzlePhase::Solving if puzzle.notice.with(Option::is_some) => {
            "text-orange-twilight font-semibold"
        }
        PuzzlePhase::Solving if awaiting_confirm() => "text-pillbug-teal font-semibold",
        _ => "text-gray-700 dark:text-gray-300",
    }
    };
    view! {
        <div class="flex flex-col gap-1">
            {move || {
                daily_label()
                    .map(|label| {
                        view! {
                            <span class="text-xs font-semibold tracking-wide uppercase text-pillbug-teal">
                                {label}
                            </span>
                        }
                    })
            }} <ShowLet some=details let:details>
                <div class="flex gap-2 items-center">
                    <span
                        class=move || format!("inline-block w-5 h-5 shrink-0 {}", swatch())
                        title=move || format!("You play {}", color_name(solver()))
                        style="clip-path: polygon(25% 5%, 75% 5%, 100% 50%, 75% 95%, 25% 95%, 0% 50%)"
                        aria-hidden="true"
                    ></span>
                    <h1 class=if compact {
                        "text-base font-bold truncate min-w-0"
                    } else {
                        "text-lg font-bold"
                    }>
                        {move || {
                            format!("{} to win in {}", color_name(solver()), moves_label(details.0))
                        }}
                    </h1>
                    <span class=move || {
                        let change = puzzle.attempt.get().and_then(|attempt| attempt.rating_change);
                        match (compact, change.map(i32::signum)) {
                            (false, _) => "hidden",
                            (true, Some(1)) => {
                                "ml-auto text-sm font-semibold text-green-700 whitespace-nowrap dark:text-green-400"
                            }
                            (true, Some(-1)) => {
                                "ml-auto text-sm font-semibold text-red-700 whitespace-nowrap dark:text-red-400"
                            }
                            (true, _) => {
                                "ml-auto text-xs text-gray-500 whitespace-nowrap dark:text-gray-400"
                            }
                        }
                    }>
                        {move || {
                            let note = rating_note();
                            let practice = details.3 || puzzle.replaying.get();
                            match (note.is_empty(), practice) {
                                (false, _) => note,
                                (true, true) => format!("{} · practice", details.1),
                                (true, false) => format!("Difficulty {}", details.1),
                            }
                        }}
                    </span>
                </div>
                <div class=if compact {
                    "hidden"
                } else {
                    "flex flex-wrap gap-x-3 text-xs text-gray-600 dark:text-gray-400"
                }>
                    <span
                        class=if compact { "hidden" } else { "" }
                        title="How hard this puzzle is, measured from everyone's first tries"
                    >
                        {format!("Difficulty {}", details.1)}
                    </span>
                    {details.2.map(|title| view! { <span>{title}</span> })}
                    <Show when=move || details.3 || puzzle.replaying.get()>
                        <span title="You tried this puzzle before, so this try does not change your rating">
                            "Practice · not rated"
                        </span>
                    </Show>
                </div>
            </ShowLet> <FocusChip />
            <p class=if compact {
                "text-[15px] leading-snug text-gray-800 dark:text-gray-200 min-h-[2.6rem]"
            } else {
                "text-sm text-gray-800 dark:text-gray-200 min-h-[2.5rem]"
            }>{move || or_blank(goal())}</p> <p class="text-sm min-h-[2.5rem]" aria-live="polite">
                <span class=move || {
                    format!("box-decoration-clone {}", message_class())
                }>{move || or_blank(message())}</span>
            </p>
        </div>
    }
}

#[component]
fn EmptyMessage() -> impl IntoView {
    let puzzle = expect_context::<PuzzleContext>();
    let next = expect_context::<PuzzleNext>().0;
    let filtered = move || puzzle.game_type.get() != PuzzleGameTypeFilter::Any;
    let in_set = move || puzzle.theme.get().is_some() || puzzle.collection.get().is_some();
    view! {
        <div class="flex flex-col gap-3 items-center max-w-xs text-center ui-empty-state">
            <p class="text-base font-semibold">
                {move || match puzzle.game_type.get() {
                    _ if in_set() => "Nothing left to play in this set.",
                    PuzzleGameTypeFilter::Base => "No base-game puzzles left.",
                    PuzzleGameTypeFilter::Mlp => "No expansion puzzles left.",
                    PuzzleGameTypeFilter::Any => "This puzzle could not be found.",
                }}
            </p>
            <Show when=in_set>
                <button
                    type="button"
                    class="text-white min-h-11 ui-button bg-pillbug-teal hover:brightness-110"
                    on:click=move |_| {
                        puzzle.leave_focus();
                        puzzle.load_next();
                    }
                >
                    "Mixed puzzles"
                </button>
                <a
                    class="inline-flex items-center text-sm min-h-11 ui-text-link"
                    href="/puzzles/browse"
                >
                    "Pick another set"
                </a>
            </Show>
            <Show when=move || filtered() && !in_set()>
                <button
                    type="button"
                    class="text-white min-h-11 ui-button bg-pillbug-teal hover:brightness-110"
                    on:click=move |_| {
                        puzzle.set_game_type(PuzzleGameTypeFilter::Any);
                        next.run(());
                    }
                >
                    "Switch to all puzzles"
                </button>
            </Show>
            <a
                class="inline-flex items-center text-sm min-h-11 ui-text-link"
                href="/puzzles/history"
            >
                "Replay puzzles from your history"
            </a>
        </div>
    }
}

#[component]
fn PuzzleControls(#[prop(optional)] compact: bool) -> impl IntoView {
    let puzzle = expect_context::<PuzzleContext>();
    let can_go_next = move || next_available(puzzle);
    let at_last_step = move || puzzle.reveal_step.get() >= puzzle.solution_steps();
    let analysis = move || {
        puzzle
            .puzzle
            .with(|current| current.as_ref().and_then(analysis_link))
    };
    let over = move || puzzle.phase.get().is_over();
    let signed_in = is_signed_in();
    let phase = puzzle.phase;
    // Buttons stay in place (disabled) while the opponent answers, so nothing jumps.
    let in_progress = move || {
        matches!(
            phase.get(),
            PuzzlePhase::Solving
                | PuzzlePhase::Replying
                | PuzzlePhase::Setup
                | PuzzlePhase::Loading
        )
    };
    let solving = move || phase.get() == PuzzlePhase::Solving;
    let next = expect_context::<PuzzleNext>().0;
    let confirm_reveal = puzzle.reveal_armed;
    Effect::watch(
        move || phase.get(),
        move |_, _, _| confirm_reveal.set(false),
        false,
    );
    // Hints and the solution only cost something on a rated first try.
    let costs_rating = move || puzzle.costs_rating(signed_in.get());
    let reveal = move |_| puzzle.request_reveal(costs_rating());
    let hint_disabled = move || !solving() || puzzle.hint_level.get() >= 3;
    let hint_label = move || match puzzle.hint_level.get() {
        0 => "Hint",
        1 => "More help",
        2 => "Show where",
        _ => "Hint shown",
    };
    view! {
        <div class="flex flex-col gap-1">
            <div class=if compact {
                "flex gap-2 [&_.ui-button]:min-h-11 [&_.ui-button]:grow [&_.ui-button]:basis-0 [&_.ui-button]:px-1 [&_.ui-button]:border [&_.ui-button]:border-black/15 dark:[&_.ui-button]:border-white/20 [&_.ui-button:disabled]:opacity-30"
            } else {
                "grid grid-cols-3 gap-2 [&_.ui-button]:w-full [&_.ui-button]:whitespace-nowrap [&_.ui-button]:px-2 [&_.ui-button:focus-visible]:ring-2 [&_.ui-button:focus-visible]:ring-pillbug-teal"
            }>
                <Show when=in_progress>
                    <button
                        type="button"
                        class="ui-button ui-button-secondary ui-button-sm"
                        title="Each press gives a little more: the bug, then the piece, then where it goes (H)"
                        on:click=move |_| puzzle.hint(costs_rating())
                        disabled=hint_disabled
                    >
                        {hint_label}
                    </button>
                    <button
                        type="button"
                        class=move || {
                            if confirm_reveal.get() {
                                "ui-button ui-button-danger ui-button-sm"
                            } else {
                                "ui-button ui-button-ghost ui-button-sm"
                            }
                        }
                        title="Plays the solution (S)"
                        on:click=reveal
                        disabled=move || !solving()
                    >
                        {move || {
                            if confirm_reveal.get() {
                                "Sure?"
                            } else {
                                if compact { "Solution" } else { "Show solution" }
                            }
                        }}
                    </button>
                    // There is always a way on: free before trying or when nothing is rated, a miss otherwise.
                    <button
                        type="button"
                        class="ui-button ui-button-ghost ui-button-sm"
                        title=move || {
                            if puzzle.can_skip() {
                                "Move on without trying; nothing is recorded (N)"
                            } else if costs_rating() {
                                "Show the solution; this try counts as a miss"
                            } else {
                                "Next puzzle (N)"
                            }
                        }
                        disabled=move || !solving()
                        on:click=move |_| {
                            if puzzle.can_skip() {
                                puzzle.skip(next)
                            } else if costs_rating() {
                                puzzle.give_up()
                            } else {
                                next.run(())
                            }
                        }
                    >
                        {move || {
                            if daily_result(puzzle) == Some(true) && puzzle.can_skip() {
                                "Harder one →"
                            } else if costs_rating() && !puzzle.can_skip() {
                                "Give up"
                            } else {
                                "Skip"
                            }
                        }}
                    </button>
                </Show>
                <Show when=move || phase.get() == PuzzlePhase::Failed>
                    <button
                        type="button"
                        class=if compact {
                            "hidden"
                        } else {
                            "text-white shadow ui-button ui-button-sm bg-pillbug-teal hover:brightness-110"
                        }
                        title="Start this puzzle again from the beginning, not rated (R)"
                        on:click=move |_| puzzle.retry()
                    >
                        "Retry"
                    </button>
                    <button
                        type="button"
                        class="ui-button ui-button-ghost ui-button-sm"
                        title="Plays the solution (S)"
                        on:click=move |_| puzzle.reveal()
                    >
                        {if compact { "Solution" } else { "Show solution" }}
                    </button>
                </Show>
                <Show when=move || {
                    compact && matches!(phase.get(), PuzzlePhase::Solved | PuzzlePhase::Failed)
                }>
                    <ShowLet some=analysis let:href>
                        <a
                            href=href
                            class="ui-button ui-button-secondary ui-button-sm no-link-style"
                        >
                            "Analyze"
                        </a>
                    </ShowLet>
                    <ShareButton compact=true />
                </Show>
                <Show when=move || compact && phase.get() == PuzzlePhase::Revealed>
                    <button
                        type="button"
                        class="ui-button ui-button-secondary ui-button-sm"
                        aria-label="Previous move of the solution"
                        disabled=move || puzzle.reveal_step.get() == 0
                        on:click=move |_| {
                            puzzle
                                .show_solution_step(
                                    puzzle.reveal_step.get_untracked().saturating_sub(1),
                                    true,
                                )
                        }
                    >
                        "◀"
                    </button>
                    <button
                        type="button"
                        class="ui-button ui-button-secondary ui-button-sm"
                        aria-label="Next move of the solution"
                        disabled=at_last_step
                        on:click=move |_| {
                            let last = puzzle.solution_steps();
                            puzzle
                                .show_solution_step(
                                    (puzzle.reveal_step.get_untracked() + 1).min(last),
                                    true,
                                )
                        }
                    >
                        "▶"
                    </button>
                </Show>
                <Show when=move || {
                    matches!(phase.get(), PuzzlePhase::Solved | PuzzlePhase::Revealed)
                }>
                    <button
                        type="button"
                        class="ui-button ui-button-ghost ui-button-sm"
                        on:click=move |_| puzzle.restart()
                        title="Play it again from the start, not rated (R)"
                    >
                        "Retry"
                    </button>
                </Show>
                <Show when=move || !compact && can_go_next()>
                    <div class="flex col-span-3 gap-2 items-center [&_.ui-button]:flex-1">
                        <button
                            type="button"
                            class=move || {
                                if phase.get() == PuzzlePhase::Failed {
                                    "ui-button ui-button-secondary ui-button-sm"
                                } else {
                                    "text-white shadow ui-button ui-button-sm bg-pillbug-teal hover:brightness-110"
                                }
                            }
                            title="Next puzzle (N)"
                            on:click=move |_| next.run(())
                        >
                            {move || after_daily_label(puzzle).unwrap_or("Next →")}
                        </button>
                        <TrainerVote />
                    </div>
                </Show>
            </div>
            <Show when=move || !compact && over()>
                <div class="flex flex-wrap gap-x-5 items-center">
                    <ShowLet some=analysis let:href>
                        <a
                            href=href
                            class="inline-flex items-center self-start text-sm font-semibold min-h-11 ui-text-link"
                        >
                            "Open in analysis →"
                        </a>
                    </ShowLet>
                    <ShareButton />
                </div>
            </Show>
            <Show when=move || !compact>
                <SolutionMoves />
            </Show>
            <ManualShareLink />
        </div>
    }
}

/// The solution in words, steppable: click a move or use ◀ ▶ (or the arrow keys).
#[component]
fn SolutionMoves() -> impl IntoView {
    let puzzle = expect_context::<PuzzleContext>();
    let step = puzzle.reveal_step;
    let at_last_step = move || step.get() >= puzzle.solution_steps();
    view! {
        <Show when=move || {
            puzzle.phase.get() == PuzzlePhase::Revealed && !puzzle.reveal_tokens.with(Vec::is_empty)
        }>
            <div class="flex flex-col gap-1 p-2 text-xs rounded bg-black/5 dark:bg-white/5">
                <div class="flex gap-1 items-center">
                    <button
                        type="button"
                        class="ui-button ui-button-secondary ui-button-xs"
                        aria-label="Previous move of the solution"
                        title="Previous move (←)"
                        on:click=move |_| {
                            puzzle.show_solution_step(step.get_untracked().saturating_sub(1), true)
                        }
                        disabled=move || step.get() == 0
                    >
                        "◀"
                    </button>
                    <button
                        type="button"
                        class="ui-button ui-button-secondary ui-button-xs"
                        aria-label="Next move of the solution"
                        title="Next move (→)"
                        on:click=move |_| {
                            let last = puzzle.solution_steps();
                            puzzle.show_solution_step((step.get_untracked() + 1).min(last), true)
                        }
                        disabled=at_last_step
                    >
                        "▶"
                    </button>
                    <span class="ml-1 text-gray-600 dark:text-gray-400">
                        {move || solution_step_label(puzzle, step.get())}
                    </span>
                </div>
            </div>
        </Show>
    }
}

#[component]
fn PuzzleAfter() -> impl IntoView {
    let puzzle = expect_context::<PuzzleContext>();
    let current = move || puzzle.puzzle.get();
    view! {
        // Keyed by puzzle, so opened disclosures (like the book hint) never carry over.
        <For
            each=move || current().into_iter()
            key=|puzzle| puzzle.nanoid.clone()
            children=move |puzzle_response| view! { <PuzzleCredits puzzle=puzzle_response /> }
        />
    }
}

#[component]
fn PuzzleCredits(puzzle: PuzzleResponse) -> impl IntoView {
    let context = expect_context::<PuzzleContext>();
    let shared = expect_context::<SharedLink>().0;
    let share = move |_| share_current_puzzle(context, shared);
    let at_position = source_position(&puzzle);
    let game_link = at_position
        .as_ref()
        .map(|(game, ply)| format!("/game/{game}?move={ply}"));
    let theme_link = game_link.clone();
    let game_analysis = at_position
        .as_ref()
        .map(|(game, ply)| format!("/analysis/{game}?move={ply}"));
    let authors = puzzle.authors.join(", ");
    let themes: Vec<String> = puzzle
        .themes
        .iter()
        .filter(|theme| !theme.starts_with("winIn"))
        .cloned()
        .collect();
    let has_themes = !themes.is_empty();
    let stats = move || {
        context.puzzle.with(|current| {
            current.as_ref().map(|p| {
                let plays = if p.plays == 1 {
                    "1 play".to_string()
                } else {
                    format!("{} plays", p.plays)
                };
                format!("{plays} · 👍 {} · 👎 {}", p.likes, p.dislikes)
            })
        })
    };
    view! {
        <div class="flex flex-col gap-2 pt-2 text-xs text-gray-600 border-t dark:text-gray-400 border-black/10 dark:border-white/10">
            <div>
                {if puzzle.authors == ["philanthus"] {
                    view! {
                        <span>
                            <span class="font-semibold">"Found by "</span>
                            "our puzzle engine"
                        </span>
                    }
                        .into_any()
                } else {
                    view! {
                        <span>
                            <span class="font-semibold">"Puzzle by "</span>
                            {authors.clone()}
                        </span>
                    }
                        .into_any()
                }}
                {puzzle
                    .collection
                    .is_some()
                    .then_some(
                        " · from Hive Puzzles Vol. 1 by Enrique Sedano, Jonas Kronander and Christian Sperling",
                    )}
            </div>
            {puzzle
                .hint
                .clone()
                .map(|hint| {
                    view! {
                        <details>
                            <summary class="cursor-pointer">"Hint from the book (free)"</summary>
                            <p class="mt-1">{hint}</p>
                        </details>
                    }
                })}
            // Motifs give the idea away, so they only show once the puzzle is solved or revealed;
            // after a miss the player may still retry.
            <Show when=move || {
                has_themes
                    && matches!(context.phase.get(), PuzzlePhase::Solved | PuzzlePhase::Revealed)
            }>
                <div class="flex flex-wrap gap-1">
                    {themes
                        .iter()
                        .map(|theme| {
                            match (theme.as_str(), theme_link.clone()) {
                                ("missedWin", Some(href)) => {
                                    view! {
                                        <a
                                            class="py-0.5 px-1.5 rounded bg-black/5 ui-text-link dark:bg-white/10"
                                            href=href
                                        >
                                            {theme_label(theme)}
                                        </a>
                                    }
                                        .into_any()
                                }
                                _ => {
                                    view! {
                                        <span class="py-0.5 px-1.5 rounded bg-black/5 dark:bg-white/10">
                                            {theme_label(theme)}
                                        </span>
                                    }
                                        .into_any()
                                }
                            }
                        })
                        .collect_view()}
                </div>
            </Show>
            <div class="flex flex-wrap gap-y-1 gap-x-3">
                <Show when=move || !context.phase.get().is_over()>
                    <button
                        type="button"
                        class="inline-flex items-center font-semibold min-h-11 ui-text-link"
                        on:click=share
                    >
                        {move || match shared.get() {
                            ShareState::Copied => "Link copied ✓",
                            _ => "Share link",
                        }}
                    </button>
                </Show>
                // Only after this visit's try: the original game would give the answer away.
                <Show when=move || {
                    context.reported.get()
                }>
                    {game_link
                        .clone()
                        .map(|href| {
                            view! {
                                <a class="inline-flex items-center min-h-11 ui-text-link" href=href>
                                    "Original game"
                                </a>
                            }
                        })}
                    {game_analysis
                        .clone()
                        .map(|href| {
                            view! {
                                <a class="inline-flex items-center min-h-11 ui-text-link" href=href>
                                    "Analyze the game here"
                                </a>
                            }
                        })}
                </Show>
            </div>
            <div>{stats}</div>
        </div>
    }
}

pub fn theme_label(theme: &str) -> String {
    if let Some(moves) = theme.strip_prefix("winIn") {
        return format!("Win in {moves}");
    }
    match theme {
        "missedWin" => "Missed in the original game".to_string(),
        "throw" => "Pillbug throw".to_string(),
        "queenThrow" => "Queen throw".to_string(),
        "climb" => "Beetle climb".to_string(),
        "quietMove" => "Quiet first move".to_string(),
        "book" => "From Hive Puzzles Vol. 1".to_string(),
        "mosquito" => "Mosquito trick".to_string(),
        "easy" => "Warm-up".to_string(),
        other => other.to_string(),
    }
}

pub fn collection_label(collection: &str) -> String {
    match collection {
        "hive-puzzles-vol1" => "Hive Puzzles Vol. 1".to_string(),
        shared_types::MY_GAMES_COLLECTION => "Puzzles from my games".to_string(),
        shared_types::OLD_DAILIES_COLLECTION => "Old puzzles of the day".to_string(),
        other => other.to_string(),
    }
}

#[component]
fn Chip(
    #[prop(into)] active: Signal<bool>,
    label: &'static str,
    title: &'static str,
    on_pick: Callback<()>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            title=title
            class=move || {
                format!(
                    "ui-choice ui-choice-xs min-h-11 px-3 {}",
                    if active.get() { "ui-choice-active" } else { "ui-choice-inactive" },
                )
            }
            aria-pressed=move || active.get().to_string()
            on:click=move |_| on_pick.run(())
        >
            {label}
        </button>
    }
}

#[component]
fn PuzzleFooter(
    rating: RwSignal<Option<i32>>,
    #[prop(optional)] nav_elsewhere: bool,
) -> impl IntoView {
    let puzzle = expect_context::<PuzzleContext>();
    let settings_summary = move || {
        let pieces = match puzzle.game_type.get() {
            PuzzleGameTypeFilter::Any => "All pieces",
            PuzzleGameTypeFilter::Mlp => "Expansions",
            PuzzleGameTypeFilter::Base => "Base game",
        };
        let difficulty = match puzzle.difficulty.get() {
            PuzzleDifficulty::Easier => "Easier",
            PuzzleDifficulty::Normal => "Normal",
            PuzzleDifficulty::Harder => "Harder",
        };
        let length = match puzzle.length.get() {
            PuzzleLength::Any => "any length",
            PuzzleLength::One => "1 move",
            PuzzleLength::Two => "2 moves",
            PuzzleLength::ThreePlus => "3+ moves",
        };
        format!("{pieces} · {difficulty} · {length}")
    };
    let signed_in = is_signed_in();
    let choice = move |filter: PuzzleGameTypeFilter, label: &'static str, title: &'static str| {
        view! {
            <button
                type="button"
                title=title
                class=move || {
                    format!(
                        "ui-choice ui-choice-xs min-h-11 px-3 {}",
                        if puzzle.game_type.get() == filter {
                            "ui-choice-active"
                        } else {
                            "ui-choice-inactive"
                        },
                    )
                }
                aria-pressed=move || (puzzle.game_type.get() == filter).to_string()
                on:click=move |_| puzzle.set_game_type(filter)
            >
                {label}
            </button>
        }
    };
    view! {
        <div class="flex flex-col gap-2 pt-2 text-sm border-t border-black/10 dark:border-white/10">
            <Show
                when=move || signed_in.get()
                fallback=move || {
                    let remember_return = use_remember_return();
                    let hidden = move || !nav_elsewhere && puzzle.phase.get().is_over();
                    // On phones the top card already offers sign-in once the puzzle is over.
                    view! {
                        <p class="text-xs text-gray-600 dark:text-gray-400" class:hidden=hidden>
                            <a
                                href="/login"
                                class="ui-text-link"
                                on:click=move |_| remember_return()
                            >
                                "Sign in"
                            </a>
                            " to get a puzzle rating."
                        </p>
                    }
                }
            >
                <div class="flex gap-2 justify-between items-baseline">
                    <span>"Your rating"</span>
                    <span class="text-base font-bold">
                        {move || {
                            rating.get().map(|r| r.to_string()).unwrap_or_else(|| "…".to_string())
                        }}
                    </span>
                </div>
            </Show>
            <Show when=move || puzzle.daily.with(Option::is_some)>
                <a
                    class="inline-flex items-center text-sm min-h-11 ui-text-link"
                    href="/puzzles/dailies"
                >
                    "Past puzzles of the day →"
                </a>
            </Show>
            <Show when=move || { puzzle.solved_streak.get() > 1 }>
                <div class="text-xs text-grasshopper-green">
                    {move || format!("{} solved in a row", puzzle.solved_streak.get())}
                </div>
            </Show>
            <details
                class="group"
                on:toggle=move |evt| {
                    let details = event_target::<web_sys::Element>(&evt);
                    if details.has_attribute("open") {
                        details.scroll_into_view_with_bool(false);
                    }
                }
            >
                <summary class="flex justify-between items-center cursor-pointer min-h-11">
                    <span class="font-semibold">"Puzzle settings"</span>
                    <span class="text-xs text-gray-600 dark:text-gray-400">{settings_summary}</span>
                </summary>
                <div class="flex flex-col gap-3 pt-1">
                    <div
                        class="flex flex-col gap-1"
                        role="group"
                        aria-label="Which puzzles to show"
                    >
                        <span class="text-xs text-gray-600 dark:text-gray-400">"Pieces"</span>
                        <div class="flex flex-wrap gap-1">
                            {choice(PuzzleGameTypeFilter::Any, "All", "Any set of pieces")}
                            {choice(
                                PuzzleGameTypeFilter::Mlp,
                                "Expansions",
                                "With the Mosquito, Ladybug and Pillbug",
                            )}
                            {choice(
                                PuzzleGameTypeFilter::Base,
                                "Base game",
                                "Only the five base bugs",
                            )}
                        </div>
                    </div>
                    <div class="flex flex-col gap-1" role="group" aria-label="Difficulty">
                        <span class="text-xs text-gray-600 dark:text-gray-400">"Difficulty"</span>
                        <div class="flex flex-wrap gap-1">
                            <Chip
                                active=Signal::derive(move || {
                                    puzzle.difficulty.get() == PuzzleDifficulty::Easier
                                })
                                label="Easier"
                                title="About 300 below your puzzle rating"
                                on_pick=Callback::new(move |()| {
                                    puzzle.set_difficulty(PuzzleDifficulty::Easier)
                                })
                            />
                            <Chip
                                active=Signal::derive(move || {
                                    puzzle.difficulty.get() == PuzzleDifficulty::Normal
                                })
                                label="Normal"
                                title="Around your puzzle rating"
                                on_pick=Callback::new(move |()| {
                                    puzzle.set_difficulty(PuzzleDifficulty::Normal)
                                })
                            />
                            <Chip
                                active=Signal::derive(move || {
                                    puzzle.difficulty.get() == PuzzleDifficulty::Harder
                                })
                                label="Harder"
                                title="About 300 above your puzzle rating"
                                on_pick=Callback::new(move |()| {
                                    puzzle.set_difficulty(PuzzleDifficulty::Harder)
                                })
                            />
                        </div>
                    </div>
                    <div class="flex flex-col gap-1" role="group" aria-label="Length">
                        <span class="text-xs text-gray-600 dark:text-gray-400">"Length"</span>
                        <div class="flex flex-wrap gap-1">
                            <Chip
                                active=Signal::derive(move || {
                                    puzzle.length.get() == PuzzleLength::Any
                                })
                                label="Any"
                                title="Any number of moves"
                                on_pick=Callback::new(move |()| {
                                    puzzle.set_length(PuzzleLength::Any)
                                })
                            />
                            <Chip
                                active=Signal::derive(move || {
                                    puzzle.length.get() == PuzzleLength::One
                                })
                                label="1 move"
                                title="Win in one move"
                                on_pick=Callback::new(move |()| {
                                    puzzle.set_length(PuzzleLength::One)
                                })
                            />
                            <Chip
                                active=Signal::derive(move || {
                                    puzzle.length.get() == PuzzleLength::Two
                                })
                                label="2 moves"
                                title="Win in two of your moves"
                                on_pick=Callback::new(move |()| {
                                    puzzle.set_length(PuzzleLength::Two)
                                })
                            />
                            <Chip
                                active=Signal::derive(move || {
                                    puzzle.length.get() == PuzzleLength::ThreePlus
                                })
                                label="3+ moves"
                                title="Win in three or more of your moves"
                                on_pick=Callback::new(move |()| {
                                    puzzle.set_length(PuzzleLength::ThreePlus)
                                })
                            />
                        </div>
                    </div>
                    <label class="flex gap-3 items-center text-xs cursor-pointer min-h-11">
                        <input
                            class="w-5 h-5 shrink-0 accent-pillbug-teal"
                            type="checkbox"
                            aria-label="One-click moves in puzzles"
                            prop:checked=move || puzzle.one_click.get()
                            on:change=move |evt| puzzle.set_one_click(event_target_checked(&evt))
                        />
                        "One-click moves"
                    </label>
                </div>
            </details>
            {(!nav_elsewhere).then(|| view! { <PuzzleNav /> })}
            <p class="hidden text-gray-500 lg:block text-[11px]">
                "Keys: H hint · S solution · ← → step · R retry · N next or skip · Esc cancel"
            </p>
        </div>
    }
}

pub fn signed(delta: i32) -> String {
    match delta.signum() {
        1 => format!("+{delta}"),
        0 => "±0".to_string(),
        _ => delta.to_string(),
    }
}

/// The puzzle's start as an analysis board, built from its own moves so it works even when the
/// source game is not on this server.
fn analysis_link(puzzle: &PuzzleResponse) -> Option<String> {
    let Some(history) = puzzle.history.as_deref() else {
        return puzzle.position.as_deref().map(|hop| {
            format!(
                "/analysis?hop={}&puzzle={}",
                encode_component(hop),
                puzzle.nanoid
            )
        });
    };
    let moves: Vec<&str> = history
        .split(';')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect();
    let to_move = if moves.len().is_multiple_of(2) {
        "White"
    } else {
        "Black"
    };
    let uhp = format!(
        "{};InProgress;{}[{}];{}",
        puzzle.game_type,
        to_move,
        moves.len() / 2 + 1,
        moves.join(";")
    );
    Some(format!(
        "/analysis?uhp={}&puzzle={}",
        encode_component(&uhp),
        puzzle.nanoid
    ))
}

/// The game a puzzle was mined from and how many moves into it the puzzle starts.
fn source_position(puzzle: &PuzzleResponse) -> Option<(String, usize)> {
    let game = puzzle.source_game.clone()?;
    let plies = puzzle
        .history
        .as_deref()?
        .split(';')
        .filter(|token| !token.trim().is_empty())
        .count();
    Some((game, plies))
}

fn encode_component(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

fn solution_step_label(puzzle: PuzzleContext, step: usize) -> String {
    let steps = puzzle.solution_steps();
    if step >= steps && steps > 0 {
        format!(
            "Step {step} of {steps} · {} wins",
            color_name(puzzle.solver())
        )
    } else {
        format!("Step {step} of {steps}")
    }
}

/// How today's puzzle went on the puzzle-of-the-day page: the earlier first try, else this visit's.
fn daily_result(puzzle: PuzzleContext) -> Option<bool> {
    let status = puzzle.daily.get()?;
    status.my_result.or_else(|| {
        puzzle
            .reported
            .get()
            .then(|| puzzle.first_result.get() == Some(true) && !puzzle.hinted.get())
    })
}

/// Today's outcome on the puzzle-of-the-day page, counting this visit's first try too.
fn daily_summary(puzzle: PuzzleContext) -> Option<String> {
    let status = puzzle.daily.get()?;
    let result = daily_result(puzzle)?;
    let this_visit = status.my_result.is_none();
    let (tried, solved) = if this_visit {
        (status.tried + 1, status.solved + i64::from(result))
    } else {
        (status.tried, status.solved)
    };
    let verdict = if result {
        "You solved today's puzzle"
    } else {
        "You missed today's puzzle"
    };
    let streak = status
        .streak
        .map(|streak| streak + i64::from(this_visit && result))
        .filter(|streak| *streak > 0)
        .map(|streak| format!(" · 🔥 {streak}-day streak"))
        .unwrap_or_default();
    Some(format!(
        "{verdict}{streak} · {solved} of {tried} solved it · next one in {}",
        until_next_utc_day()
    ))
}

/// After the daily: a harder puzzle once it is solved, otherwise ordinary training.
fn after_daily_label(puzzle: PuzzleContext) -> Option<&'static str> {
    puzzle.daily.with(Option::is_some).then(|| {
        if daily_result(puzzle) == Some(true) {
            "Try a harder one →"
        } else {
            "More puzzles →"
        }
    })
}

fn until_next_utc_day() -> String {
    const DAY_MS: f64 = 86_400_000.0;
    #[cfg(not(feature = "ssr"))]
    let now = web_sys::js_sys::Date::now();
    #[cfg(feature = "ssr")]
    let now = 0.0;
    let left = ((now / DAY_MS).floor() + 1.0) * DAY_MS - now;
    let minutes = (left / 60_000.0).ceil() as i64;
    if minutes >= 60 {
        format!("{}h", (minutes + 30) / 60)
    } else {
        format!("{minutes} min")
    }
}

fn or_blank(text: String) -> String {
    if text.is_empty() {
        "\u{a0}".to_string()
    } else {
        text
    }
}

pub fn use_remember_return() -> impl Fn() + Copy {
    let referrer = expect_context::<crate::providers::RefererContext>().pathname;
    let pathname = leptos_router::hooks::use_location().pathname;
    move || referrer.set_value(pathname.get_untracked())
}

#[derive(Clone, Copy)]
struct SharedLink(RwSignal<ShareState>);

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "ssr", allow(dead_code))]
enum ShareState {
    Idle,
    Copied,
    /// Copying failed, so the link is shown to copy by hand.
    Manual(String),
}

fn share_current_puzzle(puzzle: PuzzleContext, shared: RwSignal<ShareState>) {
    let Some((nanoid, moves, daily)) = puzzle.puzzle.with_untracked(|current| {
        current
            .as_ref()
            .map(|p| (p.nanoid.clone(), p.solver_moves, p.daily_day.is_some()))
    }) else {
        return;
    };
    let task = format!(
        "{} to win in {}",
        color_name(puzzle.solver()),
        moves_label(moves)
    );
    let title = if daily {
        format!("Puzzle of the day: {task}")
    } else {
        format!("{task} · Hive puzzle")
    };
    #[cfg(feature = "ssr")]
    let _ = (title, shared, nanoid);
    #[cfg(not(feature = "ssr"))]
    if let Some(window) = web_sys::window() {
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::{js_sys, JsFuture};
        let url = format!(
            "{}/puzzles/{nanoid}",
            window.location().origin().unwrap_or_default()
        );
        let navigator = window.navigator();
        let touch = window
            .match_media("(pointer: coarse)")
            .ok()
            .flatten()
            .is_some_and(|query| query.matches());
        let share = js_sys::Reflect::get(&navigator, &"share".into())
            .ok()
            .and_then(|share| share.dyn_into::<js_sys::Function>().ok())
            .filter(|_| touch);
        if let Some(share) = share {
            let data = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&data, &"title".into(), &title.into());
            let _ = js_sys::Reflect::set(&data, &"text".into(), &"Can you find it?".into());
            let _ = js_sys::Reflect::set(&data, &"url".into(), &url.clone().into());
            if let Ok(promise) = share.call1(&navigator, &data) {
                spawn_local(async move {
                    if let Err(error) = JsFuture::from(js_sys::Promise::from(promise)).await {
                        let cancelled = js_sys::Reflect::get(&error, &"name".into())
                            .ok()
                            .and_then(|name| name.as_string())
                            .is_some_and(|name| name == "AbortError");
                        if !cancelled {
                            let _ = shared.try_set(ShareState::Manual(url));
                        }
                    }
                });
            }
            return;
        }
        let written = navigator.clipboard().write_text(&url);
        spawn_local(async move {
            if JsFuture::from(written).await.is_ok() {
                let _ = shared.try_set(ShareState::Copied);
                leptos::leptos_dom::helpers::set_timeout(
                    move || {
                        if shared.try_get_untracked() == Some(ShareState::Copied) {
                            let _ = shared.try_set(ShareState::Idle);
                        }
                    },
                    std::time::Duration::from_millis(2500),
                );
            } else {
                let _ = shared.try_set(ShareState::Manual(url));
            }
        });
    }
}

#[component]
fn ManualShareLink() -> impl IntoView {
    let shared = expect_context::<SharedLink>().0;
    let link = move || match shared.get() {
        ShareState::Manual(url) => Some(url),
        _ => None,
    };
    view! {
        <ShowLet some=link let:url>
            <label class="flex flex-col gap-1 w-full text-xs">
                "Copy this link:"
                <input
                    type="text"
                    readonly
                    class="py-2 px-2 w-full text-sm rounded border border-black/20 bg-white/80 dark:border-white/20 dark:bg-black/30"
                    prop:value=url
                    on:focus=move |evt| {
                        event_target::<web_sys::HtmlInputElement>(&evt).select();
                    }
                />
            </label>
        </ShowLet>
    }
}

#[component]
fn ShareButton(#[prop(optional)] compact: bool) -> impl IntoView {
    let puzzle = expect_context::<PuzzleContext>();
    let shared = expect_context::<SharedLink>().0;
    view! {
        <button
            type="button"
            class=if compact {
                "ui-button ui-button-secondary ui-button-sm"
            } else {
                "inline-flex items-center text-sm font-semibold min-h-11 ui-text-link"
            }
            title="Send this puzzle to a friend"
            on:click=move |_| share_current_puzzle(puzzle, shared)
        >
            {move || match (shared.get(), compact) {
                (ShareState::Copied, _) => "Copied ✓",
                (_, true) => "Share",
                (_, false) => "Share this puzzle",
            }}
        </button>
    }
}
