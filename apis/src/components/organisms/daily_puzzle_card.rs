use crate::{
    components::{
        molecules::thumbnail_pieces::ThumbnailPieces,
        organisms::puzzle_trainer::moves_label,
    },
    functions::puzzles::daily_puzzle,
    providers::{AuthContext, AuthIdentity},
    responses::DailyPuzzleResponse,
};
use hive_lib::Color;
use leptos::prelude::*;

#[component]
pub fn DailyPuzzleCard() -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let daily = Resource::new(
        move || auth.identity.get().and_then(AuthIdentity::user_id),
        |_| daily_puzzle(),
    );
    view! {
        <Transition>
            {move || {
                daily
                    .get()
                    .map(|daily| match daily {
                        Ok(Some(daily)) => view! { <DailyCard daily /> }.into_any(),
                        _ => view! { <PuzzlesCard /> }.into_any(),
                    })
            }}
        </Transition>
    }
}

#[component]
fn DailyCard(daily: DailyPuzzleResponse) -> impl IntoView {
    let start = shared_types::start_state(
        &daily.puzzle.game_type,
        daily.puzzle.history.as_deref(),
        daily.puzzle.position.as_deref(),
    )
    .ok();
    let side = match start.as_ref().map(|start| start.turn_color) {
        Some(Color::Black) => "Black",
        _ => "White",
    };
    let thumbnail = start.map(|start| {
        let board = StoredValue::new(start.board);
        view! {
            <div class="overflow-hidden w-28 h-28 rounded-lg sm:w-36 sm:h-36 shrink-0 bg-black/5 dark:bg-white/5">
                <ThumbnailPieces board />
            </div>
        }
    });
    let tally = match (daily.tried, daily.solved) {
        (0, _) => "Be the first to solve it today".to_string(),
        (tried, solved) => format!("{solved} of {tried} solved it today"),
    };
    let streak = daily
        .streak
        .filter(|streak| streak.current > 0)
        .map(|streak| format!("🔥 {}-day streak", streak.current));
    let (action, action_class) = match daily.my_result {
        None => ("Solve it →", "text-white bg-pillbug-teal"),
        Some(true) => ("✓ Solved", "text-white bg-grasshopper-green"),
        Some(false) => ("Missed · try again", "text-white bg-orange-twilight"),
    };
    view! {
        <div class="flex flex-col gap-2 p-3 mx-auto w-full max-w-screen-md rounded-lg border shadow-sm sm:flex-row sm:items-center border-black/10 bg-even-light dark:border-white/10 dark:bg-surface-panel">
            <a
                href="/puzzles/daily"
                class="flex flex-1 gap-4 items-center min-w-0 no-link-style group"
            >
                {thumbnail}
                <span class="flex flex-col gap-1 min-w-0">
                    <span class="text-xs font-semibold tracking-wide uppercase text-pillbug-teal">
                        {format!("Puzzle of the day · {}", daily.day.format("%A"))}
                    </span>
                    <span class="text-lg font-bold group-hover:text-pillbug-teal">
                        {format!("{side} to win in {}", moves_label(daily.puzzle.solver_moves))}
                    </span>
                    <span class="text-sm text-gray-600 dark:text-gray-400">
                        {format!("Difficulty {} · {tally}", daily.puzzle.rating)}
                    </span>
                    {streak
                        .map(|streak| {
                            view! {
                                <span class="text-sm font-semibold text-orange-twilight">
                                    {streak}
                                </span>
                            }
                        })}
                    <span class=format!(
                        "self-start mt-1 px-4 py-2 text-sm font-semibold rounded-lg whitespace-nowrap shadow {action_class}",
                    )>{action}</span>
                </span>
            </a>
            <a
                href="/puzzles"
                class="inline-flex items-center self-end text-sm font-semibold whitespace-nowrap sm:self-center min-h-11 ui-text-link"
            >
                "More puzzles →"
            </a>
        </div>
    }
}

#[component]
fn PuzzlesCard() -> impl IntoView {
    view! {
        <a
            href="/puzzles"
            class="flex gap-4 justify-between items-center p-4 mx-auto w-full max-w-screen-md rounded-lg border shadow-sm no-link-style border-black/10 bg-even-light dark:border-white/10 dark:bg-surface-panel hover:border-pillbug-teal"
        >
            <span class="flex flex-col">
                <span class="text-lg font-bold">"Puzzles"</span>
                <span class="text-sm text-gray-600 dark:text-gray-400">"Find the forced win"</span>
            </span>
            <span class="text-white whitespace-nowrap shadow ui-button bg-pillbug-teal">
                "Solve a puzzle →"
            </span>
        </a>
    }
}
