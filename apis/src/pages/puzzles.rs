use crate::components::organisms::puzzle_trainer::PuzzleTrainer;
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
