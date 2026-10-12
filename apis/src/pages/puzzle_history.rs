use crate::{
    components::{
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::{
            empty_state::EmptyState,
            puzzle_stars::PuzzleStars,
            thumbnail_pieces::ThumbnailPieces,
        },
    },
    functions::puzzles::{my_puzzle_rating, puzzle_history},
    responses::PuzzleHistoryRow,
};
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_query_map};
use shared_types::{start_state, PuzzleHistoryFilter};

#[component]
pub fn PuzzleHistory() -> impl IntoView {
    let signed_in = crate::providers::puzzle::is_signed_in();
    let remember_return = crate::components::organisms::puzzle_trainer::use_remember_return();
    let query = use_query_map();
    let filter = Memo::new(move |_| {
        query.with(|query| match query.get_str("show") {
            Some("solved") => PuzzleHistoryFilter::Solved,
            Some("missed") => PuzzleHistoryFilter::Failed,
            Some("unstarred") => PuzzleHistoryFilter::Unstarred,
            _ => PuzzleHistoryFilter::All,
        })
    });
    let page = Memo::new(move |_| {
        query.with(|query| {
            query
                .get_str("page")
                .and_then(|page| page.parse::<i64>().ok())
                .unwrap_or(0)
                .max(0)
        })
    });
    let navigate = use_navigate();
    let go = Callback::new(move |(filter, page): (PuzzleHistoryFilter, i64)| {
        let show = match filter {
            PuzzleHistoryFilter::All => "all",
            PuzzleHistoryFilter::Solved => "solved",
            PuzzleHistoryFilter::Failed => "missed",
            PuzzleHistoryFilter::Unstarred => "unstarred",
        };
        navigate(
            &format!("/puzzles/history?show={show}&page={page}"),
            Default::default(),
        );
    });
    let rows = Resource::new(
        move || (filter.get(), page.get()),
        |(filter, page)| puzzle_history(filter, page),
    );
    let rating = Resource::new(|| (), |_| my_puzzle_rating());
    let tab = move |value: PuzzleHistoryFilter, label: &'static str| {
        view! {
            <button
                type="button"
                class=move || {
                    format!(
                        "ui-choice ui-choice-sm min-h-11 px-3 shrink-0 whitespace-nowrap {}",
                        if filter.get() == value {
                            "ui-choice-active"
                        } else {
                            "ui-choice-inactive"
                        },
                    )
                }
                aria-pressed=move || (filter.get() == value).to_string()
                on:click=move |_| go.run((value, 0))
            >
                {label}
            </button>
        }
    };
    view! {
        <PageShell>
            <PageHeader title="Puzzle history" />
            <div class="flex flex-wrap gap-3 justify-between items-center">
                <div
                    class=move || {
                        if signed_in.get() {
                            "flex overflow-x-auto gap-1 max-w-full"
                        } else {
                            "hidden"
                        }
                    }
                    role="group"
                    aria-label="Show"
                >
                    {tab(PuzzleHistoryFilter::All, "All")}
                    {tab(PuzzleHistoryFilter::Solved, "Solved")}
                    {tab(PuzzleHistoryFilter::Failed, "Missed")}
                    {tab(PuzzleHistoryFilter::Unstarred, "Not starred")}
                </div>
                <div class="flex gap-3 items-center text-sm">
                    <Suspense>
                        {move || Suspend::new(async move {
                            rating
                                .await
                                .ok()
                                .flatten()
                                .map(|rating| {
                                    view! {
                                        <span>
                                            "Puzzle rating " <b>{rating.rating}</b>
                                            {format!(
                                                " · {} played, {} solved",
                                                rating.played,
                                                rating.won,
                                            )}
                                        </span>
                                    }
                                })
                        })}
                    </Suspense>
                    <a
                        href="/puzzles"
                        class="text-white whitespace-nowrap shadow min-h-11 ui-button ui-button-sm no-link-style bg-pillbug-teal hover:brightness-110"
                    >
                        "Solve puzzles"
                    </a>
                </div>
            </div>
            <Transition fallback=|| {
                view! { <p class="text-sm">"Loading…"</p> }
            }>
                {move || Suspend::new(async move {
                    match rows.await {
                        Err(_) => {
                            view! {
                                <div class="flex flex-col gap-3 items-start ui-empty-state">
                                    <div class="text-sm font-bold">
                                        "Sign in to see your puzzles"
                                    </div>
                                    <div class="text-xs">
                                        "Your puzzle history and rating are kept with your account."
                                    </div>
                                    <a
                                        href="/login"
                                        on:click=move |_| remember_return()
                                        class="ui-button ui-button-primary ui-button-sm no-link-style"
                                    >
                                        "Sign in"
                                    </a>
                                </div>
                            }
                                .into_any()
                        }
                        Ok(rows) if rows.is_empty() => {
                            view! {
                                <EmptyState
                                    title="Nothing here yet"
                                    message="Solve a few puzzles and they will show up here, ready to replay."
                                />
                            }
                                .into_any()
                        }
                        Ok(rows) => {
                            let full_page = rows.len() == 50;
                            view! {
                                <ul class="flex flex-col divide-y divide-black/10 dark:divide-white/10">
                                    {rows
                                        .into_iter()
                                        .map(|row| view! { <HistoryRow row /> })
                                        .collect_view()}
                                </ul>
                                <div class="flex gap-2 justify-center">
                                    <Show when=move || { page.get() > 0 }>
                                        <button
                                            type="button"
                                            class="min-h-11 ui-button ui-button-secondary"
                                            on:click=move |_| {
                                                go.run((filter.get_untracked(), page.get_untracked() - 1))
                                            }
                                        >
                                            "← Newer"
                                        </button>
                                    </Show>
                                    <Show when=move || full_page>
                                        <button
                                            type="button"
                                            class="min-h-11 ui-button ui-button-secondary"
                                            on:click=move |_| {
                                                go.run((filter.get_untracked(), page.get_untracked() + 1))
                                            }
                                        >
                                            "Older →"
                                        </button>
                                    </Show>
                                </div>
                            }
                                .into_any()
                        }
                    }
                })}
            </Transition>
        </PageShell>
    }
}

