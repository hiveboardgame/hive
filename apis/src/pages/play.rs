use crate::components::organisms::{board::Board, side_board::SideboardTabs};
use hive_lib::position::Position;
use leptos::*;
use leptos_router::*;

#[derive(Params, PartialEq, Eq)]
struct PlayParams {
    nanoid: String,
}

#[derive(Clone)]
pub struct TargetStack(pub RwSignal<Option<Position>>);

#[component]
pub fn Play(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    provide_context(TargetStack(RwSignal::new(None)));
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
        <div class=format!("grid grid-cols-10 grid-rows-6 h-[90%] w-[98%] {extend_tw_classes}")>
            <Board/>
            <SideboardTabs extend_tw_classes="border-blue-200"/>
        </div>
    }
}

