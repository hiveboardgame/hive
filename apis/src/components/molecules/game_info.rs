use leptos::*;
use shared_types::{TimeInfo, TournamentId};

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
    let tournaemnt_info = create_read_slice(game_state.signal, |gs| {
        gs.game_response.as_ref().map(|gr| {
            (
                gr.tournament.is_some(),
                gr.tournament.as_ref().map(|t| t.name.clone()),
                gr.tournament.as_ref().map(|t| t.tournament_id.clone()),
            )
        })
    });
    move || {
        if let (Some((time_info, rated)), Some((is_tournament, name, nanoid))) =
            (game_info(), tournaemnt_info())
        {
            let rated = format!("• {}", if rated { "Rated" } else { "Casual" });
            let name = store_value(name);
            let name = move || {
                if let Some(name) = name() {
                    format!("played in {}", name)
                } else {
                    String::new()
                }
            };
            let nanoid = store_value(nanoid);
            let link = move || {
                if let Some(TournamentId(id)) = nanoid() {
                    format!("/tournament/{}", id)
                } else {
                    String::new()
                }
            };
            view! {
                <div class=extend_tw_classes>
                    <div class="flex gap-1 items-center">
                        <TimeRow time_info extend_tw_classes="whitespace-nowrap"/>
                        {rated}
                        <Show when=move|| is_tournament>
                            <a
                                href=link
                            >
                                {name()}
                            </a>
                        </Show>
                    </div>
                </div>
            }
            .into_view()
        } else {
            view! { "" }.into_view()
        }
    }
}
