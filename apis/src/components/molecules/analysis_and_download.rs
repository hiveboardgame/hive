use crate::{
    components::atoms::download_pgn::DownloadPgn,
    providers::{
        game_eval::evals_visible,
        game_state::{GameStateStore, GameStateStoreFields},
    },
};
use leptos::prelude::*;
use leptos_icons::*;
use leptos_router::hooks::{use_params_map, use_query_map};
use shared_types::{eval_unavailable_reason, GameSpeed};

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
    let evals_visible = evals_visible();
    // The Evals tab is otherwise three steps away from a finished game, behind an icon.
    let evals_url = Memo::new(move |_| {
        if !evals_visible.get() {
            return None;
        }
        game_response.with(|game| {
            let game = game.as_ref()?;
            eval_unavailable_reason(
                game.finished,
                &game.game_type.to_string(),
                game.tournament_queen_rule,
                game.history.len(),
            )
            .is_none()
            .then(|| format!("/analysis/{}?tab=evals", game.game_id))
        })
    });

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
                    title="Analyse this game"
                    aria-label="Analyse this game"
                >
                    <Icon icon=icondata_tb::TbMicroscopeOutline attr:class="py-1 size-7" />
                </a>
                {move || {
                    evals_url
                        .get()
                        .map(|href| {
                            view! {
                                <a
                                    href=href
                                    class="gap-1 px-2 m-1 h-9 rounded-sm ui-button ui-button-primary no-link-style"
                                    title="Computer eval: the engine's verdict on every move"
                                >
                                    <Icon icon=icondata_tb::TbCpuOutline attr:class="size-5" />
                                    "Eval"
                                </a>
                            }
                        })
                }}
                <DownloadPgn />
            </div>
        </Show>
    }
}
