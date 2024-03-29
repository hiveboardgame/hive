use crate::components::atoms::download_pgn::DownloadPgn;
use crate::providers::game_state::GameStateSignal;
use leptos::*;
use leptos_icons::*;

#[component]
pub fn AnalysisAndDownload() -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let is_finished = move || (game_state.signal)().is_finished();

    let analysis_setup = move |_| {
        let mut game_state = expect_context::<GameStateSignal>();
        game_state.do_analysis();
    };

    view! {
        <Show when=is_finished>
            <div class="flex items-center justify-center mt-1">
                <a
                    href="/analysis"
                    class="bg-ant-blue hover:bg-pillbug-teal duration-300 text-white rounded m-1 place-self-center justify-self-end"
                    on:click=analysis_setup
                >
                    <Icon icon=icondata::TbMicroscope class="h-7 w-7 py-1"/>
                </a>
                <DownloadPgn/>
            </div>
        </Show>
    }
}
