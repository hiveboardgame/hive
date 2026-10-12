use crate::{
    components::{
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::puzzle_nav::PuzzleNav,
        organisms::puzzle_trainer::PuzzleTrainer,
    },
    functions::puzzles::daily_puzzle,
    providers::puzzle::DailyStatus,
};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[component]
pub fn Puzzles() -> impl IntoView {
    view! { <PuzzleTrainer nanoid=Signal::derive(|| None::<String>) /> }
}

#[component]
pub fn PuzzleView() -> impl IntoView {
    let params = use_params_map();
    let nanoid = Signal::derive(move || params.with(|params| params.get("nanoid")));
    view! { <PuzzleTrainer nanoid /> }
}

#[component]
pub fn DailyPuzzleView() -> impl IntoView {
    let daily = Resource::new(|| (), |_| daily_puzzle());
    view! {
        <Suspense>
            {move || Suspend::new(async move {
                match daily.await {
                    Ok(Some(daily)) => {
                        let nanoid = daily.puzzle.nanoid.clone();
                        let status = DailyStatus {
                            tried: daily.tried,
                            solved: daily.solved,
                            my_result: daily.my_result,
                        };
                        view! {
                            <PuzzleTrainer
                                nanoid=Signal::derive(move || Some(nanoid.clone()))
                                daily=status
                            />
                        }
                            .into_any()
                    }
                    _ => {
                        view! {
                            <PageShell>
                                <PageHeader title="Puzzle of the day" />
                                <PuzzleNav />
                                <p class="text-sm">"There is no puzzle of the day yet."</p>
                            </PageShell>
                        }
                            .into_any()
                    }
                }
            })}
        </Suspense>
    }
}