#[component]
fn HistoryRow(row: PuzzleHistoryRow) -> impl IntoView {
    let played_at = row.played_at;
    let when = RwSignal::new(played_at.format("%Y-%m-%d %H:%M UTC").to_string());
    Effect::new(move |_| when.set(local_time(played_at)));
    let tags: Vec<String> = row
        .themes
        .iter()
        .filter(|theme| !theme.starts_with("winIn"))
        .map(|theme| crate::components::organisms::puzzle_trainer::theme_label(theme))
        .collect();
    let my_stars = RwSignal::new(row.my_stars);
    let nanoid = row.nanoid.clone();
    let start = start_state(
        &row.game_type,
        row.history.as_deref(),
        row.position.as_deref(),
    )
    .ok();
    let task = match (&row.title, &start) {
        (Some(title), _) => title.clone(),
        (None, Some(start)) => format!(
            "{} to win in {}",
            if start.turn_color == hive_lib::Color::White {
                "White"
            } else {
                "Black"
            },
            if row.solver_moves == 1 {
                "1 move".to_string()
            } else {
                format!("{} moves", row.solver_moves)
            },
        ),
        (None, None) => format!("Win in {}", row.solver_moves),
    };
    let thumbnail = start.map(|start| {
        let board = StoredValue::new(start.board);
        view! {
            <div class="overflow-hidden w-20 h-20 rounded sm:w-28 sm:h-28 shrink-0 bg-black/5 dark:bg-white/5">
                <ThumbnailPieces board />
            </div>
        }
    });
    let result = match (row.solved, row.hinted) {
        (true, false) => ("Solved", "text-grasshopper-green"),
        (true, true) => ("Solved with a hint", "text-orange-twilight"),
        (false, true) => ("Missed · used a hint", "text-red-700 dark:text-red-400"),
        (false, false) => ("Missed", "text-red-700 dark:text-red-400"),
    };
    let change = row.rating_change.map(|delta| {
        let class = match delta.signum() {
            1 => "text-grasshopper-green",
            0 => "text-gray-500",
            _ => "text-red-700 dark:text-red-400",
        };
        let text = crate::components::organisms::puzzle_trainer::signed(delta);
        view! { <span class=format!("text-xs {class}")>{text}</span> }
    });
    view! {
        <li class="flex flex-col gap-1 py-2 sm:flex-row sm:gap-4 sm:items-center">
            <a
                href=format!("/puzzles/{}", row.nanoid)
                class="flex flex-1 gap-3 items-center min-w-0 min-h-16 no-link-style group"
            >
                {thumbnail}
                <span class="flex flex-col min-w-0">
                    <span class="text-sm font-semibold sm:text-base group-hover:underline">
                        {task}
                    </span>
                    <span class=format!(
                        "text-sm font-semibold {}",
                        result.1,
                    )>{result.0} " " {change}</span>
                    <span class="text-xs text-gray-600 dark:text-gray-400">
                        {format!(
                            "{} · difficulty now {} · ",
                            if row.game_type == "Base" { "Base game" } else { "Expansions" },
                            row.rating,
                        )} <time datetime=played_at.to_rfc3339()>{when}</time>
                    </span>
                    <span class="flex flex-wrap gap-1">
                        {tags
                            .into_iter()
                            .map(|tag| {
                                view! {
                                    <span class="py-0.5 px-1.5 text-xs rounded bg-black/5 dark:bg-white/10">
                                        {tag}
                                    </span>
                                }
                            })
                            .collect_view()}
                    </span>
                </span>
            </a>
            <div class="sm:w-80 shrink-0">
                <PuzzleStars
                    nanoid=Signal::derive(move || nanoid.clone())
                    my_stars
                    prominent=false
                    compact=true
                />
            </div>
        </li>
    }
}

fn local_time(at: chrono::DateTime<chrono::Utc>) -> String {
    #[cfg(not(feature = "ssr"))]
    {
        let date = web_sys::js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(
            at.timestamp_millis() as f64,
        ));
        let options = web_sys::js_sys::Object::new();
        let _ = web_sys::js_sys::Reflect::set(&options, &"dateStyle".into(), &"medium".into());
        let _ = web_sys::js_sys::Reflect::set(&options, &"timeStyle".into(), &"short".into());
        String::from(date.to_locale_string("default", &options))
    }
    #[cfg(feature = "ssr")]
    {
        at.format("%Y-%m-%d %H:%M UTC").to_string()
    }
}
