use crate::{
    common::{ActiveState, SvgPos},
    providers::game_state::GameStateStore,
};
use hive_lib::Position;
use leptos::{either::Either, prelude::*, text_prop::TextProp};

#[component]
pub fn Active(
    position: Position,
    level: usize,
    #[prop(optional)] extend_tw_classes: &'static str,
    active_state: ActiveState,
    straight: bool,
) -> impl IntoView {
    let game_signal = expect_context::<GameStateStore>();
    let center = move || SvgPos::center_for_level(position, level, straight);
    let transform = TextProp::from(move || format!("translate({},{})", center().0, center().1));
    match active_state {
        ActiveState::None | ActiveState::Board => Either::Left(view! {
            <g
                class=format!("{extend_tw_classes}")
                on:click=move |_| {
                    game_signal.reset();
                }
            >
                <Inner transform />
            </g>
        }),
        ActiveState::Reserve => Either::Right(view! {
            <g class=format!("{extend_tw_classes}")>
                <Inner transform />
            </g>
        }),
    }
}

#[component]
fn Inner(transform: TextProp) -> impl IntoView {
    let href = || "/assets/tiles/common/all.svg#active";

    view! {
        <g id="Active" transform=transform>
            <use_ href=href transform="scale(0.56, 0.56) translate(-46.608, -52.083)"></use_>
        </g>
    }
}
