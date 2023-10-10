use crate::components::organisms::{board::Board, side_board::SideboardTabs};
use leptos::*;
use leptos_router::*;

#[derive(Params, PartialEq, Eq)]
struct PlayParams {
    nanoid: String,
}

#[component]
pub fn Play(#[prop(default = "")] extend_tw_classes: &'static str) -> impl IntoView {
    let params = use_params::<PlayParams>();

    // id: || -> usize
    let _nanoid = move || {
        params.with(|params| {
            params
                .as_ref()
                .map(|params| params.nanoid.clone())
                .unwrap_or_default()
        })
    };

    view! {
        <div class=format!("grid grid-cols-10 grid-rows-6 h-full w-full {extend_tw_classes}")>
            <Board/>
            <SideboardTabs extend_tw_classes="border-blue-200"/>
        </div>
    }
}
