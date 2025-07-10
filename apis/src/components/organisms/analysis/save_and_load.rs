use crate::providers::{
    analysis::{AnalysisSignal, AnalysisTree},
    game_state::GameStateSignal,
};
use hive_lib::History;
use leptos::{html, logging, prelude::*};
use std::path::Path;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{js_sys::Array, Blob, Url};

const BTN_CLASS: &str = "z-20 content-center text-center m-1 w-1/3 h-7 text-white rounded-sm transition-transform duration-300 transform aspect-square bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95";

#[component]
pub fn DownloadTree(tree: String) -> impl IntoView {
    let download = move |_| {
        let (blob, filename) = blob_and_filename(tree.clone());
        // Create an object URL for the blob
        let url = Url::create_object_url_with_blob(&blob).unwrap();
        // Create a download link
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
        <button on:click=download class=BTN_CLASS>
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
    let analysis = expect_context::<AnalysisSignal>().0;
    let game_state = expect_context::<GameStateSignal>();
    let input_ref = NodeRef::<html::Input>::new();

    let from_pgn = move |string: JsValue| {
        string
            .as_string()
            .and_then(|string| History::from_pgn_str(string).ok())
            .and_then(|history| hive_lib::State::new_from_history(&history).ok())
            .map(|state| {
                game_state.signal.update(|gs| gs.state = state.clone());
                let tree = AnalysisTree::from_loaded_state(game_state, &state);
                analysis.set(LocalStorage::wrap(tree));
            })
    };
    let from_json = move |string: JsValue| {
        string
            .as_string()
            .and_then(|string| serde_json::from_str::<AnalysisTree>(&string).ok())
            .map(|tree| {
                analysis.set(LocalStorage::wrap(tree.clone()));
                if let Some(node) = tree.current_node {
                    analysis
                        .get()
                        .update_node(node.get_node_id(), Some(game_state));
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
                spawn_local(async move {
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
            },
        );
    };
    view! {
        <label for="load-analysis" class=BTN_CLASS>
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
