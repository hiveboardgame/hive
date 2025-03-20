use crate::components::atoms::download_pgn::DownloadPgn;
use crate::providers::game_state::GameStateSignal;
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::GameSpeed;

#[component]
pub fn AnalysisAndDownload() -> impl IntoView {
    let mut game_state = expect_context::<GameStateSignal>();
    let analysis_setup = move |_| {
        game_state.do_analysis();
    };
    let correspondence = create_read_slice(game_state.signal, |gs| {
        gs.game_response.as_ref().is_some_and(|gr| {
            gr.speed == GameSpeed::Correspondence || gr.speed == GameSpeed::Untimed
        })
    });
    let is_finished = game_state.is_finished();

    view! {
        <Show when=move || is_finished() || correspondence()>
            <div class="flex justify-center items-center place-self-start">
                <a
                    href="/analysis"
                    class="justify-self-end place-self-center m-1 text-white rounded duration-300 bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal"
                    on:click=analysis_setup
                >
                    <Icon icon=icondata::TbMicroscope attr:class="py-1 w-7 h-7" />
                </a>
                <DownloadPgn />
            </div>
        </Show>
    }
}
