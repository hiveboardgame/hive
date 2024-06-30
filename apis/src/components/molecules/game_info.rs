use leptos::*;
use shared_types::TimeInfo;

use crate::{components::molecules::time_row::TimeRow, providers::game_state::GameStateSignal};

#[component]
pub fn GameInfo(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let game_info = create_read_slice(game_state.signal, |gs| {
        gs.game_response.as_ref().map(|gr| {
            (
                TimeInfo {
                    mode: gr.time_mode.clone(),
                    base: gr.time_base,
                    increment: gr.time_increment,
                },
                gr.rated,
            )
        })
    });
    move || {
        if let Some((time_info, rated)) = game_info() {
            let rated = format!("• {}", if rated { "Rated" } else { "Casual" });
            view! {
                <div class=extend_tw_classes>
                    <div class="flex gap-1 items-center">
                        <TimeRow time_info extend_tw_classes="whitespace-nowrap"/>
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
