use std::str::FromStr;

use leptos::*;
use shared_types::time_mode::TimeMode;

use crate::{components::molecules::time_row::TimeRow, providers::game_state::GameStateSignal};

#[component]
pub fn GameInfo(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    move || {
        if let Some(gr) = game_state.signal.get().game_response {
            let rated = format!("• {}", if gr.rated { "Rated" } else { "Casual" });
            view! {
                <div class=extend_tw_classes>
                    <div class="flex items-center gap-1">
                        <TimeRow
                            time_mode=TimeMode::from_str(&gr.time_mode).expect("Valid time mode")
                            time_base=gr.time_base
                            increment=gr.time_increment
                            extend_tw_classes="whitespace-nowrap"
                        />
                        {rated}
                    </div>
                </div>
            }
            .into_view()
        } else {
            view! {}.into_view()
        }
    }
}
