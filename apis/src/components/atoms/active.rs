use crate::common::ActiveState;
use crate::common::SvgPos;
use crate::common::TileDesign;
use crate::providers::game_state::GameStateSignal;
use crate::providers::Config;
use hive_lib::Position;
use leptos::either::Either;
use leptos::prelude::*;

#[component]
pub fn Active(
    position: Position,
    #[prop(into)] level: Signal<usize>,
    #[prop(optional)] extend_tw_classes: &'static str,
    active_state: ActiveState,
) -> impl IntoView {
    match active_state {
        ActiveState::None => {
            Either::Left(view! { <ActiveWithOnClick position level extend_tw_classes /> })
        }
        ActiveState::Reserve | ActiveState::Board => {
            Either::Right(view! { <ActiveWithoutOnClick position level extend_tw_classes /> })
        }
    }
}

#[component]
pub fn ActiveWithoutOnClick(
    position: Position,
    #[prop(into)] level: Signal<usize>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let config = expect_context::<Config>().0;
    let straight = move || config().tile_design == TileDesign::ThreeD;
    let center = move || SvgPos::center_for_level(position, level(), straight());
    let transform = move || format!("translate({},{})", center().0, center().1);
    view! {
        <g class=format!("{extend_tw_classes}")>
            <g id="Active" transform=transform>
                <use_
                    href="/assets/tiles/common/all.svg#active"
                    transform="scale(0.56, 0.56) translate(-46.608, -52.083)"
                ></use_>
            </g>
        </g>
    }
}

#[component]
pub fn ActiveWithOnClick(
    position: Position,
    #[prop(into)] level: Signal<usize>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let config = expect_context::<Config>().0;
    let straight = move || config().tile_design == TileDesign::ThreeD;
    let center = move || SvgPos::center_for_level(position, level(), straight());
    let transform = move || format!("translate({},{})", center().0, center().1);
    let mut game_signal = expect_context::<GameStateSignal>();
    let onclick = move |_| {
        game_signal.reset();
        leptos::logging::log!("resetting did");
    };
    view! {
        <g class=format!("{extend_tw_classes}") on:click=onclick>
            <g id="Active" transform=transform>
                <use_
                    href="/assets/tiles/common/all.svg#active"
                    transform="scale(0.56, 0.56) translate(-46.608, -52.083)"
                ></use_>
            </g>
        </g>
    }
}
