use crate::{
    components::{
        layouts::{page_header::PageHeader, page_shell::PageShell},
        organisms::puzzle_trainer::{collection_label, theme_label},
    },
    functions::puzzles::puzzle_sets,
    responses::PuzzleSetResponse,
};
use leptos::prelude::*;

const BOOK: &str = "hive-puzzles-vol1";
const THEMES: [(&str, &str); 7] = [
    ("easy", "Quick, easy wins."),
    ("throw", "The Pillbug moves a piece into place."),
    ("queenThrow", "The Pillbug moves the Queen herself."),
    ("mosquito", "The Mosquito climbs or jumps."),
    ("climb", "Climb onto the hive to finish."),
    ("quietMove", "The first move sets up the win."),
    ("missedWin", "Wins the player missed."),
];

#[derive(Clone)]
struct SetCard {
    title: String,
    blurb: String,
    href: String,
    total: i64,
    solved: i64,
}

fn count(sets: &[PuzzleSetResponse], keys: &[&str]) -> (i64, i64) {
    sets.iter()
        .filter(|set| keys.contains(&set.key.as_str()))
        .fold((0, 0), |(total, solved), set| {
            (total + set.total, solved + set.solved)
        })
}

fn cards(sets: &[PuzzleSetResponse]) -> (Vec<SetCard>, Vec<SetCard>, Vec<SetCard>) {
    let card = |title: String, blurb: &str, href: String, keys: &[&str]| {
        let (total, solved) = count(sets, keys);
        SetCard {
            title,
            blurb: blurb.to_string(),
            href,
            total,
            solved,
        }
    };
    let book_key = format!("collection:{BOOK}");
    let book = vec![card(
        collection_label(BOOK),
        "By Enrique Sedano, Jonas Kronander and Christian Sperling.",
        format!("/puzzles?collection={BOOK}"),
        &[book_key.as_str()],
    )];
    let lengths = vec![
        card(
            "Win in 1".to_string(),
            "One move to win.",
            "/puzzles?length=1".to_string(),
            &["moves:1"],
        ),
        card(
            "Win in 2".to_string(),
            "Two moves to win.",
            "/puzzles?length=2".to_string(),
            &["moves:2"],
        ),
        card(
            "Win in 3 or more".to_string(),
            "Three or more moves.",
            "/puzzles?length=3".to_string(),
            &["moves:3", "moves:4"],
        ),
    ];
    let themes = THEMES
        .iter()
        .map(|(theme, blurb)| {
            card(
                theme_label(theme),
                blurb,
                format!("/puzzles?theme={theme}"),
                &[theme],
            )
        })
        .filter(|card| card.total > 0)
        .collect();
    (book, lengths, themes)
}

#[component]
pub fn PuzzleBrowse() -> impl IntoView {
    let sets = Resource::new(|| (), |_| puzzle_sets());
    let signed_in = crate::providers::puzzle::is_signed_in();
    let section = move |title: &'static str, cards: Vec<SetCard>| {
        view! {
            <section class="flex flex-col gap-3">
                <h2 class="text-lg font-bold">{title}</h2>
                <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4">
                    {cards
                        .into_iter()
                        .map(|card| view! { <SetCardView card signed_in /> })
                        .collect_view()}
                </div>
            </section>
        }
    };
    view! {
        <PageShell>
            <PageHeader title="Puzzle sets" />
            <div class="flex flex-wrap gap-3 justify-between items-center text-sm">
                <a
                    href="/puzzles"
                    class="text-white whitespace-nowrap shadow min-h-11 ui-button ui-button-sm no-link-style bg-pillbug-teal hover:brightness-110"
                >
                    "Mixed puzzles"
                </a>
            </div>
            <Suspense fallback=|| {
                view! { <p class="text-sm">"Loading…"</p> }
            }>
                {move || Suspend::new(async move {
                    match sets.await {
                        Ok(sets) => {
                            let (book, lengths, themes) = cards(&sets);
                            view! {
                                <div class="flex flex-col gap-8">
                                    {section("The puzzle book", book)}
                                    {section("By length", lengths)} {section("By idea", themes)}
                                </div>
                            }
                                .into_any()
                        }
                        Err(_) => {
                            view! { <p class="text-sm">"The puzzle sets could not be loaded."</p> }
                                .into_any()
                        }
                    }
                })}
            </Suspense>
        </PageShell>
    }
}

#[component]
fn SetCardView(card: SetCard, signed_in: Signal<bool>) -> impl IntoView {
    let percent = if card.total > 0 {
        (card.solved * 100 / card.total).clamp(0, 100)
    } else {
        0
    };
    let solved = card.solved;
    let total = card.total;
    view! {
        <a
            href=card.href
            class="flex flex-col gap-2 p-4 rounded-lg border shadow-sm transition-colors no-link-style border-black/10 bg-even-light dark:border-white/10 dark:bg-surface-panel hover:border-pillbug-teal"
        >
            <span class="text-base font-bold">{card.title}</span>
            <span class="text-sm text-gray-700 dark:text-gray-300">{card.blurb}</span>
            <span class="text-xs text-gray-600 dark:text-gray-400">
                {move || {
                    if signed_in.get() {
                        format!("{total} puzzles · {solved} solved")
                    } else {
                        format!("{total} puzzles")
                    }
                }}
            </span>
            <Show when=move || signed_in.get()>
                <span class="overflow-hidden h-1.5 rounded-full bg-black/15 dark:bg-white/25">
                    <span
                        class="block h-full rounded-full bg-grasshopper-green"
                        style=format!("width: {percent}%")
                    ></span>
                </span>
            </Show>
            <span class="self-start mt-1 text-sm font-semibold text-pillbug-teal">"Start →"</span>
        </a>
    }
}
