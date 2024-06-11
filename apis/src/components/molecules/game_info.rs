use leptos::*;
use shared_types::TimeMode;

use crate::{components::molecules::time_row::TimeRow, providers::game_state::GameStateSignal};

#[derive(Clone, PartialEq)]
struct TimeInfo {
    time_mode: TimeMode,
    time_base: Option<i32>,
    time_increment: Option<i32>,
    rated: bool,
}

#[component]
pub fn GameInfo(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let game_info = create_read_slice(game_state.signal, |gs| {
        gs.game_response.as_ref().map(|gr| TimeInfo {
            time_mode: gr.time_mode.clone(),
            time_base: gr.time_base,
            time_increment: gr.time_increment,
            rated: gr.rated,
        })
    });
    move || {
        if let Some(gi) = game_info() {
            let rated = format!("• {}", if gi.rated { "Rated" } else { "Casual" });
            view! {
                <div class=extend_tw_classes>
                    <div class="flex gap-1 items-center">
                        <TimeRow
                            time_mode=gi.time_mode
                            time_base=gi.time_base
                            increment=gi.time_increment
                            extend_tw_classes="whitespace-nowrap"
                        />
                        {rated}
                    </div>
                </div>
            }
            .into_view()
        } else {
            view! { "" }.into_view()
        }
    }
}
