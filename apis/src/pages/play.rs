use crate::{
    common::time_control::TimeControl,
    components::organisms::{board::Board, side_board::SideboardTabs, timer::DisplayTimer},
};
use hive_lib::{color::Color, position::Position};
use leptos::*;
use leptos_router::*;
use std::time::Duration;

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
    // TODO: move the time_control to the gamestate
    let time_control = TimeControl::RealTime(Duration::from_secs(10), Duration::from_secs(3));
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
            <DisplayTimer side=Color::White time_control=time_control.clone()/>
            <SideboardTabs extend_tw_classes="border-blue-200"/>
            <DisplayTimer side=Color::Black time_control=time_control/>
        </div>
    }
}

