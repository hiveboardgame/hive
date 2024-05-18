use crate::components::atoms::download_pgn::DownloadPgn;
use crate::providers::game_state::GameStateSignal;
use leptos::*;
use leptos_icons::*;

#[component]
pub fn AnalysisAndDownload() -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let analysis_setup = move |_| {
        let mut game_state = expect_context::<GameStateSignal>();
        game_state.do_analysis();
    };

    view! {
        <Show when=game_state.is_finished()>
            <div class="flex justify-center items-center mt-1">
                <a
                    href="/analysis"
                    class="justify-self-end place-self-center m-1 text-white rounded duration-300 bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal"
                    on:click=analysis_setup
                >
                    <Icon icon=icondata::TbMicroscope class="py-1 w-7 h-7"/>
                </a>
                <DownloadPgn/>
            </div>
        </Show>
    }
}
