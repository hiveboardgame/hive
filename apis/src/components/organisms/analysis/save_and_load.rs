use crate::providers::{
    analysis::{AnalysisSignal, AnalysisTree},
    game_state::GameStateStore,
};
use hive_lib::History;
use leptos::{
    html,
    logging,
    prelude::*,
    reactive::effect::batch,
    task::spawn_local_scoped_with_cancellation,
};
use std::path::Path;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{js_sys::Array, Blob, Url};

const BUTTON_CLASS: &str = "ui-button ui-button-primary ui-button-sm h-9 flex-1 px-3 text-xs";

#[component]
pub fn DownloadTree() -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>().tree;

    let download = move |_| {
        let tree_json = analysis.with_untracked(|a| {
            let out = AnalysisTree {
                current_node: a.current_node.clone(),
                tree: a.tree.clone(),
                hashes: a.hashes.clone(),
                game_type: a.game_type,
                annotations: a.annotations.clone(),
            };
            serde_json::to_string(&out).unwrap()
        });

        let (blob, filename) = blob_and_filename(tree_json);
        let url = Url::create_object_url_with_blob(&blob).unwrap();
        let a = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("a")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .expect("This element is not an HtmlElement");
        a.set_attribute("href", &url).unwrap();
        a.set_attribute("download", &filename).unwrap();
        a.click();
        let _ = Url::revoke_object_url(&url);
    };

    view! {
        <button on:click=download class=BUTTON_CLASS>
            "Save"
        </button>
    }
}

fn blob_and_filename(tree: String) -> (Blob, String) {
    let file = Array::from(&JsValue::from(tree));
    let date = chrono::offset::Local::now()
        .format("%d-%b-%Y_%H:%M:%S")
        .to_string();
    (
        Blob::new_with_u8_array_sequence(&file).unwrap(),
        format!("analysis_{date}.json"),
    )
}

#[component]
pub fn LoadTree() -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>();
    let game_state = expect_context::<GameStateStore>();
    let input_ref = NodeRef::<html::Input>::new();
    let load_owner = Owner::current().expect("LoadTree must run inside a reactive owner");

    let from_pgn = move |string: JsValue| {
        string
            .as_string()
            .and_then(|string| History::from_pgn_str(string).ok())
            .and_then(|history| hive_lib::State::new_from_history(&history).ok())
            .map(|state| {
                let tree = AnalysisTree::from_loaded_state(&state);
                batch(|| {
                    game_state.reset_with_state(state.clone());
                    analysis.tree.set(tree);
                });
                analysis.sync_reserve.run(state.turn_color);
            })
    };
    let from_json = move |string: JsValue| {
        string
            .as_string()
            .and_then(|string| serde_json::from_str::<AnalysisTree>(&string).ok())
            .map(|mut tree| {
                tree.ensure_start_node();
                let current_node_id = tree.current_node_id();
                batch(|| {
                    game_state.full_reset();
                    if let Some(node_id) = current_node_id {
                        tree.update_node(node_id, Some(game_state));
                    }
                    analysis.tree.set(tree);
                });
                if current_node_id.is_some() {
                    analysis.sync_reserve_from_game_state(game_state);
                }
            })
    };
    let oninput = move |_| {
        let file = input_ref
            .get_untracked()
            .unwrap()
            .files()
            .unwrap()
            .get(0)
            .unwrap();
        Path::new(&file.name()).extension().map_or_else(
            || logging::log!("Couldn't open file"),
            |ext| {
                let ext = ext.to_os_string();
                load_owner.with(|| {
                    spawn_local_scoped_with_cancellation(async move {
                        let text = JsFuture::from(file.text()).await.ok();
                        let result = if ext == "json" {
                            text.and_then(from_json)
                        } else if ext == "pgn" {
                            text.and_then(from_pgn)
                        } else {
                            logging::log!("Unsupported file type");
                            None
                        };
                        if result.is_none() {
                            logging::log!("Couldn't open file");
                        }
                    });
                });
            },
        );
    };
    view! {
        <label for="load-analysis" class=format!("{BUTTON_CLASS} cursor-pointer")>
            "Load"
        </label>
        <input
            node_ref=input_ref
            on:input=oninput
            type="file"
            id="load-analysis"
            accept=".json,.pgn"
            hidden
        />
    }
}
