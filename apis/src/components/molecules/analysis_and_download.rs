use crate::{
    components::atoms::download_pgn::DownloadPgn,
    hooks::analysis_href::use_analysis_href,
    providers::game_state::{GameStateStore, GameStateStoreFields},
};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::GameSpeed;

#[component]
pub fn AnalysisAndDownload() -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let game_response = game_state.game_response();
    let correspondence = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response.as_ref().is_some_and(|gr| {
                gr.speed == GameSpeed::Correspondence || gr.speed == GameSpeed::Untimed
            })
        })
    });
    let is_finished = game_state.is_finished();

    let analysis_href = use_analysis_href();

    view! {
        <Show when=move || is_finished() || correspondence()>
            <div class="flex justify-center items-center shrink-0">
                <a
                    href=analysis_href
                    class="m-1 rounded-sm ui-button ui-button-primary ui-button-icon no-link-style"
                >
                    <Icon icon=icondata_tb::TbMicroscopeOutline attr:class="py-1 size-7" />
                </a>
                <DownloadPgn />
            </div>
        </Show>
    }
}
