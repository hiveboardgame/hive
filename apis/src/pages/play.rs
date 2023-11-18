use crate::{
    common::time_control::TimeControl,
    components::organisms::{board::Board, side_board::SideboardTabs, timer::DisplayTimer},
    functions::games::get::get_game_from_nanoid,
    providers::game_state::GameStateSignal,
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
    let mut game_state = expect_context::<GameStateSignal>();

    let params = use_params::<PlayParams>();
    // TODO: move the time_control to the gamestate
    let time_control = store_value(TimeControl::RealTime(
        Duration::from_secs(10),
        Duration::from_secs(3),
    ));
    let nanoid = move || {
        params.with(|params| {
            params
                .as_ref()
                .map(|params| params.nanoid.clone())
                .unwrap_or_default()
        })
    };
    let game = Resource::new(nanoid, move |_| get_game_from_nanoid(nanoid()));

    // TODO: @ion move set_game_id and join into the transition
    game_state.set_game_id(nanoid());
    #[cfg(feature = "hydrate")]
    game_state.join();

    view! {
        <Transition>
            {move || {
                game()
                    .map(|data| match data {
                        Err(_) => view! { <pre>"Page not found"</pre> }.into_view(),
                        Ok(_) => {
                            view! {
                                <div class=format!(
                                    "grid grid-cols-10 grid-rows-6 max-h-[95svh] {extend_tw_classes}",
                                )>
                                    <Board/>
                                    <DisplayTimer side=Color::White time_control=time_control()/>
                                    <SideboardTabs extend_tw_classes="border-blue-200"/>
                                    <DisplayTimer side=Color::Black time_control=time_control()/>
                                </div>
                            }
                                .into_view()
                        }
                    })
            }}

        </Transition>
    }
}
