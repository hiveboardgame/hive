use crate::components::atoms::download_pgn::DownloadPgn;
use crate::providers::game_state::GameStateSignal;
use leptos::prelude::*;
use leptos_icons::*;
use leptos_router::hooks::{use_params_map, use_query_map};
use shared_types::GameSpeed;

#[component]
pub fn AnalysisAndDownload() -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let params = use_params_map();
    let queries = use_query_map();
    let correspondence = create_read_slice(game_state.signal, |gs| {
        gs.game_response.as_ref().is_some_and(|gr| {
            gr.speed == GameSpeed::Correspondence || gr.speed == GameSpeed::Untimed
        })
    });
    let is_finished = game_state.is_finished();

    let analysis_url = move || {
        if let Some(nanoid) = params.get().get("nanoid") {
            let mut url = format!("/analysis/{nanoid}");

            if let Some(move_param) = queries.get().get("move") {
                url = format!("{url}?move={move_param}");
            }

            url
        } else {
            "/analysis".to_string()
        }
    };

    view! {
        <Show when=move || is_finished() || correspondence()>
            <div class="flex justify-center items-center place-self-start">
                <a
                    href=analysis_url
                    class="justify-self-end place-self-center m-1 text-white rounded duration-300 no-link-style bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal"
                >
                    <Icon icon=icondata::TbMicroscope attr:class="py-1 w-7 h-7" />
                </a>
                <DownloadPgn />
            </div>
        </Show>
    }
}
