use crate::{
    components::atoms::download_pgn::DownloadPgn,
    providers::game_state::{GameStateStore, GameStateStoreFields},
};
use leptos::prelude::*;
use leptos_icons::*;
use leptos_router::hooks::{use_params_map, use_query_map};
use shared_types::GameSpeed;

#[component]
pub fn AnalysisAndDownload() -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let params = use_params_map();
    let queries = use_query_map();
    let game_response = game_state.game_response();
    let correspondence = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response.as_ref().is_some_and(|gr| {
                gr.speed == GameSpeed::Correspondence || gr.speed == GameSpeed::Untimed
            })
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
            <div class="flex justify-center items-center shrink-0">
                <a
                    href=analysis_url
                    class="m-1 rounded-sm ui-button ui-button-primary ui-button-icon no-link-style"
                >
                    <Icon icon=icondata_tb::TbMicroscopeOutline attr:class="py-1 size-7" />
                </a>
                <DownloadPgn />
            </div>
        </Show>
    }
}
